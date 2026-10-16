use esp_idf_svc::hal::gpio::{AnyIOPin, Pins};

pub struct CameraPins {
    pub xclk: AnyIOPin<'static>,
    pub sda: AnyIOPin<'static>,
    pub scl: AnyIOPin<'static>,
    pub data: [AnyIOPin<'static>; 8],
    pub vsync: AnyIOPin<'static>,
    pub href: AnyIOPin<'static>,
    pub pclk: AnyIOPin<'static>,
}

pub struct SdPins {
    pub cmd: AnyIOPin<'static>,
    pub clk: AnyIOPin<'static>,
    pub d0: AnyIOPin<'static>,
}

pub struct Board {
    pub camera: CameraPins,
    pub sd: SdPins,
}

impl Board {
    pub fn new(pins: Pins) -> Self {
        Self {
            camera: CameraPins {
                xclk: pins.gpio15.into(),
                sda: pins.gpio4.into(),
                scl: pins.gpio5.into(),
                data: [
                    pins.gpio11.into(),
                    pins.gpio9.into(),
                    pins.gpio8.into(),
                    pins.gpio10.into(),
                    pins.gpio12.into(),
                    pins.gpio18.into(),
                    pins.gpio17.into(),
                    pins.gpio16.into(),
                ],
                vsync: pins.gpio6.into(),
                href: pins.gpio7.into(),
                pclk: pins.gpio13.into(),
            },
            sd: SdPins {
                cmd: pins.gpio38.into(),
                clk: pins.gpio39.into(),
                d0: pins.gpio40.into(),
            },
        }
    }
}
