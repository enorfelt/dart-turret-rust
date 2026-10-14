#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArmSwitch {
    settle_ms: u64,
    closed_since_ms: Option<u64>,
}

impl ArmSwitch {
    pub const fn new(settle_ms: u64) -> Self {
        Self {
            settle_ms,
            closed_since_ms: None,
        }
    }

    pub fn update(&mut self, closed: bool, now_ms: u64) {
        match (closed, self.closed_since_ms) {
            (false, _) => self.closed_since_ms = None,
            (true, None) => self.closed_since_ms = Some(now_ms),
            (true, Some(_)) => {}
        }
    }

    pub fn is_on(&self, now_ms: u64) -> bool {
        self.closed_since_ms
            .is_some_and(|since| now_ms.saturating_sub(since) >= self.settle_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_off() {
        assert!(!ArmSwitch::new(50).is_on(1_000));
    }

    #[test]
    fn turns_on_after_staying_closed_for_the_settle_time() {
        let mut switch = ArmSwitch::new(50);
        switch.update(true, 100);
        switch.update(true, 149);
        assert!(!switch.is_on(149));
        switch.update(true, 150);
        assert!(switch.is_on(150));
    }

    #[test]
    fn bounce_restarts_the_settle_time() {
        let mut switch = ArmSwitch::new(50);
        switch.update(true, 100);
        switch.update(false, 120);
        switch.update(true, 130);
        assert!(!switch.is_on(160));
        assert!(switch.is_on(180));
    }

    #[test]
    fn turns_off_at_once_when_opened() {
        let mut switch = ArmSwitch::new(50);
        switch.update(true, 0);
        assert!(switch.is_on(100));
        switch.update(false, 101);
        assert!(!switch.is_on(101));
    }
}
