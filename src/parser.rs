use std::cell::RefCell;
use std::rc::Rc;
use std::str::FromStr;
use typed_builder::TypedBuilder;

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
        println!("{}", self.name);
        for mut box_capture in self.options {
            box_capture.capture(value).unwrap();
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
        field_reference.set(5);
        assert_eq!(variable, 5);
    }

    #[test]
    fn field_reference_from_str() {
        let mut variable: u32 = u32::default();
        let mut field_reference = FieldReference::new(&mut variable);
        field_reference.set_from("5");
        assert_eq!(variable, 5);
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
