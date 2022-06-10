use std::collections::VecDeque;
use std::env;
use std::str::FromStr;

use crate::field::*;
use crate::tokens::*;

pub struct ArgumentParser<'ap> {
    name: &'ap str,
    // We need a (dyn .. [ignoring T] ..) here in order to put all the fields of varying types T under one collection.
    // In other words, we want the bottom of the object graph to include the types T, but up here we want to work across all T.
    options: Vec<Box<(dyn Capturable + 'ap)>>,
    arguments: Vec<Box<(dyn Capturable + 'ap)>>,
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
            arguments: Vec::new(),
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

    pub fn add_argument<T>(mut self, field: Field<'ap, T>) -> Self
    where
        T: FromStr,
        <T as FromStr>::Err: std::fmt::Debug,
    {
        self.arguments.push(Box::new(field));
        self
    }

    pub fn capture(self, value: &str) {
        for mut box_capture in self.options {
            box_capture.capture(value).unwrap();
        }
    }

    fn token_parser(&self) -> TokenParser {
        todo!()
    }

    pub fn parse(self) {
        let mut token_parser = self.token_parser();

        for (i, next) in env::args().enumerate() {
            println!("{i}: {next}");
            token_parser.feed(&next).unwrap();
        }

        /*for (name, parts) in token_parser.matches() {
            println!("{name}: {parts:?}");
        }*/

        /*let mut inputs: VecDeque<String> = env::args().collect();
        //let mut argument_index = 0;
        //let mut buffer: Vec<String> = Vec::default();

        loop {
            if let Some(next) = args.pop_front() {
            } else {
                break;
            }
        }*/
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
