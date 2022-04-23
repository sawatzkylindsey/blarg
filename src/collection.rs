use std::collections::HashSet;

/// Behaviour allowing for multiple (1 to many) items T to be collected together.
pub trait Collectable<T> {
    fn add(&mut self, item: T) -> Result<(), ()>;
}

impl<T> Collectable<T> for Vec<T> {
    fn add(&mut self, item: T) -> Result<(), ()> {
        self.push(item);
        Ok(())
    }
}

impl<T: std::cmp::Eq + std::hash::Hash> Collectable<T> for HashSet<T> {
    fn add(&mut self, item: T) -> Result<(), ()> {
        self.insert(item);
        Ok(())
    }
}

impl<T> Collectable<T> for Option<T> {
    fn add(&mut self, item: T) -> Result<(), ()> {
        self.replace(item);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vec() {
        let mut collection: Vec<u32> = Vec::default();
        collection.add(1).unwrap();
        collection.add(0).unwrap();
        assert_eq!(collection, vec![1, 0]);
    }

    #[test]
    fn hash_set() {
        let mut collection: HashSet<u32> = HashSet::default();
        collection.add(1).unwrap();
        collection.add(0).unwrap();
        collection.add(1).unwrap();
        assert_eq!(collection, HashSet::from([1, 0]));
    }

    #[test]
    fn option() {
        let mut collection: Option<u32> = None;
        collection.add(1).unwrap();
        assert_eq!(collection, Some(1));

        let mut collection: Option<u32> = Some(2);
        collection.add(1).unwrap();
        assert_eq!(collection, Some(1));
    }
}
