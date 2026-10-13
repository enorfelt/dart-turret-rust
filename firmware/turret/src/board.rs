use esp_idf_svc::hal::gpio::{AnyInputPin, AnyOutputPin, Gpio16, Pins};

pub struct Board {
    pub esc_left: AnyOutputPin<'static>,
    pub esc_right: AnyOutputPin<'static>,
    pub pusher: AnyOutputPin<'static>,
    pub servo_tx: AnyOutputPin<'static>,
    pub servo_rx: AnyInputPin<'static>,
    pub light_gate: AnyInputPin<'static>,
    pub battery: Gpio16<'static>,
}

impl Board {
    pub fn new(pins: Pins) -> Self {
        Self {
            esc_left: pins.gpio17.into(),
            esc_right: pins.gpio18.into(),
            pusher: pins.gpio39.into(),
            servo_tx: pins.gpio33.into(),
            servo_rx: pins.gpio34.into(),
            light_gate: pins.gpio38.into(),
            battery: pins.gpio16,
        }
    }
}
