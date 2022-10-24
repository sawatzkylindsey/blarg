use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;
use std::str::FromStr;
use thiserror::Error;

use crate::collection::*;

/// Describes the number of command inputs associated with the argument/option.
/// Inspired by argparse: <https://docs.python.org/3/library/argparse.html#nargs>
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Nargs {
    /// N: Limited by precisely `N` values.
    Precisely(u8),
    /// *: May be any number of values, including `0`.
    Any,
    /// +: At least one value must be specified.
    AtLeastOne,
}

/// Marker trait for capturable types that can formulate an option in the CLI
pub trait CliOption {}

/// Marker trait for capturable types that can formulate an argument in the CLI.
pub trait CliArgument {}

/// Behaviour to capture an explicit generic type T from an input `&str`.
///
/// We use this at the bottom of the argument parser object graph so the compiler can maintain each field's type.
#[doc(hidden)]
pub trait GenericCapturable<'ap, T> {
    /// Declare that the parameter has been matched.
    fn matched(&mut self);

    /// Capture a value into the generic type T for this parameter.
    fn capture(&mut self, token: &str) -> Result<(), InvalidConversion>;

    /// Get the `Nargs` for this implementation.
    fn nargs(&self) -> Nargs;
}

#[derive(Debug, Error)]
#[doc(hidden)]
#[error("'{token}' cannot convert to {type_name}.")]
pub struct InvalidConversion {
    token: String,
    type_name: &'static str,
}

/// Behaviour to capture an implicit generic type T from an input `&str`.
///
/// We use this at the middle/top of the argument parser object graph so that different types may all be 'captured' in a single argument parser.
pub(crate) trait AnonymousCapturable {
    /// Declare that the parameter has been matched.
    fn matched(&mut self);

    /// Capture a value anonymously for this parameter.
    fn capture(&mut self, value: &str) -> Result<(), InvalidConversion>;
}

/// Describes an argument/option parameter that takes a single value `Nargs::Precisely(1)`.
pub struct Value<'ap, T> {
    variable: Rc<RefCell<&'ap mut T>>,
}

impl<'ap, T> CliOption for Value<'ap, T> {}
impl<'ap, T> CliArgument for Value<'ap, T> {}

impl<'ap, T> Value<'ap, T> {
    pub fn new(variable: &'ap mut T) -> Self {
        Self {
            variable: Rc::new(RefCell::new(variable)),
        }
    }
}

impl<'ap, T> GenericCapturable<'ap, T> for Value<'ap, T>
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

/// Describes an option parameter that takes no values `Nargs::Precisely(0)`.
/// This can never be used as an argument parameter, since by definition arguments take at least one value.
///
/// Notice, once Rust allows for 'specialization', we can actually implement this on `Collectable`:
/// <https://doc.rust-lang.org/unstable-book/language-features/specialization.html>
///
/// ```ignore
/// // Implement switch behaviour via specialization - aka: polymorphism.
/// impl<bool> Collectable<bool> for Option<bool> {
/// ```
pub struct Switch<'ap, T> {
    variable: Rc<RefCell<&'ap mut T>>,
    target: Option<T>,
}

impl<'ap, T> CliOption for Switch<'ap, T> {}

impl<'ap, T> Switch<'ap, T> {
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
        panic!("internal error - must not capture on a Switch");
    }

    fn nargs(&self) -> Nargs {
        Nargs::Precisely(0)
    }
}

/// Describes an option parameter that maps down to `Option`, taking a single value `Nargs::Precisely(1)`.
/// This can never be used as an argument parameter by "blarg" paradigm.
/// That is, if the value is an `Option`, then it must not be an argument - it is by definition optional
pub struct Optional<'ap, T> {
    variable: Rc<RefCell<&'ap mut Option<T>>>,
}

impl<'ap, T> CliOption for Optional<'ap, T> {}

impl<'ap, T> Optional<'ap, T> {
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

/// Describes an argument/option parameter that takes multiple values.
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

pub struct Field<'ap, T: 'ap> {
    generic_capturable: Box<dyn GenericCapturable<'ap, T> + 'ap>,
}

impl<'ap, T> Field<'ap, T> {
    pub(crate) fn binding(generic_capturable: impl GenericCapturable<'ap, T> + 'ap) -> Self {
        Self {
            generic_capturable: Box::new(generic_capturable),
        }
    }
}

impl<'ap, T> AnonymousCapturable for Field<'ap, T> {
    fn matched(&mut self) {
        self.generic_capturable.matched();
    }

    fn capture(&mut self, value: &str) -> Result<(), InvalidConversion> {
        self.generic_capturable.capture(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn value_capture() {
        // Integer
        let mut variable: u32 = u32::default();
        let mut value = Value::new(&mut variable);
        value.capture("5").unwrap();
        assert_eq!(variable, 5);

        // Boolean
        let mut variable: bool = false;
        let mut value = Value::new(&mut variable);
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
        let mut value = Value::new(&mut variable);
        value.capture("5").unwrap();
        variable = 2;
        assert_eq!(variable, 2);
    }

    #[test]
    fn value_matched() {
        let mut variable: u32 = u32::default();
        let mut value = Value::new(&mut variable);
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
        let value = Value::new(&mut variable);
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
