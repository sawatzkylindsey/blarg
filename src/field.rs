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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
pub trait GenericCapturable<'ap, T>
where
    T: FromStr + std::fmt::Debug,
    <T as FromStr>::Err: std::fmt::Debug,
{
    /// Declare that the parameter has been matched.
    fn matched(&mut self);

    /// Capture a value into the generic type T for this parameter.
    fn capture(&mut self, str_value: &str) -> Result<(), GenericCaptureError<T>>;

    /// Get the `Nargs` for this implementation.
    fn nargs(&self) -> Nargs;
}

#[derive(Debug, Error)]
pub enum GenericCaptureError<T>
where
    T: FromStr + std::fmt::Debug,
    <T as FromStr>::Err: std::fmt::Debug,
{
    #[error("Parse error during capture: {0:?}.")]
    FromStrError(<T as FromStr>::Err),

    #[error("The capture is prohibited.")]
    ProhibitedCapture,
}

/// Behaviour to capture an implicit generic type T from an input `&str`.
///
/// We use this at the middle/top of the argument parser object graph so that different types may all be 'captured' in a single argument parser.
pub trait AnonymousCapturable {
    /// Declare that the parameter has been matched.
    fn matched(&mut self);

    /// Capture a value anonymously for this parameter.
    fn capture(&mut self, value: &str) -> Result<(), AnonymousCaptureError>;
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AnonymousCaptureError {
    #[error("{0}")]
    FromStrError(String),

    #[error("The capture is prohibited.")]
    ProhibitedCapture,
}

impl<T> From<GenericCaptureError<T>> for AnonymousCaptureError
where
    T: FromStr + std::fmt::Debug,
    <T as FromStr>::Err: std::fmt::Debug,
{
    fn from(error: GenericCaptureError<T>) -> Self {
        match error {
            GenericCaptureError::FromStrError(_) => {
                AnonymousCaptureError::FromStrError(format!("{error:?}"))
            }
            GenericCaptureError::ProhibitedCapture => AnonymousCaptureError::ProhibitedCapture,
        }
    }
}

/// Describes an argument/option parameter that takes a single value `Nargs::Precisely(1)`.
pub struct Value<'ap, T: 'ap> {
    variable: Rc<RefCell<&'ap mut T>>,
}

impl<'ap, T: FromStr + std::fmt::Debug> Value<'ap, T> {
    pub fn new(variable: &'ap mut T) -> Self {
        Self {
            variable: Rc::new(RefCell::new(variable)),
        }
    }
}

impl<'ap, T> GenericCapturable<'ap, T> for Value<'ap, T>
where
    T: FromStr + std::fmt::Debug,
    <T as FromStr>::Err: std::fmt::Debug,
{
    fn matched(&mut self) {
        // Do nothing.
    }

    fn capture(&mut self, str_value: &str) -> Result<(), GenericCaptureError<T>> {
        let result: Result<T, GenericCaptureError<T>> =
            T::from_str(str_value).map_err(|e| GenericCaptureError::FromStrError(e));
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
pub struct Switch<'ap, T: 'ap> {
    variable: Rc<RefCell<&'ap mut T>>,
    target: Option<T>,
}

impl<'ap, T: FromStr + std::fmt::Debug> Switch<'ap, T> {
    pub fn new(variable: &'ap mut T, target: T) -> Self {
        Self {
            variable: Rc::new(RefCell::new(variable)),
            target: Some(target),
        }
    }
}

impl<'ap, T> GenericCapturable<'ap, T> for Switch<'ap, T>
where
    T: FromStr + std::fmt::Debug,
    <T as FromStr>::Err: std::fmt::Debug,
{
    fn matched(&mut self) {
        **self.variable.borrow_mut() = self
            .target
            .take()
            .expect("internal error - must be able to take the Switch#target");
    }

    fn capture(&mut self, _str_value: &str) -> Result<(), GenericCaptureError<T>> {
        Err(GenericCaptureError::ProhibitedCapture)
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

impl<'ap, C, T> GenericCapturable<'ap, T> for Container<'ap, C, T>
where
    C: 'ap + Collectable<T> + Nargable,
    T: FromStr + std::fmt::Debug,
    <T as FromStr>::Err: std::fmt::Debug,
{
    fn matched(&mut self) {
        // Do nothing.
    }

    fn capture(&mut self, str_value: &str) -> Result<(), GenericCaptureError<T>> {
        let result: Result<T, GenericCaptureError<T>> =
            T::from_str(str_value).map_err(|e| GenericCaptureError::FromStrError(e));
        let value = result?;
        (**self.variable.borrow_mut()).add(value);
        Ok(())
    }

    fn nargs(&self) -> Nargs {
        C::nargs()
    }
}

pub struct Field<'ap, T: 'ap> {
    pub(crate) nargs: Nargs,
    generic_capturable: Box<dyn GenericCapturable<'ap, T> + 'ap>,
}

impl<'ap, T> Field<'ap, T>
where
    T: FromStr + std::fmt::Debug,
    <T as FromStr>::Err: std::fmt::Debug,
{
    pub fn binding(generic_capturable: impl GenericCapturable<'ap, T> + 'ap) -> Self {
        Self {
            nargs: generic_capturable.nargs(),
            generic_capturable: Box::new(generic_capturable),
        }
    }

    /*pub fn help(self, message: &'static str) -> Self {
        Self {
            help: Some(message),
            ..self
        }
    }*/
}

impl<'ap, T> AnonymousCapturable for Field<'ap, T>
where
    T: FromStr + std::fmt::Debug,
    <T as FromStr>::Err: std::fmt::Debug,
{
    fn matched(&mut self) {
        self.generic_capturable.matched();
    }

    fn capture(&mut self, value: &str) -> Result<(), AnonymousCaptureError> {
        self.generic_capturable
            .capture(value)
            .map_err(AnonymousCaptureError::from)
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
    fn switch_capture() {
        let mut variable: u32 = u32::default();
        let mut switch = Switch::new(&mut variable, 1);
        assert!(matches!(
            switch.capture("5").unwrap_err(),
            GenericCaptureError::ProhibitedCapture
        ));
    }

    #[test]
    fn container_capture() {
        // Option<u32>
        let mut variable: Option<u32> = None;
        let mut container = Container::new(&mut variable);
        container.capture("1").unwrap();
        assert_eq!(variable, Some(1));

        // Vec<u32>
        let mut variable: Vec<u32> = Vec::default();
        let mut container = Container::new(&mut variable);
        container.capture("1").unwrap();
        container.capture("0").unwrap();
        assert_eq!(variable, vec![1, 0]);

        // HashSet<u32>
        let mut variable: HashSet<u32> = HashSet::default();
        let mut container = Container::new(&mut variable);
        container.capture("1").unwrap();
        container.capture("0").unwrap();
        container.capture("0").unwrap();
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
    fn container_matched() {
        let mut variable: Vec<u32> = Vec::default();
        let mut container = Container::new(&mut variable);
        container.matched();
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

        let mut variable: Vec<u32> = Vec::default();
        let container = Container::new(&mut variable);
        assert_eq!(container.nargs(), Nargs::Any);

        let mut variable: Option<u32> = None;
        let container = Container::new(&mut variable);
        assert_eq!(container.nargs(), Nargs::ZeroOrOne);
    }

    /*#[test]
    fn test_field() {
        let mut variable: u32 = u32::default();
        let mut value = Value::new(&mut variable);
        value.capture("5").unwrap();
        assert_eq!(variable, 5);

        let mut variable: u32 = u32::default();
        let mut field = Field::binding(Value::new(&mut variable));
        field.matched();
        field.capture("1").unwrap();
        assert_eq!(variable, 1);
    }*/
}
