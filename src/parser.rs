use std::collections::{HashMap, HashSet, VecDeque};
use std::env;
use std::str::FromStr;

use crate::field::*;
use crate::tokens::*;

pub enum Parameter {
    Opt {
        name: &'static str,
        short: Option<char>,
        help: Option<&'static str>,
    },
    Arg {
        name: &'static str,
        help: Option<&'static str>,
    },
}

impl Parameter {
    pub fn option(name: &'static str, short: Option<char>) -> Self {
        Parameter::Opt {
            name,
            short,
            help: None,
        }
    }

    pub fn argument(name: &'static str) -> Self {
        Parameter::Arg { name, help: None }
    }

    pub fn help(self, message: &'static str) -> Self {
        match self {
            Parameter::Opt { name, short, .. } => Parameter::Opt {
                name,
                short,
                help: Some(message),
            },
            Parameter::Arg { name, .. } => Parameter::Arg {
                name,
                help: Some(message),
            },
        }
    }
}

pub struct ArgumentParser<'ap> {
    program: &'ap str,
    options: Vec<OptionCapture<'ap>>,
    arguments: Vec<ArgumentCapture<'ap>>,
}

impl<'ap> std::fmt::Debug for ArgumentParser<'ap> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ArgumentParser")
            .field("program", &self.program)
            .finish()
    }
}

impl<'ap> ArgumentParser<'ap> {
    pub fn new(program: &'ap str) -> Self {
        Self {
            program,
            options: Vec::new(),
            arguments: Vec::new(),
        }
    }

    pub fn add<T>(mut self, parameter: Parameter, field: Field<'ap, T>) -> Self
    where
        T: FromStr,
        <T as FromStr>::Err: std::fmt::Debug,
    {
        match parameter {
            Parameter::Opt { name, short, .. } => {
                // Derive the bound from nargs, in the context of an option parameter.
                let bound = match field.nargs {
                    Nargs::Precisely(n) => Bound::Range(n, n),
                    Nargs::ZeroOrOne => Bound::Range(0, 1),
                    Nargs::Any => Bound::Lower(0),
                };
                self.options.push((
                    OptionConfig::new(name.to_string(), short, bound),
                    Box::new(field),
                ));
            }
            Parameter::Arg { name, .. } => {
                // Derive the bound from nargs, in the context of an argument parameter.
                let bound = match field.nargs {
                    Nargs::Precisely(n) => Bound::Range(n, n),
                    Nargs::ZeroOrOne => Bound::Range(1, 1),
                    Nargs::Any => Bound::Lower(1),
                };
                self.arguments.push((
                    ArgumentConfig::new(name.to_string(), bound).unwrap(),
                    Box::new(field),
                ));
            }
        };
        self
    }

    pub fn parse_tokens(self, tokens: &[&str]) {
        println!("{tokens:?}");
        ParseCapture::new(self.options, self.arguments)
            .consume(tokens)
            .unwrap();
    }

    pub fn parse(self) {
        let command_input: Vec<String> = env::args().skip(1).collect();
        self.parse_tokens(
            command_input
                .iter()
                .map(AsRef::as_ref)
                .collect::<Vec<&str>>()
                .as_slice(),
        );
    }
}

struct ParseCapture<'ap> {
    token_matcher: TokenMatcher,
    captures: HashMap<String, Box<(dyn Capturable + 'ap)>>,
}

// We need a (dyn .. [ignoring T] ..) here in order to put all the fields of varying types T under one collection.
// In other words, we want the bottom of the object graph to include the types T, but up here we want to work across all T.
type OptionCapture<'ap> = (OptionConfig, Box<(dyn Capturable + 'ap)>);
type ArgumentCapture<'ap> = (ArgumentConfig, Box<(dyn Capturable + 'ap)>);

impl<'ap> ParseCapture<'ap> {
    fn new(options: Vec<OptionCapture<'ap>>, arguments: Vec<ArgumentCapture<'ap>>) -> Self {
        let mut option_configs = HashSet::default();
        let mut argument_configs = VecDeque::default();
        let mut captures: HashMap<String, Box<(dyn Capturable + 'ap)>> = HashMap::default();

        for (oc, f) in options.into_iter() {
            assert!(captures.insert(oc.name(), f).is_none());
            option_configs.insert(oc);
        }

        for (ac, f) in arguments.into_iter() {
            assert!(captures.insert(ac.name(), f).is_none());
            argument_configs.push_back(ac);
        }

        Self {
            token_matcher: TokenMatcher::new(option_configs, argument_configs),
            captures,
        }
    }

    fn consume(mut self, tokens: &[&str]) -> Result<(), ()> {
        // 1. Feed the raw token strings to the matcher.
        for next in tokens {
            self.token_matcher.feed(next).unwrap();
        }

        // 2. Get the matching between tokens-parameter/options, still as raw strings.
        for match_tokens in self.token_matcher.matches().unwrap() {
            // 3. Find the corresponding capture.
            let mut box_capture = self.captures.remove(&match_tokens.name).unwrap();
            // 4. Let the capture know it has been matched.
            // Some captures may do something based off the fact they were simply matched.
            box_capture.matched_anonymous();

            // 5. Convert each of the raw value strings into the capture type.
            for value in &match_tokens.values {
                box_capture.capture_anonymous(value).unwrap();
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(vec!["--variable", "1"])]
    #[case(vec!["--variable", "01"])]
    #[case(vec!["-v", "1"])]
    #[case(vec!["-v", "01"])]
    #[case(vec!["-v=1"])]
    #[case(vec!["-v=01"])]
    fn ap_option_value(#[case] tokens: Vec<&str>) {
        let ap = ArgumentParser::new("abc");
        let mut variable: u32 = 0;
        ap.add(
            Parameter::option("variable", Some('v')),
            Field::binding(Value::new(&mut variable)),
        )
        .parse_tokens(tokens.as_slice());
        assert_eq!(variable, 1);
    }

    #[rstest]
    #[case(vec!["--variable"])]
    #[case(vec!["-v"])]
    fn ap_option_switch(#[case] tokens: Vec<&str>) {
        let ap = ArgumentParser::new("abc");
        let mut variable: u32 = 0;
        ap.add(
            Parameter::option("variable", Some('v')),
            Field::binding(Switch::new(&mut variable, 2)),
        )
        .parse_tokens(tokens.as_slice());
        assert_eq!(variable, 2);
    }

    #[rstest]
    #[case(vec!["--variable", "1"], vec![1])]
    #[case(vec!["--variable", "1", "3", "2", "1"], vec![1, 3, 2, 1])]
    #[case(vec!["--variable", "01"], vec![1])]
    #[case(vec!["-v", "1"], vec![1])]
    #[case(vec!["-v", "1", "3", "2", "1"], vec![1, 3, 2, 1])]
    #[case(vec!["-v=01"], vec![1])]
    #[case(vec!["-v=1"], vec![1])]
    #[case(vec!["-v=01"], vec![1])]
    fn ap_option_container(#[case] tokens: Vec<&str>, #[case] expected: Vec<u32>) {
        let ap = ArgumentParser::new("abc");
        let mut variable: Vec<u32> = Vec::default();
        ap.add(
            Parameter::option("variable", Some('v')),
            Field::binding(Container::new(&mut variable)),
        )
        .parse_tokens(tokens.as_slice());
        assert_eq!(variable, expected);
    }

    #[test]
    fn ap_argument_value() {
        let ap = ArgumentParser::new("abc");
        let mut variable: u32 = 0;
        ap.add(
            Parameter::argument("variable"),
            Field::binding(Value::new(&mut variable)),
        )
        .parse_tokens(vec!["1"].as_slice());
        assert_eq!(variable, 1);
    }

    #[rstest]
    #[case(vec!["1"], vec![1])]
    #[case(vec!["1", "3", "2", "1"], vec![1, 3, 2, 1])]
    #[case(vec!["01"], vec![1])]
    fn ap_argument_container(#[case] tokens: Vec<&str>, #[case] expected: Vec<u32>) {
        let ap = ArgumentParser::new("abc");
        let mut variable: Vec<u32> = Vec::default();
        ap.add(
            Parameter::argument("variable"),
            Field::binding(Container::new(&mut variable)),
        )
        .parse_tokens(&tokens[..]);
        assert_eq!(variable, expected);
    }

    #[test]
    fn parse_capture_empty() {
        let options = vec![];
        let arguments = vec![];
        let pc = ParseCapture::new(options, arguments);
        pc.consume(empty::slice()).unwrap();
    }
}
