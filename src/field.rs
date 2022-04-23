use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;
use std::str::FromStr;
use thiserror::Error;

use crate::collection::*;

/// Behaviour to capture an explicit generic type T from an input `&str`.
///
/// We use this at the bottom of the argument parser object graph so the compiler can maintain each field's type.
pub trait TypeCapturable<'ap, T>
where
    T: FromStr,
    <T as FromStr>::Err: std::fmt::Debug,
{
    fn capture_from(&mut self, str_value: &str);
}

/// Behaviour to capture an implicit generic type T from an input `&str`.
///
/// We use this at the middle/top of the argument parser object graph so that different types may all be 'captured' in a single argument parser.
pub trait Capturable {
    fn capture(&mut self, value: &str) -> Result<(), ()>;
}

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
    fn capture_from(&mut self, str_value: &str) {
        let value = T::from_str(str_value).unwrap();
        **self.variable.borrow_mut() = value;
    }
}

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
    C: 'ap + Collectable<T>,
    T: FromStr,
    <T as FromStr>::Err: std::fmt::Debug,
{
    fn capture_from(&mut self, str_value: &str) {
        let result: Result<T, ParseError<T>> =
            T::from_str(str_value).map_err(|e| ParseError::FromStrError(e));
        let value = result.unwrap();
        (**self.variable.borrow_mut()).add(value).unwrap();
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
    type_capturable: Box<dyn TypeCapturable<'ap, T> + 'ap>,
}

impl<'ap, T> Field<'ap, T>
where
    T: FromStr,
    <T as FromStr>::Err: std::fmt::Debug,
{
    pub fn binding(type_capturable: impl TypeCapturable<'ap, T> + 'ap) -> Self {
        Self {
            type_capturable: Box::new(type_capturable),
        }
    }
}

impl<'ap, T> Capturable for Field<'ap, T>
where
    T: FromStr,
    <T as FromStr>::Err: std::fmt::Debug,
{
    fn capture(&mut self, value: &str) -> Result<(), ()> {
        self.type_capturable.capture_from(value);
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
        value.capture_from("5");
        assert_eq!(variable, 5);

        // Boolean
        let mut variable: bool = false;
        let mut value = Value::new(&mut variable);
        value.capture_from("true");
        assert!(variable);
    }

    #[test]
    fn container_capture() {
        // Option<u32>
        let mut variable: Option<u32> = None;
        let mut container = Container::new(&mut variable);
        container.capture_from("1");
        assert_eq!(variable, Some(1));

        // Vec<u32>
        let mut variable: Vec<u32> = Vec::default();
        let mut container = Container::new(&mut variable);
        container.capture_from("1");
        container.capture_from("0");
        assert_eq!(variable, vec![1, 0]);

        // HashSet<u32>
        let mut variable: HashSet<u32> = HashSet::default();
        let mut container = Container::new(&mut variable);
        container.capture_from("1");
        container.capture_from("0");
        container.capture_from("0");
        assert_eq!(variable, HashSet::from([0, 1]));
    }

    #[test]
    fn value_overwritten() {
        let mut variable: u32 = u32::default();
        let mut value = Value::new(&mut variable);
        value.capture_from("5");
        variable = 2;
        assert_eq!(variable, 2);
    }
}
