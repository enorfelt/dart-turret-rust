use crate::people::{People, Role};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Candidate {
    pub person: u16,
    pub role: Role,
    pub similarity: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Matches {
    pub best: Option<Candidate>,
    pub runner_up: Option<Candidate>,
}

pub fn similarity(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}

pub fn best_matches(people: &People, embedding: &[f32]) -> Matches {
    let mut matches = Matches::default();
    for person in people.iter() {
        let Some(similarity) = person
            .samples
            .iter()
            .map(|sample| similarity(sample, embedding))
            .reduce(f32::max)
        else {
            continue;
        };
        let candidate = Candidate {
            person: person.id,
            role: person.role,
            similarity,
        };
        match matches.best {
            Some(best) if best.similarity >= similarity => {
                if matches
                    .runner_up
                    .is_none_or(|runner_up| runner_up.similarity < similarity)
                {
                    matches.runner_up = Some(candidate);
                }
            }
            _ => {
                matches.runner_up = matches.best;
                matches.best = Some(candidate);
            }
        }
    }
    matches
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    fn people() -> People {
        let mut people = People::new(3);
        let ada = people
            .add("Ada", Role::Target, vec![1.0, 0.0, 0.0])
            .unwrap();
        people.add_sample(ada, vec![0.0, 1.0, 0.0]).unwrap();
        people
            .add("Bo", Role::Protected, vec![0.0, 0.0, 1.0])
            .unwrap();
        people.add("Cy", Role::Target, vec![0.6, 0.0, 0.8]).unwrap();
        people
    }

    #[test]
    fn similarity_of_unit_vectors_is_their_dot_product() {
        assert_eq!(similarity(&[0.6, 0.8], &[0.6, 0.8]), 1.0);
        assert_eq!(similarity(&[1.0, 0.0], &[0.0, 1.0]), 0.0);
    }

    #[test]
    fn a_person_scores_with_their_closest_sample() {
        let matches = best_matches(&people(), &[0.0, 1.0, 0.0]);

        let best = matches.best.unwrap();
        assert_eq!((best.person, best.similarity), (1, 1.0));
    }

    #[test]
    fn best_and_runner_up_are_distinct_people() {
        let matches = best_matches(&people(), &[0.0, 0.0, 1.0]);

        assert_eq!(matches.best.unwrap().person, 2);
        assert_eq!(matches.best.unwrap().role, Role::Protected);
        let runner_up = matches.runner_up.unwrap();
        assert_eq!(runner_up.person, 3);
        assert!((runner_up.similarity - 0.8).abs() < 1e-6);
    }

    #[test]
    fn runner_up_is_found_when_it_comes_after_the_best() {
        let matches = best_matches(&people(), &[0.6, 0.0, 0.8]);

        assert_eq!(matches.best.unwrap().person, 3);
        assert_eq!(matches.runner_up.unwrap().person, 2);
    }

    #[test]
    fn empty_gallery_has_no_matches() {
        assert_eq!(
            best_matches(&People::new(3), &[1.0, 0.0, 0.0]),
            Matches::default()
        );
    }
}
