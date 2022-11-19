use std::cell::RefCell;
use std::collections::HashSet;
use std::marker::PhantomData;
use std::rc::Rc;
use std::str::FromStr;

use crate::api::capture::*;
use crate::model::Nargs;

/// An option parameter that takes a single value (`Nargs::Precisely(1)`).
pub struct Scalar<'ap, T> {
    variable: Rc<RefCell<&'ap mut T>>,
}

impl<'ap, T> CliOption for Scalar<'ap, T> {}
impl<'ap, T> CliArgument for Scalar<'ap, T> {}

impl<'ap, T> Scalar<'ap, T> {
    /// Create a scalar parameter.
    pub fn new(variable: &'ap mut T) -> Self {
        Self {
            variable: Rc::new(RefCell::new(variable)),
        }
    }
}

impl<'ap, T> GenericCapturable<'ap, T> for Scalar<'ap, T>
where
    T: FromStr,
{
    fn matched(&mut self) {
        // Do nothing.
    }

    fn capture(&mut self, token: &str) -> Result<(), InvalidConversion> {
        let result: Result<T, InvalidConversion> =
            T::from_str(token).map_err(|_| InvalidConversion {
                token: token.to_string(),
                type_name: std::any::type_name::<T>(),
            });
        let value = result?;
        **self.variable.borrow_mut() = value;
        Ok(())
    }

    fn nargs(&self) -> Nargs {
        Nargs::Precisely(1)
    }
}

/// An option parameter that takes no values (`Nargs::Precisely(0)`).
pub struct Switch<'ap, T> {
    variable: Rc<RefCell<&'ap mut T>>,
    target: Option<T>,
}

impl<'ap, T> CliOption for Switch<'ap, T> {}

impl<'ap, T> Switch<'ap, T> {
    /// Create a switch parameter (option).
    pub fn new(variable: &'ap mut T, target: T) -> Self {
        Self {
            variable: Rc::new(RefCell::new(variable)),
            target: Some(target),
        }
    }
}

impl<'ap, T> GenericCapturable<'ap, T> for Switch<'ap, T> {
    fn matched(&mut self) {
        **self.variable.borrow_mut() = self
            .target
            .take()
            .expect("internal error - must be able to take the Switch#target");
    }

    fn capture(&mut self, _token: &str) -> Result<(), InvalidConversion> {
        unreachable!("internal error - must not capture on a Switch");
    }

    fn nargs(&self) -> Nargs {
        Nargs::Precisely(0)
    }
}

/// An option parameter that maps down to `Option`, taking a single value (`Nargs::Precisely(1)`).
pub struct Optional<'ap, T> {
    variable: Rc<RefCell<&'ap mut Option<T>>>,
}

impl<'ap, T> CliOption for Optional<'ap, T> {}

impl<'ap, T> Optional<'ap, T> {
    /// Create an optional parameter (option).
    pub fn new(variable: &'ap mut Option<T>) -> Self {
        Self {
            variable: Rc::new(RefCell::new(variable)),
        }
    }
}

impl<'ap, T> GenericCapturable<'ap, T> for Optional<'ap, T>
where
    T: FromStr,
{
    fn matched(&mut self) {
        // Do nothing
    }

    fn capture(&mut self, token: &str) -> Result<(), InvalidConversion> {
        let result: Result<T, InvalidConversion> =
            T::from_str(token).map_err(|_| InvalidConversion {
                token: token.to_string(),
                type_name: std::any::type_name::<T>(),
            });
        let value = result?;
        self.variable.borrow_mut().replace(value);
        Ok(())
    }

    fn nargs(&self) -> Nargs {
        Nargs::Precisely(1)
    }
}

/// A parameter that takes multiple values (specifiable `Nargs`).
pub struct Collection<'ap, C, T>
where
    C: 'ap + Collectable<T>,
{
    variable: Rc<RefCell<&'ap mut C>>,
    nargs: Nargs,
    _phantom: PhantomData<T>,
}

impl<'ap, C, T> CliOption for Collection<'ap, C, T> where C: 'ap + Collectable<T> {}

impl<'ap, C, T> CliArgument for Collection<'ap, C, T> where C: 'ap + Collectable<T> {}

impl<'ap, C, T> Collection<'ap, C, T>
where
    C: 'ap + Collectable<T>,
{
    /// Create a collection parameter.
    pub fn new(variable: &'ap mut C, nargs: Nargs) -> Self {
        Self {
            variable: Rc::new(RefCell::new(variable)),
            nargs,
            _phantom: PhantomData,
        }
    }
}

impl<'ap, C, T> GenericCapturable<'ap, T> for Collection<'ap, C, T>
where
    T: FromStr,
    C: 'ap + Collectable<T>,
{
    fn matched(&mut self) {
        // Do nothing.
    }

    fn capture(&mut self, token: &str) -> Result<(), InvalidConversion> {
        let result: Result<T, InvalidConversion> =
            T::from_str(token).map_err(|_| InvalidConversion {
                token: token.to_string(),
                type_name: std::any::type_name::<T>(),
            });
        let value = result?;
        (**self.variable.borrow_mut()).add(value);
        Ok(())
    }

    fn nargs(&self) -> Nargs {
        self.nargs
    }
}

impl<T> Collectable<T> for Vec<T> {
    fn add(&mut self, item: T) {
        self.push(item);
    }
}

impl<T: Eq + std::hash::Hash> Collectable<T> for HashSet<T> {
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

    #[test]
    fn value_capture() {
        // Integer
        let mut variable: u32 = u32::default();
        let mut value = Scalar::new(&mut variable);
        value.capture("5").unwrap();
        assert_eq!(variable, 5);

        // Boolean
        let mut variable: bool = false;
        let mut value = Scalar::new(&mut variable);
        value.capture("true").unwrap();
        assert!(variable);
    }

    #[test]
    #[should_panic]
    fn switch_capture() {
        let mut variable: u32 = u32::default();
        let mut switch = Switch::new(&mut variable, 1);
        match switch.capture("5") {
            Ok(_) => {}
            Err(_) => {}
        };
    }

    #[test]
    fn optional_capture() {
        // Option<u32>
        let mut variable: Option<u32> = None;
        let mut optional = Optional::new(&mut variable);
        optional.capture("1").unwrap();
        assert_eq!(variable, Some(1));
    }

    #[test]
    fn collection_capture() {
        // Vec<u32>
        let mut variable: Vec<u32> = Vec::default();
        let mut collection = Collection::new(&mut variable, Nargs::Any);
        collection.capture("1").unwrap();
        collection.capture("0").unwrap();
        assert_eq!(variable, vec![1, 0]);

        // HashSet<u32>
        let mut variable: HashSet<u32> = HashSet::default();
        let mut collection = Collection::new(&mut variable, Nargs::Any);
        collection.capture("1").unwrap();
        collection.capture("0").unwrap();
        collection.capture("0").unwrap();
        assert_eq!(variable, HashSet::from([0, 1]));
    }

    #[test]
    fn value_overwritten() {
        let mut variable: u32 = u32::default();
        let mut value = Scalar::new(&mut variable);
        value.capture("5").unwrap();
        variable = 2;
        assert_eq!(variable, 2);
    }

    #[test]
    fn value_matched() {
        let mut variable: u32 = u32::default();
        let mut value = Scalar::new(&mut variable);
        value.matched();
        assert_eq!(variable, 0);
    }

    #[test]
    fn switch_matched() {
        let mut variable: u32 = u32::default();
        let mut switch = Switch::new(&mut variable, 2);
        switch.matched();
        assert_eq!(variable, 2);
    }

    #[test]
    fn optional_matched() {
        let mut variable: Option<u32> = None;
        let mut optional = Optional::new(&mut variable);
        optional.matched();
        assert_eq!(variable, None);
    }

    #[test]
    fn collection_matched() {
        let mut variable: Vec<u32> = Vec::default();
        let mut collection = Collection::new(&mut variable, Nargs::Any);
        collection.matched();
        assert_eq!(variable, vec![]);
    }

    #[test]
    fn test_nargs() {
        let mut variable: u32 = u32::default();
        let value = Scalar::new(&mut variable);
        assert_eq!(value.nargs(), Nargs::Precisely(1));

        let mut variable: u32 = u32::default();
        let switch = Switch::new(&mut variable, 2);
        assert_eq!(switch.nargs(), Nargs::Precisely(0));

        let mut variable: Option<u32> = None;
        let optional = Optional::new(&mut variable);
        assert_eq!(optional.nargs(), Nargs::Precisely(1));

        let mut variable: Vec<u32> = Vec::default();
        let collection = Collection::new(&mut variable, Nargs::Any);
        assert_eq!(collection.nargs(), Nargs::Any);

        let mut variable: Vec<u32> = Vec::default();
        let collection = Collection::new(&mut variable, Nargs::AtLeastOne);
        assert_eq!(collection.nargs(), Nargs::AtLeastOne);
    }
}
