use std::sync::mpsc::Receiver;
use std::thread;
use std::time::{Duration, Instant};

use turret_core::command::Command;
use turret_core::config::TurretConfig;
use turret_core::fire::{Event, FireControl, Phase, Rejection};

use crate::actuators::Actuators;
use crate::battery::{BatteryMonitor, Reading};
use crate::gimbal::Gimbal;
use crate::light_gate::LightGate;

const LOOP_PERIOD: Duration = Duration::from_millis(5);
const BATTERY_PERIOD_MS: u64 = 5_000;

const HELP: &str = "\
commands:
  arm | disarm            enable / disable the flywheels and pusher
  status                  show arming, fire phase, battery and gimbal
  ping <id>               ping a serial bus servo
  P;pan;tilt;vpan;vtilt   move the gimbal (DartDroid format)
  F;1;mode;speed;burst    fire: mode 0 single, 1 burst, 2 auto; speed 0-180
  F;0                     cease fire
  B                       battery level";

pub struct Turret {
    pub actuators: Actuators,
    pub gimbal: Gimbal,
    pub light_gate: LightGate,
    pub battery: BatteryMonitor,
    pub config: TurretConfig,
}

impl Turret {
    pub fn run(self, commands: Receiver<Command>) -> ! {
        let Self {
            mut actuators,
            gimbal,
            light_gate,
            mut battery,
            config,
        } = self;
        let boot = Instant::now();
        let mut fire = FireControl::new(config.fire);
        let mut state = State::default();

        log::info!("turret ready, disarmed; type help");
        loop {
            let now_ms = boot.elapsed().as_millis() as u64;

            while let Ok(command) = commands.try_recv() {
                handle(command, now_ms, &mut fire, &gimbal, &mut state);
            }

            let phase_before = fire.phase();
            if let Some(blocked_us) = light_gate.take_blocked_us() {
                fire.record_gate(blocked_us);
            }
            if let Some(event) = fire.tick(now_ms) {
                report(event);
            }
            if fire.phase() == Phase::Pushing && phase_before != Phase::Pushing {
                light_gate.reset();
            }
            if let Err(error) = actuators.apply(&fire.outputs(now_ms)) {
                log::error!("actuator update failed: {error}");
            }

            if now_ms >= state.next_battery_ms {
                state.next_battery_ms = now_ms + BATTERY_PERIOD_MS;
                match battery.read() {
                    Ok(reading) => state.battery = Some(reading),
                    Err(error) => log::warn!("battery read failed: {error}"),
                }
            }

            thread::sleep(LOOP_PERIOD);
        }
    }
}

#[derive(Default)]
struct State {
    battery: Option<Reading>,
    gimbal: Option<(u16, u16)>,
    next_battery_ms: u64,
}

fn handle(
    command: Command,
    now_ms: u64,
    fire: &mut FireControl,
    gimbal: &Gimbal,
    state: &mut State,
) {
    match command {
        Command::Arm => {
            fire.arm();
            println!("armed");
        }
        Command::Disarm => {
            if let Some(event) = fire.disarm(now_ms) {
                report(event);
            }
            println!("disarmed");
        }
        Command::Fire(request) => match fire.fire(&request, now_ms) {
            Ok(()) => println!("firing {request:?}"),
            Err(rejection) => println!("fire rejected: {}", describe(rejection)),
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
            Err(error) => println!("move failed: {error}"),
        },
        Command::Ping(id) => match gimbal.ping(id) {
            Ok(status) => println!(
                "servo {} replied, error flags {:#04x}",
                status.id, status.error
            ),
            Err(error) => println!("servo {id}: {error}"),
        },
        Command::Battery => match state.battery {
            Some(reading) => println!("battery {} mV, {}%", reading.pack_mv, reading.percent),
            None => println!("battery not read yet"),
        },
        Command::Status => {
            println!(
                "{}, fire phase {:?}, battery {}, gimbal {}",
                if fire.is_armed() { "armed" } else { "disarmed" },
                fire.phase(),
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
        Command::Setup => println!("setup is not supported yet"),
        Command::Help => println!("{HELP}"),
    }
}

fn report(event: Event) {
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

fn describe(rejection: Rejection) -> &'static str {
    match rejection {
        Rejection::Disarmed => "disarmed, send arm first",
        Rejection::Busy => "already firing",
        Rejection::CoolingDown => "cooling down",
        Rejection::NoThrottle => "speed must be above 0",
    }
}
