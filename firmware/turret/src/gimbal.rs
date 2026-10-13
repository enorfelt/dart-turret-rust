use esp_idf_svc::hal::delay::TickType;
use esp_idf_svc::hal::gpio::{AnyIOPin, AnyInputPin, AnyOutputPin};
use esp_idf_svc::hal::uart::{config::Config, UartDriver, UART1};
use esp_idf_svc::hal::units::Hertz;
use esp_idf_svc::sys::{EspError, ESP_ERR_TIMEOUT};
use turret_core::config::GimbalConfig;
use turret_core::scs::{self, Status, StatusError, MAX_PACKET};

const REPLY_TIMEOUT: TickType = TickType::new_millis(20);

#[derive(Debug)]
pub enum PingError {
    Bus(EspError),
    NoReply,
    Corrupt(StatusError),
}

impl core::fmt::Display for PingError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Bus(error) => write!(f, "bus error: {error}"),
            Self::NoReply => write!(f, "no reply"),
            Self::Corrupt(error) => write!(f, "corrupt reply: {error:?}"),
        }
    }
}

pub struct Gimbal {
    uart: UartDriver<'static>,
    config: GimbalConfig,
}

impl Gimbal {
    pub fn new(
        uart: UART1<'static>,
        tx: AnyOutputPin<'static>,
        rx: AnyInputPin<'static>,
        config: GimbalConfig,
    ) -> Result<Self, EspError> {
        let uart = UartDriver::new(
            uart,
            tx,
            rx,
            Option::<AnyIOPin>::None,
            Option::<AnyIOPin>::None,
            &Config::new().baudrate(Hertz(config.baud)),
        )?;
        Ok(Self { uart, config })
    }

    pub fn move_to(
        &self,
        pan: u16,
        tilt: u16,
        pan_speed: u16,
        tilt_speed: u16,
    ) -> Result<(u16, u16), EspError> {
        let pan = self.config.pan.clamp(pan);
        let tilt = self.config.tilt.clamp(tilt);
        self.uart.write(&scs::reg_write_position(
            self.config.pan_id,
            pan,
            0,
            pan_speed,
        ))?;
        self.uart.write(&scs::reg_write_position(
            self.config.tilt_id,
            tilt,
            0,
            tilt_speed,
        ))?;
        self.uart.write(&scs::action())?;
        Ok((pan, tilt))
    }

    pub fn ping(&self, id: u8) -> Result<Status, PingError> {
        let request = scs::ping(id);
        self.uart.clear_rx().map_err(PingError::Bus)?;
        self.uart.write(&request).map_err(PingError::Bus)?;
        let mut reply = [0u8; 2 * MAX_PACKET];
        let received = match self.uart.read(&mut reply, REPLY_TIMEOUT.ticks()) {
            Ok(received) => received,
            Err(error) if error.code() == ESP_ERR_TIMEOUT => 0,
            Err(error) => return Err(PingError::Bus(error)),
        };
        let reply = scs::strip_echo(&reply[..received], &request);
        if reply.is_empty() {
            return Err(PingError::NoReply);
        }
        scs::parse_status(reply).map_err(PingError::Corrupt)
    }
}
