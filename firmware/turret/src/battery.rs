use esp_idf_svc::hal::adc::attenuation::DB_12;
use esp_idf_svc::hal::adc::oneshot::config::{AdcChannelConfig, Calibration};
use esp_idf_svc::hal::adc::oneshot::{AdcChannelDriver, AdcDriver};
use esp_idf_svc::hal::adc::{ADC1, ADCU1};
use esp_idf_svc::hal::gpio::{ADCPin, Gpio1};
use esp_idf_svc::sys::EspError;
use turret_core::battery::Battery;

type BatteryPin = Gpio1<'static>;
type BatteryChannel = <BatteryPin as ADCPin>::AdcChannel;

#[derive(Debug, Clone, Copy)]
pub struct Reading {
    pub pack_mv: u32,
    pub percent: u8,
}

pub struct BatteryMonitor {
    channel: AdcChannelDriver<'static, BatteryChannel, AdcDriver<'static, ADCU1>>,
    battery: Battery,
}

impl BatteryMonitor {
    pub fn new(adc: ADC1<'static>, pin: BatteryPin, battery: Battery) -> Result<Self, EspError> {
        let config = AdcChannelConfig {
            attenuation: DB_12,
            calibration: Calibration::Curve,
            ..Default::default()
        };
        let channel = AdcChannelDriver::new(AdcDriver::new(adc)?, pin, &config)?;
        Ok(Self { channel, battery })
    }

    pub fn read(&mut self) -> Result<Reading, EspError> {
        let pack_mv = self.battery.pack_mv(self.channel.read()?);
        Ok(Reading {
            pack_mv,
            percent: self.battery.percent(pack_mv),
        })
    }
}
