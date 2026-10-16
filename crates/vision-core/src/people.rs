use alloc::vec::Vec;

use heapless::String;
use serde::{Deserialize, Serialize};

pub const MAX_PEOPLE: usize = 32;
pub const MAX_SAMPLES: usize = 5;
pub const MAX_NAME: usize = 24;

const FORMAT_VERSION: u8 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    Protected,
    Target,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Person {
    pub id: u16,
    pub name: String<MAX_NAME>,
    pub role: Role,
    pub samples: Vec<Vec<f32>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeopleError {
    Full,
    InvalidName,
    UnknownPerson,
    SamplesFull,
    WrongEmbeddingLength,
    Corrupt,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct People {
    embedding_len: u16,
    next_id: u16,
    people: Vec<Person>,
}

impl People {
    pub fn new(embedding_len: usize) -> Self {
        Self {
            embedding_len: embedding_len as u16,
            next_id: 1,
            people: Vec::new(),
        }
    }

    pub fn embedding_len(&self) -> usize {
        usize::from(self.embedding_len)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Person> {
        self.people.iter()
    }

    pub fn get(&self, id: u16) -> Option<&Person> {
        self.people.iter().find(|person| person.id == id)
    }

    pub fn len(&self) -> usize {
        self.people.len()
    }

    pub fn is_empty(&self) -> bool {
        self.people.is_empty()
    }

    pub fn add(&mut self, name: &str, role: Role, sample: Vec<f32>) -> Result<u16, PeopleError> {
        if self.people.len() >= MAX_PEOPLE {
            return Err(PeopleError::Full);
        }
        let name = valid_name(name)?;
        self.check_len(&sample)?;
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1).max(1);
        self.people.push(Person {
            id,
            name,
            role,
            samples: alloc::vec![sample],
        });
        Ok(id)
    }

    pub fn add_sample(&mut self, id: u16, sample: Vec<f32>) -> Result<(), PeopleError> {
        self.check_len(&sample)?;
        let person = self
            .people
            .iter_mut()
            .find(|person| person.id == id)
            .ok_or(PeopleError::UnknownPerson)?;
        if person.samples.len() >= MAX_SAMPLES {
            return Err(PeopleError::SamplesFull);
        }
        person.samples.push(sample);
        Ok(())
    }

    pub fn remove(&mut self, id: u16) -> Result<Person, PeopleError> {
        let index = self
            .people
            .iter()
            .position(|person| person.id == id)
            .ok_or(PeopleError::UnknownPerson)?;
        Ok(self.people.remove(index))
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut bytes = alloc::vec![FORMAT_VERSION];
        bytes.extend(postcard::to_allocvec(self).unwrap_or_default());
        bytes
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, PeopleError> {
        let (&version, body) = bytes.split_first().ok_or(PeopleError::Corrupt)?;
        if version != FORMAT_VERSION {
            return Err(PeopleError::Corrupt);
        }
        let people: Self = postcard::from_bytes(body).map_err(|_| PeopleError::Corrupt)?;
        let consistent = people.people.len() <= MAX_PEOPLE
            && people.people.iter().all(|person| {
                !person.samples.is_empty()
                    && person.samples.len() <= MAX_SAMPLES
                    && person
                        .samples
                        .iter()
                        .all(|sample| sample.len() == people.embedding_len())
            });
        if !consistent {
            return Err(PeopleError::Corrupt);
        }
        Ok(people)
    }

    fn check_len(&self, sample: &[f32]) -> Result<(), PeopleError> {
        if sample.len() == self.embedding_len() {
            Ok(())
        } else {
            Err(PeopleError::WrongEmbeddingLength)
        }
    }
}

fn valid_name(name: &str) -> Result<String<MAX_NAME>, PeopleError> {
    let name = name.trim();
    if name.is_empty() || name.chars().any(char::is_control) {
        return Err(PeopleError::InvalidName);
    }
    String::try_from(name).map_err(|_| PeopleError::InvalidName)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    fn sample(value: f32) -> Vec<f32> {
        vec![value; 4]
    }

    #[test]
    fn ids_are_not_reused_after_removal() {
        let mut people = People::new(4);
        let first = people.add("Ada", Role::Target, sample(1.0)).unwrap();
        people.remove(first).unwrap();

        let second = people.add("Bo", Role::Protected, sample(1.0)).unwrap();

        assert_ne!(first, second);
    }

    #[test]
    fn names_are_trimmed_and_validated() {
        let mut people = People::new(4);
        let id = people.add("  Ada  ", Role::Target, sample(1.0)).unwrap();

        assert_eq!(people.get(id).unwrap().name, "Ada");
        assert_eq!(
            people.add("   ", Role::Target, sample(1.0)),
            Err(PeopleError::InvalidName)
        );
        assert_eq!(
            people.add("a\nb", Role::Target, sample(1.0)),
            Err(PeopleError::InvalidName)
        );
        assert_eq!(
            people.add("abcdefghijklmnopqrstuvwxyz", Role::Target, sample(1.0)),
            Err(PeopleError::InvalidName)
        );
    }

    #[test]
    fn samples_must_match_the_embedding_length() {
        let mut people = People::new(4);

        assert_eq!(
            people.add("Ada", Role::Target, vec![1.0; 3]),
            Err(PeopleError::WrongEmbeddingLength)
        );
    }

    #[test]
    fn samples_per_person_are_capped() {
        let mut people = People::new(4);
        let id = people.add("Ada", Role::Target, sample(1.0)).unwrap();
        for _ in 1..MAX_SAMPLES {
            people.add_sample(id, sample(1.0)).unwrap();
        }

        assert_eq!(
            people.add_sample(id, sample(1.0)),
            Err(PeopleError::SamplesFull)
        );
    }

    #[test]
    fn registry_is_capped() {
        let mut people = People::new(4);
        for _ in 0..MAX_PEOPLE {
            people.add("Ada", Role::Target, sample(1.0)).unwrap();
        }

        assert_eq!(
            people.add("Bo", Role::Target, sample(1.0)),
            Err(PeopleError::Full)
        );
    }

    #[test]
    fn registry_round_trips() {
        let mut people = People::new(4);
        let id = people.add("Ada", Role::Target, sample(0.5)).unwrap();
        people.add_sample(id, sample(-0.25)).unwrap();
        people.add("Bo", Role::Protected, sample(1.0)).unwrap();

        assert_eq!(People::decode(&people.encode()), Ok(people));
    }

    #[test]
    fn corrupt_or_foreign_files_are_rejected() {
        let people = People::new(4);
        let mut bytes = people.encode();
        bytes[0] = 99;

        assert_eq!(People::decode(&bytes), Err(PeopleError::Corrupt));
        assert_eq!(People::decode(&[]), Err(PeopleError::Corrupt));
        assert_eq!(
            People::decode(&[FORMAT_VERSION, 0xFF]),
            Err(PeopleError::Corrupt)
        );
    }
}
