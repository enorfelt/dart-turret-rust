use esp_idf_svc::hal::gpio::{AnyInputPin, AnyOutputPin, Gpio1, Pins};

pub struct Board {
    pub esc_left: AnyOutputPin<'static>,
    pub esc_right: AnyOutputPin<'static>,
    pub pusher: AnyOutputPin<'static>,
    pub servo_tx: AnyOutputPin<'static>,
    pub servo_rx: AnyInputPin<'static>,
    pub light_gate: AnyInputPin<'static>,
    pub arm_switch: AnyInputPin<'static>,
    pub battery: Gpio1<'static>,
}

impl Board {
    pub fn new(pins: Pins) -> Self {
        Self {
            esc_left: pins.gpio17.into(),
            esc_right: pins.gpio18.into(),
            pusher: pins.gpio41.into(),
            servo_tx: pins.gpio14.into(),
            servo_rx: pins.gpio21.into(),
            light_gate: pins.gpio42.into(),
            arm_switch: pins.gpio47.into(),
            battery: pins.gpio1,
        }
    }
}
