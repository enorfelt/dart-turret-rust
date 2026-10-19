use esp_idf_svc::hal::gpio::AnyOutputPin;
use esp_idf_svc::hal::ledc::config::TimerConfig;
use esp_idf_svc::hal::ledc::{LedcDriver, LedcTimerDriver, LowSpeed, Resolution, LEDC};
use esp_idf_svc::hal::units::Hertz;
use esp_idf_svc::sys::EspError;
use turret_core::config::TurretConfig;
use turret_core::fire::{Outputs, Pusher};
use turret_core::pulse::duty_for_pulse;

pub struct ActuatorPins {
    pub esc_left: AnyOutputPin<'static>,
    pub esc_right: AnyOutputPin<'static>,
    pub pusher: AnyOutputPin<'static>,
}

pub struct Actuators {
    _timer: LedcTimerDriver<'static, LowSpeed>,
    esc_left: LedcDriver<'static>,
    esc_right: LedcDriver<'static>,
    pusher: LedcDriver<'static>,
    config: TurretConfig,
}

impl Actuators {
    pub fn new(ledc: LEDC, pins: ActuatorPins, config: TurretConfig) -> Result<Self, EspError> {
        let timer = LedcTimerDriver::new(
            ledc.timer0,
            &TimerConfig::new()
                .frequency(Hertz(config.pwm_frequency_hz))
                .resolution(Resolution::Bits14),
        )?;
        let mut actuators = Self {
            esc_left: LedcDriver::new(ledc.channel0, &timer, pins.esc_left)?,
            esc_right: LedcDriver::new(ledc.channel1, &timer, pins.esc_right)?,
            pusher: LedcDriver::new(ledc.channel2, &timer, pins.pusher)?,
            _timer: timer,
            config,
        };
        actuators.apply(
            &Outputs {
                throttle_permille: 0,
                pusher: Pusher::Retracted,
            },
            None,
        )?;
        Ok(actuators)
    }

    pub fn apply(
        &mut self,
        outputs: &Outputs,
        held_pusher_us: Option<u16>,
    ) -> Result<(), EspError> {
        let esc_us = self.config.esc.at_permille(outputs.throttle_permille);
        let pusher_us = held_pusher_us.unwrap_or(match outputs.pusher {
            Pusher::Retracted => self.config.pusher.retracted_us,
            Pusher::Extended => self.config.pusher.extended_us,
        });
        let period_us = self.config.pwm_period_us();
        set_pulse(&mut self.esc_left, esc_us, period_us)?;
        set_pulse(&mut self.esc_right, esc_us, period_us)?;
        set_pulse(&mut self.pusher, pusher_us, period_us)
    }
}

fn set_pulse(
    channel: &mut LedcDriver<'static>,
    pulse_us: u16,
    period_us: u32,
) -> Result<(), EspError> {
    let duty = duty_for_pulse(pulse_us, period_us, channel.get_max_duty());
    if channel.get_duty() != duty {
        channel.set_duty(duty)?;
    }
    Ok(())
}
