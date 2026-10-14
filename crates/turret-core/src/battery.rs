#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Battery {
    pub cells: u8,
    pub divider_top_ohm: u32,
    pub divider_bottom_ohm: u32,
    pub empty_cell_mv: u16,
    pub low_cell_mv: u16,
    pub full_cell_mv: u16,
    pub fault_cell_mv: u16,
    pub absent_pack_mv: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Health {
    Absent,
    Fault,
    Empty,
    Low,
    Good,
}

impl Health {
    pub fn allows_arming(self) -> bool {
        !matches!(self, Self::Fault | Self::Empty)
    }
}

impl Battery {
    pub fn pack_mv(&self, adc_mv: u16) -> u32 {
        let total = self.divider_top_ohm + self.divider_bottom_ohm;
        u32::from(adc_mv) * total / self.divider_bottom_ohm
    }

    pub fn percent(&self, pack_mv: u32) -> u8 {
        let empty = self.pack_level(self.empty_cell_mv);
        let full = self.pack_level(self.full_cell_mv);
        let charge = pack_mv.clamp(empty, full) - empty;
        (charge * 100 / (full - empty)) as u8
    }

    pub fn health(&self, pack_mv: u32) -> Health {
        if pack_mv < self.absent_pack_mv {
            Health::Absent
        } else if pack_mv < self.pack_level(self.fault_cell_mv) {
            Health::Fault
        } else if pack_mv < self.pack_level(self.empty_cell_mv) {
            Health::Empty
        } else if pack_mv < self.pack_level(self.low_cell_mv) {
            Health::Low
        } else {
            Health::Good
        }
    }

    fn pack_level(&self, cell_mv: u16) -> u32 {
        u32::from(cell_mv) * u32::from(self.cells)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PACK_4S: Battery = Battery {
        cells: 4,
        divider_top_ohm: 39_000,
        divider_bottom_ohm: 8_200,
        empty_cell_mv: 3_300,
        low_cell_mv: 3_500,
        full_cell_mv: 4_200,
        fault_cell_mv: 2_500,
        absent_pack_mv: 5_000,
    };

    #[test]
    fn divider_scales_adc_reading_to_pack_voltage() {
        assert_eq!(PACK_4S.pack_mv(2_920), 16_807);
    }

    #[test]
    fn percent_is_linear_between_empty_and_full() {
        assert_eq!(PACK_4S.percent(13_200), 0);
        assert_eq!(PACK_4S.percent(15_000), 50);
        assert_eq!(PACK_4S.percent(16_800), 100);
    }

    #[test]
    fn percent_saturates_outside_the_cell_range() {
        assert_eq!(PACK_4S.percent(9_000), 0);
        assert_eq!(PACK_4S.percent(17_500), 100);
    }

    #[test]
    fn usb_power_without_a_pack_reads_as_absent() {
        assert_eq!(PACK_4S.health(0), Health::Absent);
        assert_eq!(PACK_4S.health(4_999), Health::Absent);
        assert!(Health::Absent.allows_arming());
    }

    #[test]
    fn one_reversed_cell_is_a_fault() {
        assert_eq!(PACK_4S.health(2 * 4_200), Health::Fault);
        assert_eq!(PACK_4S.health(2 * 3_300), Health::Fault);
        assert_eq!(PACK_4S.health(9_999), Health::Fault);
        assert!(!Health::Fault.allows_arming());
    }

    #[test]
    fn pack_below_empty_refuses_arming() {
        assert_eq!(PACK_4S.health(10_000), Health::Empty);
        assert_eq!(PACK_4S.health(13_199), Health::Empty);
        assert!(!Health::Empty.allows_arming());
    }

    #[test]
    fn low_pack_still_arms() {
        assert_eq!(PACK_4S.health(13_200), Health::Low);
        assert_eq!(PACK_4S.health(13_999), Health::Low);
        assert!(Health::Low.allows_arming());
    }

    #[test]
    fn charged_pack_is_good() {
        assert_eq!(PACK_4S.health(14_000), Health::Good);
        assert_eq!(PACK_4S.health(16_800), Health::Good);
        assert!(Health::Good.allows_arming());
    }
}
