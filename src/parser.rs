use std::env;
use std::str::FromStr;

use crate::field::*;

pub struct ArgumentParser<'ap> {
    name: &'ap str,
    // We need a (dyn .. [ignoring T] ..) here in order to put all the fields of varying types T under one collection.
    // In other words, we want the bottom of the object graph to include the types T, but up here we want to work across all T.
    options: Vec<Box<(dyn Capturable + 'ap)>>,
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
    fn ap_capture_value() {
        let ap = ArgumentParser::new("abc");
        let mut variable: u32 = u32::default();
        ap.add_option(Field::binding(Value::new(&mut variable)))
            .capture("7");
        assert_eq!(variable, 7);
    }

    #[test]
    fn ap_capture_option() {
        let ap = ArgumentParser::new("abc");
        let mut variable: Option<u32> = None;
        ap.add_option(Field::binding(Container::new(&mut variable)))
            .capture("7");
        assert_eq!(variable, Some(7));
    }

    #[test]
    fn ap_capture_vec() {
        let ap = ArgumentParser::new("abc");
        let mut variable: Vec<u32> = Vec::default();
        ap.add_option(Field::binding(Container::new(&mut variable)))
            .capture("7");
        assert_eq!(variable, vec![7]);
    }
}
