use std::collections::HashSet;

use crate::field::{Nargable, Nargs};

/// Behaviour allowing for multiple (0 to many) items T to be collected together.
pub trait Collectable<T> {
    /// Add a value to this `Collectable`.
    fn add(&mut self, item: T);
}

impl<T> Collectable<T> for Vec<T> {
    fn add(&mut self, item: T) {
        self.push(item);
    }
}

impl<T> Nargable for Vec<T> {
    fn nargs() -> Nargs {
        Nargs::AtLeastOne
    }
}

impl<T: std::cmp::Eq + std::hash::Hash> Collectable<T> for HashSet<T> {
    fn add(&mut self, item: T) {
        self.insert(item);
    }
}

impl<T> Nargable for HashSet<T> {
    fn nargs() -> Nargs {
        Nargs::AtLeastOne
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vec() {
        let mut collection: Vec<u32> = Vec::default();
        collection.add(1);
        collection.add(0);
        assert_eq!(collection, vec![1, 0]);

        assert!(matches!(Vec::<u32>::nargs(), Nargs::AtLeastOne));
    }

    #[test]
    fn hash_set() {
        let mut collection: HashSet<u32> = HashSet::default();
        collection.add(1);
        collection.add(0);
        collection.add(1);
        assert_eq!(collection, HashSet::from([1, 0]));

        assert!(matches!(HashSet::<u32>::nargs(), Nargs::AtLeastOne));
    }
}
