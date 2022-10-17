use std::collections::{HashMap, HashSet, VecDeque};
use std::env;
use std::str::FromStr;
use thiserror::Error;

use crate::field::*;
use crate::tokens::*;
use crate::ui::*;

#[derive(Debug, Error)]
#[error("Config error: {0}")]
pub struct ConfigError(String);

impl From<TokenMatcherError> for ConfigError {
    fn from(error: TokenMatcherError) -> Self {
        match error {
            TokenMatcherError::DuplicateOption(_) => {
                panic!("internal error - invalid option should have been caught")
            }
            TokenMatcherError::DuplicateShortOption(_) => ConfigError(error.to_string()),
        }
    }
}

#[derive(Debug, Error)]
#[error("Parse error: {0}")]
pub(crate) struct ParseError(String);

impl From<MatchError> for ParseError {
    fn from(error: MatchError) -> Self {
        ParseError(error.to_string())
    }
}

impl From<AnonymousCaptureError> for ParseError {
    fn from(error: AnonymousCaptureError) -> Self {
        ParseError(error.to_string())
    }
}

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
            options: Vec::default(),
            arguments: Vec::default(),
        }
    }

    pub fn add<T>(mut self, parameter: Parameter, field: Field<'ap, T>) -> Self
    where
        T: FromStr + std::fmt::Debug,
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
                    ArgumentConfig::new(name.to_string(), bound)
                        .expect("internal error - argument construction must be valid"),
                    Box::new(field),
                ));
            }
        };
        self
    }

    pub fn build(self) -> Result<Parser<'ap>, ConfigError> {
        let parse_capture = ParseCapture::new(self.options, self.arguments)?;
        Ok(Parser {
            parse_capture,
            user_interface: Box::new(Console::default()),
        })
    }
}


pub struct Parser<'ap> {
    parse_capture: ParseCapture<'ap>,
    user_interface: Box<dyn UserInterface>,
}

impl<'ap> Parser<'ap> {
    fn parse_tokens(self, tokens: &[&str]) -> Result<(), ()> {
        match self.parse_capture.consume(tokens) {
            Err((offset, parse_error)) => {
                //println!("{i}");
                self.user_interface.print_error(parse_error);
                self.user_interface.print_context(tokens, offset);
                Err(())
            }
            Ok(()) => Ok(()),
        }
    }

    pub fn parse(self) {
        let command_input: Vec<String> = env::args().skip(1).collect();
        let result = self.parse_tokens(
            command_input
                .iter()
                .map(AsRef::as_ref)
                .collect::<Vec<&str>>()
                .as_slice(),
        );

        if let Err(_) = result {
            std::process::exit(1);
        }
    }
}

struct ParseCapture<'ap> {
    token_matcher: TokenMatcher,
    captures: HashMap<String, Box<(dyn AnonymousCapturable + 'ap)>>,
}

// We need a (dyn .. [ignoring T] ..) here in order to put all the fields of varying types T under one collection.
// In other words, we want the bottom of the object graph to include the types T, but up here we want to work across all T.
type OptionCapture<'ap> = (OptionConfig, Box<(dyn AnonymousCapturable + 'ap)>);
type ArgumentCapture<'ap> = (ArgumentConfig, Box<(dyn AnonymousCapturable + 'ap)>);
const HELP_NAME: &'static str = "help";
const HELP_SHORT: char = 'h';

impl<'ap> ParseCapture<'ap> {
    fn new(
        options: Vec<OptionCapture<'ap>>,
        arguments: Vec<ArgumentCapture<'ap>>,
    ) -> Result<Self, ConfigError> {
        let help_config =
            OptionConfig::new(HELP_NAME.to_string(), Some(HELP_SHORT), Bound::Range(0, 1));
        let mut option_configs = HashSet::from([help_config]);
        let mut argument_configs = VecDeque::default();
        let mut captures: HashMap<String, Box<(dyn AnonymousCapturable + 'ap)>> =
            HashMap::default();

        for (oc, f) in options.into_iter() {
            if captures.insert(oc.name(), f).is_some() {
                return Err(ConfigError(format!(
                    "Cannot duplicate the parameter '{0}'.",
                    oc.name()
                )));
            }

            option_configs.insert(oc);
        }

        for (ac, f) in arguments.into_iter() {
            if captures.insert(ac.name(), f).is_some() {
                return Err(ConfigError(format!(
                    "Cannot duplicate the parameter '{0}'.",
                    ac.name()
                )));
            }

            argument_configs.push_back(ac);
        }

        let token_matcher = TokenMatcher::new(option_configs, argument_configs)?;

        Ok(Self {
            token_matcher,
            captures,
        })
    }

    fn consume(mut self, tokens: &[&str]) -> Result<(), (usize, ParseError)> {
        // 1. Feed the raw token strings to the matcher.
        for (i, next) in tokens.iter().enumerate() {
            self.token_matcher.feed(next).map_err(|e| (i, ParseError::from(e)))?;
        }

        let matches = match self.token_matcher.close() {
            Ok(matches) | Err((_, MatchError::ArgumentsUnused, matches))
                if matches.contains(HELP_NAME) =>
            {
                return Ok(());
            }
            Ok(matches) => Ok(matches),
            Err((offset, e, _)) => Err((offset, ParseError::from(e))),
        }?;

        // 2. Get the matching between tokens-parameter/options, still as raw strings.
        for match_tokens in matches.values {
            // 3. Find the corresponding capture.
            let mut box_capture = self
                .captures
                .remove(&match_tokens.name)
                .expect("internal error - mismatch between matches and captures");
            // 4. Let the capture know it has been matched.
            // Some captures may do something based off the fact they were simply matched.
            box_capture.matched();

            // 5. Convert each of the raw value strings into the capture type.
            for (offset, value) in &match_tokens.values {
                box_capture.capture(value).map_err(|e| (*offset, ParseError::from(e)))?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[test]
    fn ap_empty() {
        let ap = ArgumentParser::new("abc");
        ap.build().unwrap().parse_tokens(empty::slice()).unwrap();
    }

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
        ).build().unwrap()
        .parse_tokens(tokens.as_slice())
        .unwrap();
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
        ).build().unwrap()
        .parse_tokens(tokens.as_slice())
        .unwrap();
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
        ).build().unwrap()
        .parse_tokens(tokens.as_slice())
        .unwrap();
        assert_eq!(variable, expected);
    }

    #[test]
    fn ap_argument_value() {
        let ap = ArgumentParser::new("abc");
        let mut variable: u32 = 0;
        ap.add(
            Parameter::argument("variable"),
            Field::binding(Value::new(&mut variable)),
        ).build().unwrap()
        .parse_tokens(vec!["1"].as_slice())
        .unwrap();
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
        ).build().unwrap()
        .parse_tokens(&tokens[..])
        .unwrap();
        assert_eq!(variable, expected);
    }

    #[rstest]
    #[case(vec!["--help"])]
    #[case(vec!["-h"])]
    #[case(vec!["--help", "1"])]
    #[case(vec!["-h", "1"])]
    #[case(vec!["--help", "not-a-u32"])]
    #[case(vec!["-h", "not-a-u32"])]
    fn ap_help(#[case] tokens: Vec<&str>) {
        let ap = ArgumentParser::new("abc");
        let mut variable: u32 = 0;
        ap.add(
            Parameter::argument("variable"),
            Field::binding(Value::new(&mut variable)),
        )
        .build()
        .unwrap()
        .parse_tokens(tokens.as_slice())
        .unwrap();
        assert_eq!(variable, 0);
    }
}
