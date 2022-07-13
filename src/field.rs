use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;
use std::str::FromStr;
use thiserror::Error;

use crate::collection::*;

/// Describes the number of command inputs associated with the argument/option.
/// Inspired by argparse: <https://docs.python.org/3/library/argparse.html#nargs>
///
/// Notice, this isn't used directly on the user interface.
#[derive(Debug, Clone, Copy)]
#[doc(hidden)]
pub enum Nargs {
    /// N: Limited by precisely `N` values.
    Precisely(u8),
    /// ?: Must be either `1` value, or no values.
    ZeroOrOne,
    /// *: May be any number of values, including `0`.
    Any,
}

pub(crate) trait Nargable {
    fn nargs() -> Nargs;
}

/// Behaviour to capture an explicit generic type T from an input `&str`.
///
/// We use this at the bottom of the argument parser object graph so the compiler can maintain each field's type.
#[doc(hidden)]
pub trait TypeCapturable<'ap, T>
where
    T: FromStr,
    <T as FromStr>::Err: std::fmt::Debug,
{
    /// Declare that the parameter has been matched.
    fn matched_generic(&mut self);

    /// Capture a value into the generic type T for this parameter.
    fn capture_generic(&mut self, str_value: &str);

    /// Get the `Nargs` for this implementation.
    fn nargs(&self) -> Nargs;
}

/// Behaviour to capture an implicit generic type T from an input `&str`.
///
/// We use this at the middle/top of the argument parser object graph so that different types may all be 'captured' in a single argument parser.
pub trait Capturable {
    /// Declare that the parameter has been matched.
    fn matched_anonymous(&mut self);

    /// Capture a value anonymously for this parameter.
    fn capture_anonymous(&mut self, value: &str) -> Result<(), ()>;
}

/// Describes an argument/option parameter that takes a single value `Nargs::Precisely(1)`.
pub struct Value<'ap, T: 'ap> {
    variable: Rc<RefCell<&'ap mut T>>,
}

impl<'ap, T: FromStr> Value<'ap, T> {
    pub fn new(variable: &'ap mut T) -> Self {
        Self {
            variable: Rc::new(RefCell::new(variable)),
        }
    }
}

impl<'ap, T> TypeCapturable<'ap, T> for Value<'ap, T>
where
    T: FromStr,
    <T as FromStr>::Err: std::fmt::Debug,
{
    fn matched_generic(&mut self) {
        // Do nothing.
    }

    fn capture_generic(&mut self, str_value: &str) {
        let value = T::from_str(str_value).unwrap();
        **self.variable.borrow_mut() = value;
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
pub struct Switch<'ap, T: 'ap> {
    variable: Rc<RefCell<&'ap mut T>>,
    target: Option<T>,
}

impl<'ap, T: FromStr> Switch<'ap, T> {
    pub fn new(variable: &'ap mut T, target: T) -> Self {
        Self {
            variable: Rc::new(RefCell::new(variable)),
            target: Some(target),
        }
    }
}

impl<'ap, T> TypeCapturable<'ap, T> for Switch<'ap, T>
where
    T: FromStr,
    <T as FromStr>::Err: std::fmt::Debug,
{
    fn matched_generic(&mut self) {
        **self.variable.borrow_mut() = self.target.take().unwrap();
    }

    fn capture_generic(&mut self, _str_value: &str) {
        panic!("cannot capture on switch");
    }

    fn nargs(&self) -> Nargs {
        Nargs::Precisely(0)
    }
}

/// Describes an argument/option parameter that takes multiple values.
/// The exact `Nargs` is derived by the specific `Collectable` implementation.
pub struct Container<'ap, C, T>
where
    C: 'ap + Collectable<T>,
{
    variable: Rc<RefCell<&'ap mut C>>,
    _phantom: PhantomData<T>,
}

impl<'ap, C, T> Container<'ap, C, T>
where
    C: 'ap + Collectable<T>,
{
    pub fn new(variable: &'ap mut C) -> Self {
        Self {
            variable: Rc::new(RefCell::new(variable)),
            _phantom: PhantomData,
        }
    }
}

impl<'ap, C, T> TypeCapturable<'ap, T> for Container<'ap, C, T>
where
    C: 'ap + Collectable<T> + Nargable,
    T: FromStr,
    <T as FromStr>::Err: std::fmt::Debug,
{
    fn matched_generic(&mut self) {
        // Do nothing.
    }

    fn capture_generic(&mut self, str_value: &str) {
        let result: Result<T, ParseError<T>> =
            T::from_str(str_value).map_err(|e| ParseError::FromStrError(e));
        let value = result.unwrap();
        (**self.variable.borrow_mut()).add(value).unwrap();
    }

    fn nargs(&self) -> Nargs {
        C::nargs()
    }
}

#[derive(Error)]
pub enum ParseError<T>
where
    T: FromStr,
    <T as FromStr>::Err: std::fmt::Debug,
{
    #[error("Encountered error while parsing from_str: {0:?}.")]
    FromStrError(<T as FromStr>::Err),
}

// There is a bug/limitation with the #[derive(Debug)] for this case.
// So simply implement it ourselves.
impl<T> std::fmt::Debug for ParseError<T>
where
    T: FromStr,
    <T as FromStr>::Err: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub struct Field<'ap, T: 'ap> {
    pub(crate) nargs: Nargs,
    type_capturable: Box<dyn TypeCapturable<'ap, T> + 'ap>,
}

impl<'ap, T> Field<'ap, T>
where
    T: FromStr,
    <T as FromStr>::Err: std::fmt::Debug,
{
    pub fn binding(type_capturable: impl TypeCapturable<'ap, T> + 'ap) -> Self {
        Self {
            nargs: type_capturable.nargs(),
            type_capturable: Box::new(type_capturable),
        }
    }

    /*pub fn help(self, message: &'static str) -> Self {
        Self {
            help: Some(message),
            ..self
        }
    }*/
}

impl<'ap, T> Capturable for Field<'ap, T>
where
    T: FromStr,
    <T as FromStr>::Err: std::fmt::Debug,
{
    fn matched_anonymous(&mut self) {
        self.type_capturable.matched_generic();
    }

    fn capture_anonymous(&mut self, value: &str) -> Result<(), ()> {
        self.type_capturable.capture_generic(value);
        Ok(())
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
        value.capture_generic("5");
        assert_eq!(variable, 5);

        // Boolean
        let mut variable: bool = false;
        let mut value = Value::new(&mut variable);
        value.capture_generic("true");
        assert!(variable);
    }

    #[test]
    fn container_capture() {
        // Option<u32>
        let mut variable: Option<u32> = None;
        let mut container = Container::new(&mut variable);
        container.capture_generic("1");
        assert_eq!(variable, Some(1));

        // Vec<u32>
        let mut variable: Vec<u32> = Vec::default();
        let mut container = Container::new(&mut variable);
        container.capture_generic("1");
        container.capture_generic("0");
        assert_eq!(variable, vec![1, 0]);

        // HashSet<u32>
        let mut variable: HashSet<u32> = HashSet::default();
        let mut container = Container::new(&mut variable);
        container.capture_generic("1");
        container.capture_generic("0");
        container.capture_generic("0");
        assert_eq!(variable, HashSet::from([0, 1]));
    }

    #[test]
    fn value_overwritten() {
        let mut variable: u32 = u32::default();
        let mut value = Value::new(&mut variable);
        value.capture_generic("5");
        variable = 2;
        assert_eq!(variable, 2);
    }
}
