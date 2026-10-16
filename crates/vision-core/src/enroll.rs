use alloc::vec::Vec;

use crate::config::VisionConfig;
use crate::gallery::Matches;
use crate::people::Role;

#[derive(Debug, Clone, PartialEq)]
pub enum Target {
    New {
        name: heapless::String<{ crate::people::MAX_NAME }>,
        role: Role,
    },
    Existing(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnrollError {
    Timeout,
    AlreadyEnrolled(u16),
    DifferentPerson,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Wait,
    Embed,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Enrollment {
    pub target: Target,
    started_ms: u32,
    samples: Vec<Vec<f32>>,
}

impl Enrollment {
    pub fn new(target: Target, now_ms: u32) -> Self {
        Self {
            target,
            started_ms: now_ms,
            samples: Vec::new(),
        }
    }

    pub fn step(
        &self,
        faces: usize,
        now_ms: u32,
        config: &VisionConfig,
    ) -> Result<Step, EnrollError> {
        if now_ms.wrapping_sub(self.started_ms) > config.enroll_timeout_ms {
            return Err(EnrollError::Timeout);
        }
        Ok(if faces == 1 { Step::Embed } else { Step::Wait })
    }

    pub fn add(
        &mut self,
        sample: Vec<f32>,
        matches: &Matches,
        config: &VisionConfig,
    ) -> Result<Option<Vec<Vec<f32>>>, EnrollError> {
        let known = matches
            .best
            .filter(|best| best.similarity >= config.match_similarity)
            .map(|best| best.person);
        match (&self.target, known) {
            (Target::New { .. }, Some(person)) => return Err(EnrollError::AlreadyEnrolled(person)),
            (Target::Existing(id), Some(person)) if person != *id => {
                return Err(EnrollError::DifferentPerson)
            }
            (Target::Existing(_), None) => return Err(EnrollError::DifferentPerson),
            _ => {}
        }
        self.samples.push(sample);
        let wanted = match self.target {
            Target::New { .. } => usize::from(config.enroll_samples),
            Target::Existing(_) => 1,
        };
        Ok((self.samples.len() >= wanted).then(|| core::mem::take(&mut self.samples)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gallery::Candidate;
    use alloc::vec;

    const CONFIG: VisionConfig = VisionConfig {
        min_face_score: 0.5,
        track_iou: 0.3,
        track_max_misses: 2,
        match_similarity: 0.5,
        match_margin: 0.1,
        target_confirmations: 3,
        recheck_ms: 1_000,
        enroll_samples: 2,
        enroll_timeout_ms: 5_000,
    };

    fn new_person() -> Enrollment {
        Enrollment::new(
            Target::New {
                name: "Ada".try_into().unwrap(),
                role: Role::Target,
            },
            1_000,
        )
    }

    fn matching(person: u16, similarity: f32) -> Matches {
        Matches {
            best: Some(Candidate {
                person,
                role: Role::Protected,
                similarity,
            }),
            runner_up: None,
        }
    }

    #[test]
    fn embeds_only_with_exactly_one_face_in_view() {
        let enrollment = new_person();

        assert_eq!(enrollment.step(0, 1_100, &CONFIG), Ok(Step::Wait));
        assert_eq!(enrollment.step(2, 1_100, &CONFIG), Ok(Step::Wait));
        assert_eq!(enrollment.step(1, 1_100, &CONFIG), Ok(Step::Embed));
    }

    #[test]
    fn gives_up_after_the_timeout() {
        assert_eq!(
            new_person().step(1, 6_001, &CONFIG),
            Err(EnrollError::Timeout)
        );
    }

    #[test]
    fn new_person_collects_the_configured_samples() {
        let mut enrollment = new_person();

        assert_eq!(
            enrollment.add(vec![1.0], &Matches::default(), &CONFIG),
            Ok(None)
        );
        assert_eq!(
            enrollment.add(vec![2.0], &matching(4, 0.3), &CONFIG),
            Ok(Some(vec![vec![1.0], vec![2.0]]))
        );
    }

    #[test]
    fn known_face_cannot_be_enrolled_again_under_a_new_name() {
        let mut enrollment = new_person();

        assert_eq!(
            enrollment.add(vec![1.0], &matching(4, 0.8), &CONFIG),
            Err(EnrollError::AlreadyEnrolled(4))
        );
    }

    #[test]
    fn extra_sample_must_come_from_the_same_person() {
        let mut enrollment = Enrollment::new(Target::Existing(4), 0);

        assert_eq!(
            enrollment.add(vec![1.0], &matching(5, 0.8), &CONFIG),
            Err(EnrollError::DifferentPerson)
        );
        assert_eq!(
            enrollment.add(vec![1.0], &Matches::default(), &CONFIG),
            Err(EnrollError::DifferentPerson)
        );
        assert_eq!(
            enrollment.add(vec![1.0], &matching(4, 0.6), &CONFIG),
            Ok(Some(vec![vec![1.0]]))
        );
    }
}
