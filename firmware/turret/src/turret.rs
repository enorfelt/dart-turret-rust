use std::sync::mpsc::Receiver;
use std::thread;
use std::time::{Duration, Instant};

use esp_idf_svc::hal::gpio::{Input as GpioInput, PinDriver};
use turret_core::arm_switch::ArmSwitch;
use turret_core::battery::Health;
use turret_core::command::Command;
use turret_core::config::TurretConfig;
use turret_core::fire::{Event, FireControl, Phase, Rejection};
use turret_core::remote::{RemoteEvent, StateSnapshot};

use crate::actuators::Actuators;
use crate::battery::{BatteryMonitor, Reading};
use crate::ble::Remote;
use crate::gimbal::Gimbal;
use crate::light_gate::LightGate;

const LOOP_PERIOD: Duration = Duration::from_millis(5);
const BATTERY_PERIOD_MS: u64 = 5_000;

const HELP: &str = "\
commands:
  arm | disarm            enable / disable the flywheels and pusher (arm needs the arm switch on
                          and a battery that is not empty or faulty)
  status                  show arming, arm switch, fire phase, battery and gimbal
  ping <id>               ping a serial bus servo
  pusher <us> | off       hold the pusher at a pulse (500-2500 us) for calibration, disarmed only
  P;pan;tilt;vpan;vtilt   move the gimbal in servo units
  F;1;mode;throttle;burst fire: mode 0 single, 1 burst, 2 auto; throttle 0-100 %
  F;0                     cease fire
  B                       battery level
  unpair                  forget all paired phones";

pub enum Input {
    Command(Command),
    RemoteLost,
}

pub struct Turret {
    pub actuators: Actuators,
    pub gimbal: Gimbal,
    pub light_gate: LightGate,
    pub arm_switch: PinDriver<'static, GpioInput>,
    pub battery: BatteryMonitor,
    pub remote: Remote,
    pub config: TurretConfig,
}

impl Turret {
    pub fn run(self, inputs: Receiver<Input>) -> ! {
        let Self {
            mut actuators,
            gimbal,
            light_gate,
            arm_switch: arm_switch_pin,
            mut battery,
            remote,
            config,
        } = self;
        let boot = Instant::now();
        let mut fire = FireControl::new(config.fire);
        let mut arm_switch = ArmSwitch::new(config.arm_switch_settle_ms);
        let mut state = State::default();
        let mut sent_state = None;

        log::info!("turret ready, disarmed; type help");
        loop {
            let now_ms = boot.elapsed().as_millis() as u64;

            arm_switch.update(arm_switch_pin.is_low(), now_ms);
            let arm_switch_on = arm_switch.is_on(now_ms);
            if fire.is_armed() && !arm_switch_on {
                if let Some(event) = fire.disarm(now_ms) {
                    report(event, &remote);
                }
                log::warn!("arm switch off, disarmed");
            }

            while let Ok(input) = inputs.try_recv() {
                match input {
                    Input::Command(command) => {
                        handle(
                            command,
                            now_ms,
                            arm_switch_on,
                            &mut fire,
                            &gimbal,
                            &mut state,
                            &remote,
                        );
                    }
                    Input::RemoteLost => {
                        state.held_pusher_us = None;
                        if let Some(event) = fire.disarm(now_ms) {
                            report(event, &remote);
                        }
                        log::warn!("remote lost, disarmed");
                    }
                }
            }

            let phase_before = fire.phase();
            if let Some(blocked_us) = light_gate.take_blocked_us() {
                fire.record_gate(blocked_us);
            }
            if let Some(event) = fire.tick(now_ms) {
                report(event, &remote);
            }
            if fire.phase() == Phase::Pushing && phase_before != Phase::Pushing {
                light_gate.reset();
            }
            if let Err(error) = actuators.apply(&fire.outputs(now_ms), state.held_pusher_us) {
                log::error!("actuator update failed: {error}");
            }

            if now_ms >= state.next_battery_ms && fire.phase() == Phase::Idle {
                state.next_battery_ms = now_ms + BATTERY_PERIOD_MS;
                match battery.read() {
                    Ok(reading) => {
                        if state.battery.map(|previous| previous.health) != Some(reading.health) {
                            log_health(reading);
                        }
                        state.battery = Some(reading);
                        if fire.is_armed() && !reading.health.allows_arming() {
                            if let Some(event) = fire.disarm(now_ms) {
                                report(event, &remote);
                            }
                            log::warn!("battery {}, disarmed", describe_health(reading.health));
                        }
                    }
                    Err(error) => log::warn!("battery read failed: {error}"),
                }
            }

            let snapshot = state.snapshot(&fire, arm_switch_on, now_ms);
            if sent_state != Some(snapshot) {
                remote.state(snapshot);
                sent_state = Some(snapshot);
            }

            thread::sleep(LOOP_PERIOD);
        }
    }
}

#[derive(Default)]
struct State {
    battery: Option<Reading>,
    gimbal: Option<(u16, u16)>,
    held_pusher_us: Option<u16>,
    next_battery_ms: u64,
}

impl State {
    fn snapshot(&self, fire: &FireControl, arm_switch: bool, now_ms: u64) -> StateSnapshot {
        StateSnapshot {
            armed: fire.is_armed(),
            ready: fire.is_ready(now_ms),
            arm_switch,
            phase: fire.phase(),
            battery_mv: self
                .battery
                .map(|reading| u16::try_from(reading.pack_mv).unwrap_or(u16::MAX)),
            battery_pct: self.battery.map_or(0, |reading| reading.percent),
            gimbal: self.gimbal,
            held_pusher_us: self.held_pusher_us,
        }
    }
}

fn handle(
    command: Command,
    now_ms: u64,
    arm_switch_on: bool,
    fire: &mut FireControl,
    gimbal: &Gimbal,
    state: &mut State,
    remote: &Remote,
) {
    match command {
        Command::Arm if !arm_switch_on => println!("not armed: the arm switch is off"),
        Command::Arm => match state
            .battery
            .map(|reading| reading.health)
            .filter(|health| !health.allows_arming())
        {
            Some(health) => println!("not armed: battery {}", describe_health(health)),
            None => {
                state.held_pusher_us = None;
                fire.arm();
                println!("armed");
            }
        },
        Command::Disarm => {
            if let Some(event) = fire.disarm(now_ms) {
                report(event, remote);
            }
            println!("disarmed");
        }
        Command::Fire(request) => match fire.fire(&request, now_ms) {
            Ok(()) => println!("firing {request:?}"),
            Err(rejection) => {
                println!("fire rejected: {}", describe(rejection));
                remote.event(RemoteEvent::FireRejected(rejection));
            }
        },
        Command::CeaseFire => {
            fire.cease_fire(now_ms);
            println!("cease fire");
        }
        Command::Move {
            pan,
            tilt,
            pan_speed,
            tilt_speed,
        } => match gimbal.move_to(pan, tilt, pan_speed, tilt_speed) {
            Ok(position) => {
                state.gimbal = Some(position);
                println!("moving to pan {} tilt {}", position.0, position.1);
            }
            Err(error) => {
                println!("move failed: {error}");
                remote.event(RemoteEvent::MoveFailed);
            }
        },
        Command::Ping(id) => match gimbal.ping(id) {
            Ok(status) => println!(
                "servo {} replied, error flags {:#04x}",
                status.id, status.error
            ),
            Err(error) => println!("servo {id}: {error}"),
        },
        Command::Pusher(_) if fire.is_armed() => println!("pusher test refused: disarm first"),
        Command::Pusher(Some(pulse_us)) => {
            state.held_pusher_us = Some(pulse_us);
            if arm_switch_on {
                println!("pusher held at {pulse_us} us");
            } else {
                println!("pusher held at {pulse_us} us, but the arm switch is off so the servo has no power");
            }
        }
        Command::Pusher(None) => {
            state.held_pusher_us = None;
            println!("pusher released to retracted");
        }
        Command::Battery => match state.battery {
            Some(reading) => println!(
                "battery {} mV, {}%, {}",
                reading.pack_mv,
                reading.percent,
                describe_health(reading.health)
            ),
            None => println!("battery not read yet"),
        },
        Command::Status => {
            println!(
                "{}, arm switch {}, fire phase {:?}, pusher {}, battery {}, gimbal {}",
                if fire.is_armed() { "armed" } else { "disarmed" },
                if arm_switch_on { "on" } else { "off" },
                fire.phase(),
                state
                    .held_pusher_us
                    .map_or("normal".into(), |pulse_us| format!("held at {pulse_us} us")),
                state.battery.map_or("unknown".into(), |reading| format!(
                    "{} mV {}%",
                    reading.pack_mv, reading.percent
                )),
                state
                    .gimbal
                    .map_or("not moved yet".into(), |(pan, tilt)| format!(
                        "pan {pan} tilt {tilt}"
                    )),
            );
        }
        Command::Unpair => {
            remote.forget_bonds();
            println!("forgetting paired phones");
        }
        Command::Help => println!("{HELP}"),
    }
}

fn report(event: Event, remote: &Remote) {
    remote.event(event.into());
    match event {
        Event::Shot {
            number,
            speed_kmh: Some(speed),
        } => log::info!("shot {number}: {speed} km/h"),
        Event::Shot {
            number,
            speed_kmh: None,
        } => log::warn!("shot {number}: no light gate reading"),
        Event::Finished { shots } => log::info!("sequence finished after {shots} shot(s)"),
        Event::Aborted => log::warn!("fire sequence aborted"),
    }
}

fn log_health(reading: Reading) {
    match reading.health {
        Health::Good => log::info!("battery {} mV, good", reading.pack_mv),
        Health::Absent => log::info!("battery {} mV, no pack (USB power)", reading.pack_mv),
        health => log::warn!(
            "battery {} mV, {}",
            reading.pack_mv,
            describe_health(health)
        ),
    }
}

fn describe_health(health: Health) -> &'static str {
    match health {
        Health::Absent => "absent",
        Health::Fault => "fault: check for a reversed or dead cell",
        Health::Empty => "empty, charge the cells",
        Health::Low => "low",
        Health::Good => "good",
    }
}

fn describe(rejection: Rejection) -> &'static str {
    match rejection {
        Rejection::Disarmed => "disarmed, send arm first",
        Rejection::Busy => "already firing",
        Rejection::CoolingDown => "cooling down",
        Rejection::NoThrottle => "throttle must be above 0",
    }
}
