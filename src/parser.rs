use std::env;
use std::cell::RefCell;
use std::collections::HashSet;
use std::marker::PhantomData;
use std::rc::Rc;
use std::str::FromStr;
use typed_builder::TypedBuilder;
use thiserror::Error;

pub struct FieldReference<'ap, T: 'ap + FromStr>
where
    <T as FromStr>::Err: std::fmt::Debug,
{
    variable: Rc<RefCell<&'ap mut T>>,
}

impl<'ap, T: FromStr> FieldReference<'ap, T>
where
    <T as FromStr>::Err: std::fmt::Debug,
{
    pub fn new(variable: &'ap mut T) -> Self {
        Self {
            variable: Rc::new(RefCell::new(variable)),
        }
    }

    pub fn set(&mut self, value: T) {
        **self.variable.borrow_mut() = value;
    }

    pub fn set_from(&mut self, str_value: &str) {
        self.set(T::from_str(str_value).unwrap())
    }
}

pub trait Collection<T> {
    fn add(&mut self, item: T) -> Result<(), ()>;
}

impl<T> Collection<T> for Vec<T> {
    fn add(&mut self, item: T) -> Result<(), ()> {
        self.push(item);
        Ok(())
    }
}

impl<T: std::cmp::Eq + std::hash::Hash> Collection<T> for HashSet<T> {
    fn add(&mut self, item: T) -> Result<(), ()> {
        self.insert(item);
        Ok(())
    }
}

pub struct FieldReferenceCollection<'ap, C, T>
where
    C: 'ap + Collection<T>,
    T: FromStr,
{
    variable: Rc<RefCell<&'ap mut C>>,
    _phantom: PhantomData<T>,
}

impl<'ap, C, T> FieldReferenceCollection<'ap, C, T>
where
    C: 'ap + Collection<T>,
    T: FromStr,
    <T as FromStr>::Err: std::fmt::Debug,
{
    pub fn new(variable: &'ap mut C) -> Self {
        Self {
            variable: Rc::new(RefCell::new(variable)),
            _phantom: PhantomData,
        }
    }

    pub fn add(&mut self, value: T) {
        (**self.variable.borrow_mut()).add(value);
    }

    pub fn add_from(&mut self, str_value: &str) -> Result<(), ParseError<T>> {
        let item = T::from_str(str_value)
            .map_err(|e| ParseError::FromStrError(e))?;
        self.add(item);
        Ok(())
    }
}

#[derive(Error, Debug)]
pub enum ParseError<T>
where
    T: FromStr,
    <T as FromStr>::Err: std::fmt::Debug,
{
    #[error("Encountered error while parsing from_str: {0:?}.")]
    FromStrError(<T as FromStr>::Err),
}

#[derive(TypedBuilder)]
pub struct Field<'ap, T: 'ap + FromStr>
where
    <T as FromStr>::Err: std::fmt::Debug,
{
    reference: FieldReference<'ap, T>,
}

impl<'ap, T: 'ap + FromStr> Capture for Field<'ap, T>
where
    <T as FromStr>::Err: std::fmt::Debug,
{
    fn capture(&mut self, value: &str) -> Result<(), ()> {
        self.reference.set_from(value);
        Ok(())
    }
}

pub trait Capture {
    fn capture(&mut self, value: &str) -> Result<(), ()>;
}

pub struct ArgumentParser<'ap> {
    name: &'ap str,
    options: Vec<Box<(dyn Capture + 'ap)>>,
}

impl<'ap> ArgumentParser<'ap> {
    pub fn new(name: &'ap str) -> Self {
        Self {
            name,
            options: Vec::new(),
        }
    }

    pub fn add_option<T: FromStr>(mut self, field: Field<'ap, T>) -> Self
    where
        <T as FromStr>::Err: std::fmt::Debug,
    {
        self.options.push(Box::new(field));
        self
    }

    pub fn capture(self, value: &str) {
        for mut box_capture in self.options {
            box_capture.capture(value).unwrap();
        }
    }

    /*pub fn parse(self) {
        for arg in env::args() {
            println!("{}", arg);
        }
    }*/
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_reference() {
        let mut variable: u32 = u32::default();
        let mut field_reference = FieldReference::new(&mut variable);
        field_reference.set(5);
        assert_eq!(variable, 5);
    }

    #[test]
    fn field_reference_from_str() {
        // Integer
        let mut variable: u32 = u32::default();
        let mut field_reference = FieldReference::new(&mut variable);
        field_reference.set_from("5");
        assert_eq!(variable, 5);

        // Boolean
        let mut variable: bool = false;
        let mut field_reference = FieldReference::new(&mut variable);
        field_reference.set_from("true");
        assert!(variable);

        // Vec<u32>
        let mut variable: Vec<u32> = Vec::default();
        let mut field_reference = FieldReferenceCollection::new(&mut variable);
        field_reference.add_from("1");
        field_reference.add_from("0");
        assert_eq!(variable, vec![1, 0]);

        // HashSet<u32>
        let mut variable: HashSet<u32> = HashSet::default();
        let mut field_reference = FieldReferenceCollection::new(&mut variable);
        field_reference.add_from("1");
        field_reference.add_from("0");
        field_reference.add_from("0");
        // It is driving me insane that `HashSet::from([0, 1])` isn't working here!
        assert_eq!(variable, vec![0, 1].into_iter().collect());
    }

    #[test]
    fn field_reference_overwritten() {
        let mut variable: u32 = u32::default();
        let mut field_reference = FieldReference::new(&mut variable);
        field_reference.set(5);
        variable = 2;
        assert_eq!(variable, 2);
    }

    #[test]
    fn ap_capture() {
        let ap = ArgumentParser::new("abc");
        let mut variable: u32 = u32::default();
        ap.add_option(
            Field::builder()
                .reference(FieldReference::new(&mut variable))
                .build(),
        )
        .capture("7");
        assert_eq!(variable, 7);
    }
}
