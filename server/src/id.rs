use std::hash::Hash;
use std::marker::PhantomData;

use serde::Deserialize;

#[derive(Debug)]
pub struct Id<T> {
    value: u64,
    phantom: PhantomData<T>,
}

impl<T> Id<T> {
    pub fn new(value: u64) -> Id<T> {
        Id {
            value,
            phantom: PhantomData,
        }
    }
}

impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value,
            phantom: self.phantom,
        }
    }
}

impl<T> Copy for Id<T> {}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<T> Eq for Id<T> {
    fn assert_receiver_is_total_eq(&self) {}
}

impl<T> Hash for Id<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}

impl<'de, T> Deserialize<'de> for Id<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Id::new(u64::deserialize(deserializer)?))
    }
}
