#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PulseRange {
    pub min_us: u16,
    pub max_us: u16,
}

impl PulseRange {
    pub fn at_permille(&self, permille: u16) -> u16 {
        let span = u32::from(self.max_us - self.min_us);
        let offset = span * u32::from(permille.min(1000)) / 1000;
        self.min_us + offset as u16
    }

    pub fn clamp(&self, pulse_us: u16) -> u16 {
        pulse_us.clamp(self.min_us, self.max_us)
    }
}

pub fn duty_for_pulse(pulse_us: u16, period_us: u32, max_duty: u32) -> u32 {
    let pulse_us = u64::from(pulse_us).min(u64::from(period_us));
    (pulse_us * u64::from(max_duty) / u64::from(period_us)) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    const ESC: PulseRange = PulseRange {
        min_us: 1000,
        max_us: 2000,
    };

    #[test]
    fn permille_maps_linearly_onto_the_range() {
        assert_eq!(ESC.at_permille(0), 1000);
        assert_eq!(ESC.at_permille(250), 1250);
        assert_eq!(ESC.at_permille(1000), 2000);
    }

    #[test]
    fn permille_above_full_scale_saturates() {
        assert_eq!(ESC.at_permille(5000), 2000);
    }

    #[test]
    fn clamp_keeps_pulses_inside_the_range() {
        assert_eq!(ESC.clamp(400), 1000);
        assert_eq!(ESC.clamp(1500), 1500);
        assert_eq!(ESC.clamp(2400), 2000);
    }

    #[test]
    fn duty_at_50_hz_with_14_bit_resolution() {
        let max_duty = (1 << 14) - 1;

        assert_eq!(duty_for_pulse(1000, 20_000, max_duty), 819);
        assert_eq!(duty_for_pulse(2000, 20_000, max_duty), 1638);
        assert_eq!(duty_for_pulse(25_000, 20_000, max_duty), max_duty);
    }
}
