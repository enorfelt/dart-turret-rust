use crate::config::VisionConfig;
use crate::gallery::{Candidate, Matches};
use crate::people::Role;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Class {
    #[default]
    Unknown,
    Protected,
    Target,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Identity {
    class: Class,
    person: Option<u16>,
    streak: u8,
    similarity: f32,
}

impl Identity {
    pub fn class(&self) -> Class {
        self.class
    }

    pub fn person(&self) -> Option<u16> {
        match self.class {
            Class::Unknown => None,
            Class::Protected | Class::Target => self.person,
        }
    }

    pub fn confidence_percent(&self) -> u8 {
        match self.class {
            Class::Unknown => 0,
            Class::Protected | Class::Target => (self.similarity.clamp(0.0, 1.0) * 100.0) as u8,
        }
    }

    pub fn is_settled(&self) -> bool {
        self.class == Class::Protected
    }

    pub fn observe(&mut self, matches: &Matches, config: &VisionConfig) {
        if self.class == Class::Protected {
            return;
        }
        let matching = |candidate: &Option<Candidate>| {
            candidate.filter(|candidate| candidate.similarity >= config.match_similarity)
        };
        let best = matching(&matches.best);
        let runner_up = matching(&matches.runner_up);

        if let Some(protected) = [best, runner_up]
            .into_iter()
            .flatten()
            .find(|candidate| candidate.role == Role::Protected)
        {
            *self = Self {
                class: Class::Protected,
                person: Some(protected.person),
                streak: 0,
                similarity: protected.similarity,
            };
            return;
        }

        let clear = |best: &Candidate| {
            matches.runner_up.is_none_or(|runner_up| {
                best.similarity - runner_up.similarity >= config.match_margin
            })
        };
        match best.filter(clear) {
            Some(best) => {
                let streak = if self.person == Some(best.person) {
                    self.streak.saturating_add(1)
                } else {
                    1
                };
                *self = Self {
                    class: if streak >= config.target_confirmations {
                        Class::Target
                    } else {
                        Class::Unknown
                    },
                    person: Some(best.person),
                    streak,
                    similarity: best.similarity,
                };
            }
            None => *self = Self::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONFIG: VisionConfig = VisionConfig {
        min_face_score: 0.5,
        track_iou: 0.3,
        track_max_misses: 5,
        match_similarity: 0.5,
        match_margin: 0.1,
        target_confirmations: 3,
        recheck_ms: 1_000,
        enroll_samples: 3,
        enroll_timeout_ms: 10_000,
    };

    fn candidate(person: u16, role: Role, similarity: f32) -> Candidate {
        Candidate {
            person,
            role,
            similarity,
        }
    }

    fn only(best: Candidate) -> Matches {
        Matches {
            best: Some(best),
            runner_up: None,
        }
    }

    fn observed(matches: &[Matches]) -> Identity {
        let mut identity = Identity::default();
        for m in matches {
            identity.observe(m, &CONFIG);
        }
        identity
    }

    #[test]
    fn target_needs_consecutive_confirmations() {
        let ada = only(candidate(1, Role::Target, 0.7));

        assert_eq!(observed(&[ada, ada]).class(), Class::Unknown);
        let identity = observed(&[ada, ada, ada]);
        assert_eq!(identity.class(), Class::Target);
        assert_eq!(identity.person(), Some(1));
        assert_eq!(identity.confidence_percent(), 70);
    }

    #[test]
    fn unknown_person_is_never_a_target() {
        let stranger = only(candidate(1, Role::Target, 0.3));

        assert_eq!(observed(&[stranger; 5]).class(), Class::Unknown);
        assert_eq!(observed(&[Matches::default(); 5]).class(), Class::Unknown);
    }

    #[test]
    fn switching_person_restarts_the_streak() {
        let ada = only(candidate(1, Role::Target, 0.7));
        let cy = only(candidate(3, Role::Target, 0.7));

        assert_eq!(observed(&[ada, ada, cy, ada]).class(), Class::Unknown);
    }

    #[test]
    fn a_failed_recheck_drops_a_target() {
        let ada = only(candidate(1, Role::Target, 0.7));

        let identity = observed(&[ada, ada, ada, Matches::default()]);

        assert_eq!(identity.class(), Class::Unknown);
        assert_eq!(identity.person(), None);
    }

    #[test]
    fn ambiguous_match_is_unknown() {
        let close = Matches {
            best: Some(candidate(1, Role::Target, 0.70)),
            runner_up: Some(candidate(3, Role::Target, 0.65)),
        };

        assert_eq!(observed(&[close; 5]).class(), Class::Unknown);
    }

    #[test]
    fn protected_match_wins_even_as_runner_up() {
        let close = Matches {
            best: Some(candidate(1, Role::Target, 0.70)),
            runner_up: Some(candidate(2, Role::Protected, 0.55)),
        };

        let identity = observed(&[close]);

        assert_eq!(identity.class(), Class::Protected);
        assert_eq!(identity.person(), Some(2));
    }

    #[test]
    fn protected_is_sticky_for_the_track() {
        let bo = only(candidate(2, Role::Protected, 0.8));
        let ada = only(candidate(1, Role::Target, 0.9));

        let identity = observed(&[bo, ada, ada, ada, Matches::default()]);

        assert_eq!(identity.class(), Class::Protected);
        assert!(identity.is_settled());
    }
}
