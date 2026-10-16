#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rect {
    pub x0: i16,
    pub y0: i16,
    pub x1: i16,
    pub y1: i16,
}

impl Rect {
    pub const fn new(x0: i16, y0: i16, x1: i16, y1: i16) -> Self {
        Self { x0, y0, x1, y1 }
    }

    pub fn width(&self) -> i32 {
        (i32::from(self.x1) - i32::from(self.x0)).max(0)
    }

    pub fn height(&self) -> i32 {
        (i32::from(self.y1) - i32::from(self.y0)).max(0)
    }

    pub fn area(&self) -> i32 {
        self.width() * self.height()
    }

    pub fn iou(&self, other: &Rect) -> f32 {
        let overlap = Rect {
            x0: self.x0.max(other.x0),
            y0: self.y0.max(other.y0),
            x1: self.x1.min(other.x1),
            y1: self.y1.min(other.y1),
        }
        .area();
        let union = self.area() + other.area() - overlap;
        if union <= 0 {
            return 0.0;
        }
        overlap as f32 / union as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_rects_have_iou_one() {
        let rect = Rect::new(10, 10, 50, 60);
        assert_eq!(rect.iou(&rect), 1.0);
    }

    #[test]
    fn disjoint_rects_have_iou_zero() {
        assert_eq!(Rect::new(0, 0, 10, 10).iou(&Rect::new(20, 20, 30, 30)), 0.0);
    }

    #[test]
    fn half_shifted_rect_overlaps_a_third() {
        let iou = Rect::new(0, 0, 10, 10).iou(&Rect::new(5, 0, 15, 10));
        assert!((iou - 1.0 / 3.0).abs() < 1e-6);
    }

    #[test]
    fn inverted_rect_has_no_area() {
        assert_eq!(Rect::new(10, 10, 0, 0).area(), 0);
    }
}
