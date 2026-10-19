use crate::battery::Battery;
use crate::fire::FireTiming;
use crate::pulse::PulseRange;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AxisLimits {
    pub min: u16,
    pub max: u16,
}

impl AxisLimits {
    pub fn clamp(&self, position: u16) -> u16 {
        position.clamp(self.min, self.max)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GimbalConfig {
    pub pan_id: u8,
    pub tilt_id: u8,
    pub pan: AxisLimits,
    pub tilt: AxisLimits,
    pub baud: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PusherConfig {
    pub retracted_us: u16,
    pub extended_us: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TurretConfig {
    pub pwm_frequency_hz: u32,
    pub esc: PulseRange,
    pub pusher: PusherConfig,
    pub gimbal: GimbalConfig,
    pub fire: FireTiming,
    pub battery: Battery,
    pub arm_switch_settle_ms: u64,
}

impl TurretConfig {
    pub const fn pwm_period_us(&self) -> u32 {
        1_000_000 / self.pwm_frequency_hz
    }
}

impl Default for TurretConfig {
    fn default() -> Self {
        Self {
            pwm_frequency_hz: 50,
            esc: PulseRange {
                min_us: 1000,
                max_us: 2000,
            },
            pusher: PusherConfig {
                retracted_us: 500,
                extended_us: 1140,
            },
            gimbal: GimbalConfig {
                pan_id: 0,
                tilt_id: 1,
                pan: AxisLimits { min: 0, max: 1023 },
                tilt: AxisLimits { min: 0, max: 1023 },
                baud: 1_000_000,
            },
            fire: FireTiming::default(),
            battery: Battery {
                cells: 4,
                divider_top_ohm: 39_000,
                divider_bottom_ohm: 8_200,
                empty_cell_mv: 3_300,
                low_cell_mv: 3_500,
                full_cell_mv: 4_200,
                fault_cell_mv: 2_500,
                absent_pack_mv: 5_000,
            },
            arm_switch_settle_ms: 50,
        }
    }
}
