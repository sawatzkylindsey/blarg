use std::cell::RefCell;
use std::collections::HashSet;
use std::env;
use std::marker::PhantomData;
use std::rc::Rc;
use std::str::FromStr;
use thiserror::Error;

pub struct FieldReference<'ap, T: 'ap>
//where
//    <T as FromStr>::Err: std::fmt::Debug,
{
    variable: Rc<RefCell<&'ap mut T>>,
}

impl<'ap, T: FromStr> FieldReference<'ap, T>
//where
//    <T as FromStr>::Err: std::fmt::Debug,
{
    pub fn new(variable: &'ap mut T) -> Self {
        Self {
            variable: Rc::new(RefCell::new(variable)),
        }
    }
}

impl<'ap, T> TypeCapture<'ap, T> for FieldReference<'ap, T>
where
    T: FromStr,
    <T as FromStr>::Err: std::fmt::Debug,
{
    fn capture(&mut self, value: T) {
        **self.variable.borrow_mut() = value;
    }

    fn capture_from(&mut self, str_value: &str) {
        self.capture(T::from_str(str_value).unwrap())
    }
}

/*pub struct FieldReferenceOption<'ap, T: 'ap + FromStr>
//where
//    <T as FromStr>::Err: std::fmt::Debug,
{
    variable: Rc<RefCell<&'ap mut Option<T>>>,
}

impl<'ap, T: FromStr> FieldReferenceOption<'ap, T>
//where
//    <T as FromStr>::Err: std::fmt::Debug,
{
    pub fn new(variable: &'ap mut Option<T>) -> Self {
        Self {
            variable: Rc::new(RefCell::new(variable)),
        }
    }
}

impl<'ap, T> TypeCapture<'ap, Option<T>> for FieldReferenceOption<'ap, T>
//where
//    T: FromStr,
//    <T as FromStr>::Err: std::fmt::Debug,
{
    fn capture(&mut self, value: Option<T>) {
        **self.variable.borrow_mut() = value;
    }

    fn capture_from(&mut self, str_value: &str) {
        self.capture(Some(T::from_str(str_value).unwrap()))
    }
}

type OptionT<T> = Option<T>;
*/
/*
impl<'ap, T> TypeCapture<'ap, T> for Option<T>
where
    T: FromStr,
    <T as FromStr>::Err: std::fmt::Debug,
{
    fn capture(&mut self, value: T) {
        std::mem::replace(self, value);
    }

    fn capture_from(&mut self, str_value: &str) {
        self.capture(Some(T::from_str(str_value).unwrap()))
    }
}*/

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

impl<T> Collection<T> for Option<T> {
    fn add(&mut self, item: T) -> Result<(), ()> {
        if self.is_none() {
            self.replace(item);
            Ok(())
        } else {
            Err(())
        }
    }
}

/*struct Optional<T> {
    items: Vec<T>,
}*/

/*impl<T> Collection<T> for Option<T> {
    fn add(&mut self, item: T) -> Result<(), ()> {
        let mut value = Some(item);
        std::mem::replace(&mut self, &mut value);
        Ok(())
        /*if !self.items.is_empty() {
            Err(())
        } else {
            self.items.add(item);
            Ok(())
        }*/
    }
}*/

pub struct FieldReferenceCollection<'ap, C, T>
where
    C: 'ap + Collection<T>,
//    T: FromStr,
{
    variable: Rc<RefCell<&'ap mut C>>,
    _phantom: PhantomData<T>,
}

impl<'ap, C, T> FieldReferenceCollection<'ap, C, T>
where
    C: 'ap + Collection<T>,
//    T: FromStr,
//    <T as FromStr>::Err: std::fmt::Debug,
{
    pub fn new(variable: &'ap mut C) -> Self {
        Self {
            variable: Rc::new(RefCell::new(variable)),
            _phantom: PhantomData,
        }
    }
}

impl<'ap, C, T> TypeCapture<'ap, T> for FieldReferenceCollection<'ap, C, T>
where
    C: 'ap + Collection<T>,
    T: FromStr,
    <T as FromStr>::Err: std::fmt::Debug,
{
    fn capture(&mut self, value: T) {
        (**self.variable.borrow_mut()).add(value).unwrap();
    }

    fn capture_from(&mut self, str_value: &str) {
        let result: Result<T, ParseError<T>> =
            T::from_str(str_value).map_err(|e| ParseError::FromStrError(e));
        let item = result.unwrap();
        self.capture(item);
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

pub struct Field<'ap, T: 'ap>
/*where
    T: FromStr,
    <T as FromStr>::Err: std::fmt::Debug,*/
{
    reference: Box<dyn TypeCapture<'ap, T> + 'ap>,
}

impl<'ap, T> Field<'ap, T>
where
    T: FromStr,
    <T as FromStr>::Err: std::fmt::Debug,
{
    pub fn any(reference: impl TypeCapture<'ap, T> + 'ap) -> Self {
        Self {
            reference: Box::new(reference),
        }
    }
}

impl<'ap, T> Field<'ap, T>
where
    T: FromStr,
    <T as FromStr>::Err: std::fmt::Debug,
{
    pub fn scalar(reference: FieldReference<'ap, T>) -> Self {
        Self {
            reference: Box::new(reference),
        }
    }
}

impl<'ap, T> Field<'ap, T>
where
    T: FromStr,
    <T as FromStr>::Err: std::fmt::Debug,
{
    pub fn collection<C: 'ap + Collection<T>>(
        reference: FieldReferenceCollection<'ap, C, T>,
    ) -> Self {
        Self {
            reference: Box::new(reference),
        }
    }
}

impl<'ap, T> Capture for Field<'ap, T>
where
    T: FromStr,
    <T as FromStr>::Err: std::fmt::Debug,
{
    fn capture(&mut self, value: &str) -> Result<(), ()> {
        self.reference.capture_from(value);
        Ok(())
    }
}

pub trait TypeCapture<'ap, T>
where
    T: FromStr,
    <T as FromStr>::Err: std::fmt::Debug,
{
    fn capture(&mut self, value: T);
    fn capture_from(&mut self, str_value: &str);
}

pub trait Capture {
    fn capture(&mut self, value: &str) -> Result<(), ()>;
}

pub struct ArgumentParser<'ap> {
    name: &'ap str,
    // We need a (dyn .. [ignoring T] ..) here in order to put all the fields of varying types T under one collection.
    // In other words, we want the bottom of the object graph to include the types T, but up here we want to work across all T.
    options: Vec<Box<(dyn Capture + 'ap)>>,
}

impl<'ap> std::fmt::Debug for ArgumentParser<'ap> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ArgumentParser")
            .field("name", &self.name)
            .finish()
    }
}

impl<'ap> ArgumentParser<'ap> {
    pub fn new(name: &'ap str) -> Self {
        Self {
            name,
            options: Vec::new(),
        }
    }

    pub fn add_option<T>(mut self, field: Field<'ap, T>) -> Self
    where
        T: FromStr,
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

    pub fn parse(self) {
        for arg in env::args() {
            println!("{}", arg);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_reference() {
        let mut variable: u32 = u32::default();
        let mut field_reference = FieldReference::new(&mut variable);
        field_reference.capture(5);
        assert_eq!(variable, 5);
    }

    #[test]
    fn field_reference_from_str() {
        // Integer
        let mut variable: u32 = u32::default();
        let mut field_reference = FieldReference::new(&mut variable);
        field_reference.capture_from("5");
        assert_eq!(variable, 5);

        // Boolean
        let mut variable: bool = false;
        let mut field_reference = FieldReference::new(&mut variable);
        field_reference.capture_from("true");
        assert!(variable);

        // Vec<u32>
        let mut variable: Vec<u32> = Vec::default();
        let mut field_reference = FieldReferenceCollection::new(&mut variable);
        field_reference.capture_from("1");
        field_reference.capture_from("0");
        assert_eq!(variable, vec![1, 0]);

        // HashSet<u32>
        let mut variable: HashSet<u32> = HashSet::default();
        let mut field_reference = FieldReferenceCollection::new(&mut variable);
        field_reference.capture_from("1");
        field_reference.capture_from("0");
        field_reference.capture_from("0");
        // It is driving me insane that `HashSet::from([0, 1])` isn't working here!
        assert_eq!(variable, vec![0, 1].into_iter().collect());
    }

    #[test]
    fn field_reference_overwritten() {
        let mut variable: u32 = u32::default();
        let mut field_reference = FieldReference::new(&mut variable);
        field_reference.capture(5);
        variable = 2;
        assert_eq!(variable, 2);
    }

    #[test]
    fn ap_capture_scalar() {
        let ap = ArgumentParser::new("abc");
        let mut variable: u32 = u32::default();
        ap.add_option(Field::any(FieldReference::new(&mut variable)))
            .capture("7");
        assert_eq!(variable, 7);
    }

    #[test]
    fn ap_capture_option() {
        let ap = ArgumentParser::new("abc");
        let mut variable: Option<u32> = None;
        ap.add_option(Field::any(FieldReferenceCollection::new(&mut variable)))
            .capture("7");
        assert_eq!(variable, Some(7));
    }

    #[test]
    fn ap_capture_collection() {
        let ap = ArgumentParser::new("abc");
        let mut variable: Vec<u32> = Vec::default();
        ap.add_option(Field::any(FieldReferenceCollection::new(
            &mut variable,
        )))
        .capture("7");
        assert_eq!(variable, vec![7]);
    }
}
