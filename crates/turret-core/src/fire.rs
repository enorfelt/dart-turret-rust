use crate::command::{FireMode, FireRequest};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FireTiming {
    pub spin_up_ms: u64,
    pub push_ms: u64,
    pub retract_ms: u64,
    pub spin_down_ms: u64,
    pub cooldown_ms: u64,
    pub max_shots: u8,
    pub dart_length_mm: u32,
}

impl Default for FireTiming {
    fn default() -> Self {
        Self {
            spin_up_ms: 300,
            push_ms: 300,
            retract_ms: 300,
            spin_down_ms: 400,
            cooldown_ms: 500,
            max_shots: 10,
            dart_length_mm: 70,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pusher {
    Retracted,
    Extended,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Outputs {
    pub throttle_permille: u16,
    pub pusher: Pusher,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Shot { number: u8, speed_kmh: Option<u32> },
    Finished { shots: u8 },
    Aborted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rejection {
    Disarmed,
    Busy,
    CoolingDown,
    NoThrottle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Idle,
    SpinUp,
    Pushing,
    Retracting,
    SpinDown,
}

#[derive(Debug)]
pub struct FireControl {
    timing: FireTiming,
    armed: bool,
    phase: Phase,
    phase_started_ms: u64,
    throttle_permille: u16,
    shots_fired: u8,
    shots_left: u8,
    gate_blocked_us: Option<u32>,
    ready_at_ms: u64,
}

impl FireControl {
    pub fn new(timing: FireTiming) -> Self {
        Self {
            timing,
            armed: false,
            phase: Phase::Idle,
            phase_started_ms: 0,
            throttle_permille: 0,
            shots_fired: 0,
            shots_left: 0,
            gate_blocked_us: None,
            ready_at_ms: 0,
        }
    }

    pub fn is_armed(&self) -> bool {
        self.armed
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    pub fn is_ready(&self, now_ms: u64) -> bool {
        self.phase == Phase::Idle && now_ms >= self.ready_at_ms
    }

    pub fn arm(&mut self) {
        self.armed = true;
    }

    pub fn disarm(&mut self, now_ms: u64) -> Option<Event> {
        self.armed = false;
        if self.phase == Phase::Idle {
            return None;
        }
        self.phase = Phase::Idle;
        self.throttle_permille = 0;
        self.ready_at_ms = now_ms + self.timing.cooldown_ms;
        Some(Event::Aborted)
    }

    pub fn fire(&mut self, request: &FireRequest, now_ms: u64) -> Result<(), Rejection> {
        if !self.armed {
            return Err(Rejection::Disarmed);
        }
        if self.phase != Phase::Idle {
            return Err(Rejection::Busy);
        }
        if now_ms < self.ready_at_ms {
            return Err(Rejection::CoolingDown);
        }
        let throttle_permille = request.throttle_permille();
        if throttle_permille == 0 {
            return Err(Rejection::NoThrottle);
        }
        let max_shots = self.timing.max_shots;
        self.shots_left = match request.mode {
            FireMode::Single => 1,
            FireMode::Burst => request.burst.clamp(1, max_shots),
            FireMode::Auto => max_shots,
        };
        self.shots_fired = 0;
        self.throttle_permille = throttle_permille;
        self.enter(Phase::SpinUp, now_ms);
        Ok(())
    }

    pub fn cease_fire(&mut self, now_ms: u64) {
        match self.phase {
            Phase::SpinUp => self.enter(Phase::SpinDown, now_ms),
            Phase::Pushing => self.shots_left = 1,
            Phase::Retracting => self.shots_left = 0,
            Phase::Idle | Phase::SpinDown => {}
        }
    }

    pub fn record_gate(&mut self, blocked_us: u32) {
        if self.phase == Phase::Pushing && self.gate_blocked_us.is_none() {
            self.gate_blocked_us = Some(blocked_us);
        }
    }

    pub fn tick(&mut self, now_ms: u64) -> Option<Event> {
        let elapsed = now_ms.saturating_sub(self.phase_started_ms);
        match self.phase {
            Phase::Idle => None,
            Phase::SpinUp if elapsed >= self.timing.spin_up_ms => {
                self.enter(Phase::Pushing, now_ms);
                None
            }
            Phase::Pushing if elapsed >= self.timing.push_ms => {
                self.shots_fired += 1;
                self.shots_left = self.shots_left.saturating_sub(1);
                let speed_kmh = self
                    .gate_blocked_us
                    .take()
                    .and_then(|us| self.speed_kmh(us));
                self.enter(Phase::Retracting, now_ms);
                Some(Event::Shot {
                    number: self.shots_fired,
                    speed_kmh,
                })
            }
            Phase::Retracting if elapsed >= self.timing.retract_ms => {
                let next = if self.shots_left > 0 {
                    Phase::Pushing
                } else {
                    Phase::SpinDown
                };
                self.enter(next, now_ms);
                None
            }
            Phase::SpinDown if elapsed >= self.timing.spin_down_ms => {
                self.enter(Phase::Idle, now_ms);
                self.throttle_permille = 0;
                self.ready_at_ms = now_ms + self.timing.cooldown_ms;
                Some(Event::Finished {
                    shots: self.shots_fired,
                })
            }
            _ => None,
        }
    }

    pub fn outputs(&self, now_ms: u64) -> Outputs {
        let throttle_permille = match self.phase {
            Phase::Idle => 0,
            Phase::SpinUp | Phase::Pushing | Phase::Retracting => self.throttle_permille,
            Phase::SpinDown => self.spin_down_throttle(now_ms),
        };
        let pusher = if self.phase == Phase::Pushing {
            Pusher::Extended
        } else {
            Pusher::Retracted
        };
        Outputs {
            throttle_permille,
            pusher,
        }
    }

    fn enter(&mut self, phase: Phase, now_ms: u64) {
        self.phase = phase;
        self.phase_started_ms = now_ms;
        if phase == Phase::Pushing {
            self.gate_blocked_us = None;
        }
    }

    fn spin_down_throttle(&self, now_ms: u64) -> u16 {
        let duration = self.timing.spin_down_ms.max(1);
        let remaining = duration.saturating_sub(now_ms.saturating_sub(self.phase_started_ms));
        (u64::from(self.throttle_permille) * remaining / duration) as u16
    }

    fn speed_kmh(&self, blocked_us: u32) -> Option<u32> {
        (blocked_us > 0).then(|| self.timing.dart_length_mm * 3600 / blocked_us)
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use std::vec::Vec;

    const TIMING: FireTiming = FireTiming {
        spin_up_ms: 300,
        push_ms: 200,
        retract_ms: 250,
        spin_down_ms: 400,
        cooldown_ms: 500,
        max_shots: 5,
        dart_length_mm: 70,
    };

    fn request(mode: FireMode, burst: u8) -> FireRequest {
        FireRequest {
            mode,
            throttle_pct: 50,
            burst,
        }
    }

    fn armed() -> FireControl {
        let mut control = FireControl::new(TIMING);
        control.arm();
        control
    }

    fn run_until_idle(control: &mut FireControl, from_ms: u64) -> (Vec<Event>, u64) {
        let mut events = Vec::new();
        let mut now = from_ms;
        while control.phase() != Phase::Idle {
            now += 10;
            events.extend(control.tick(now));
            assert!(now < from_ms + 60_000, "sequence never finished");
        }
        (events, now)
    }

    #[test]
    fn starts_disarmed_and_refuses_to_fire() {
        let mut control = FireControl::new(TIMING);

        assert_eq!(
            control.fire(&request(FireMode::Single, 1), 0),
            Err(Rejection::Disarmed)
        );
        assert_eq!(control.outputs(0).throttle_permille, 0);
    }

    #[test]
    fn single_shot_follows_spin_up_push_retract_spin_down() {
        let mut control = armed();
        control.fire(&request(FireMode::Single, 1), 1_000).unwrap();

        let spinning = control.outputs(1_000);
        assert_eq!(spinning.throttle_permille, 500);
        assert_eq!(spinning.pusher, Pusher::Retracted);

        assert_eq!(control.tick(1_300), None);
        assert_eq!(control.outputs(1_300).pusher, Pusher::Extended);

        control.record_gate(1_000);
        assert_eq!(
            control.tick(1_500),
            Some(Event::Shot {
                number: 1,
                speed_kmh: Some(252)
            })
        );
        assert_eq!(control.outputs(1_500).pusher, Pusher::Retracted);
        assert_eq!(control.outputs(1_500).throttle_permille, 500);

        assert_eq!(control.tick(1_750), None);
        assert_eq!(control.phase(), Phase::SpinDown);
        assert_eq!(control.outputs(1_950).throttle_permille, 250);

        assert_eq!(control.tick(2_150), Some(Event::Finished { shots: 1 }));
        assert_eq!(control.outputs(2_150).throttle_permille, 0);
    }

    #[test]
    fn shot_without_gate_reading_has_no_speed() {
        let mut control = armed();
        control.fire(&request(FireMode::Single, 1), 0).unwrap();

        let (events, _) = run_until_idle(&mut control, 0);

        assert_eq!(
            events[0],
            Event::Shot {
                number: 1,
                speed_kmh: None
            }
        );
    }

    #[test]
    fn gate_readings_outside_the_push_are_ignored() {
        let mut control = armed();
        control.fire(&request(FireMode::Single, 1), 0).unwrap();
        control.record_gate(1_000);

        let (events, _) = run_until_idle(&mut control, 0);

        assert_eq!(
            events[0],
            Event::Shot {
                number: 1,
                speed_kmh: None
            }
        );
    }

    #[test]
    fn burst_keeps_flywheels_spinning_between_shots() {
        let mut control = armed();
        control.fire(&request(FireMode::Burst, 3), 0).unwrap();
        let mut now = 0;
        let mut shots = 0;

        while control.phase() != Phase::SpinDown {
            now += 10;
            if let Some(Event::Shot { .. }) = control.tick(now) {
                shots += 1;
            }
            if control.phase() != Phase::SpinDown {
                assert_eq!(control.outputs(now).throttle_permille, 500);
            }
        }

        assert_eq!(shots, 3);
    }

    #[test]
    fn burst_and_auto_are_capped_at_max_shots() {
        for mode in [FireMode::Burst, FireMode::Auto] {
            let mut control = armed();
            control.fire(&request(mode, 50), 0).unwrap();

            let (events, _) = run_until_idle(&mut control, 0);

            assert_eq!(events.last(), Some(&Event::Finished { shots: 5 }));
        }
    }

    #[test]
    fn cease_fire_finishes_the_current_shot_then_spins_down() {
        let mut control = armed();
        control.fire(&request(FireMode::Auto, 0), 0).unwrap();
        control.tick(300);
        assert_eq!(control.phase(), Phase::Pushing);

        control.cease_fire(350);
        let (events, _) = run_until_idle(&mut control, 350);

        assert_eq!(events.last(), Some(&Event::Finished { shots: 1 }));
    }

    #[test]
    fn cease_fire_during_spin_up_fires_nothing() {
        let mut control = armed();
        control.fire(&request(FireMode::Single, 1), 0).unwrap();

        control.cease_fire(100);
        let (events, _) = run_until_idle(&mut control, 100);

        assert_eq!(events, [Event::Finished { shots: 0 }]);
    }

    #[test]
    fn disarm_cuts_throttle_and_retracts_immediately() {
        let mut control = armed();
        control.fire(&request(FireMode::Burst, 3), 0).unwrap();
        control.tick(300);

        assert_eq!(control.disarm(310), Some(Event::Aborted));

        let outputs = control.outputs(310);
        assert_eq!(outputs.throttle_permille, 0);
        assert_eq!(outputs.pusher, Pusher::Retracted);
        assert!(!control.is_armed());
        assert_eq!(control.tick(2_000), None);
    }

    #[test]
    fn rejects_fire_while_busy_and_during_cooldown() {
        let mut control = armed();
        let single = request(FireMode::Single, 1);
        control.fire(&single, 0).unwrap();

        assert_eq!(control.fire(&single, 100), Err(Rejection::Busy));

        let (_, finished_at) = run_until_idle(&mut control, 0);
        assert_eq!(
            control.fire(&single, finished_at + 100),
            Err(Rejection::CoolingDown)
        );
        assert_eq!(control.fire(&single, finished_at + 500), Ok(()));
    }

    #[test]
    fn is_ready_only_when_idle_and_cooled_down() {
        let mut control = armed();
        assert!(control.is_ready(0));

        control.fire(&request(FireMode::Single, 1), 0).unwrap();
        assert!(!control.is_ready(10));

        let (_, finished_at) = run_until_idle(&mut control, 0);
        assert!(!control.is_ready(finished_at + 499));
        assert!(control.is_ready(finished_at + 500));
    }

    #[test]
    fn is_ready_after_disarm_cooldown() {
        let mut control = armed();
        control.fire(&request(FireMode::Single, 1), 0).unwrap();
        control.disarm(100);

        assert!(!control.is_ready(599));
        assert!(control.is_ready(600));
    }

    #[test]
    fn zero_speed_is_rejected() {
        let mut control = armed();
        let stopped = FireRequest {
            mode: FireMode::Single,
            throttle_pct: 0,
            burst: 1,
        };

        assert_eq!(control.fire(&stopped, 0), Err(Rejection::NoThrottle));
    }

    #[test]
    fn spin_down_throttle_decreases_monotonically_to_zero() {
        let mut control = armed();
        control.fire(&request(FireMode::Single, 1), 0).unwrap();
        let mut now = 0;
        while control.phase() != Phase::SpinDown {
            now += 10;
            control.tick(now);
        }
        let mut previous = control.outputs(now).throttle_permille;

        for step in 1..=50 {
            let throttle = control.outputs(now + step * 10).throttle_permille;
            assert!(throttle <= previous);
            previous = throttle;
        }

        assert_eq!(previous, 0);
    }
}
