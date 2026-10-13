#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Battery {
    pub cells: u8,
    pub divider_top_ohm: u32,
    pub divider_bottom_ohm: u32,
    pub empty_cell_mv: u16,
    pub full_cell_mv: u16,
}

impl Battery {
    pub fn pack_mv(&self, adc_mv: u16) -> u32 {
        let total = self.divider_top_ohm + self.divider_bottom_ohm;
        u32::from(adc_mv) * total / self.divider_bottom_ohm
    }

    pub fn percent(&self, pack_mv: u32) -> u8 {
        let cells = u32::from(self.cells);
        let empty = u32::from(self.empty_cell_mv) * cells;
        let full = u32::from(self.full_cell_mv) * cells;
        let charge = pack_mv.clamp(empty, full) - empty;
        (charge * 100 / (full - empty)) as u8
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
        full_cell_mv: 4_200,
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
}
