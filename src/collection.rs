use std::collections::HashSet;

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

impl<T: std::cmp::Eq + std::hash::Hash> Collectable<T> for HashSet<T> {
    fn add(&mut self, item: T) {
        self.insert(item);
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
    }

    #[test]
    fn hash_set() {
        let mut collection: HashSet<u32> = HashSet::default();
        collection.add(1);
        collection.add(0);
        collection.add(1);
        assert_eq!(collection, HashSet::from([1, 0]));
    }
}
