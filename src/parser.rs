use std::collections::{HashMap, HashSet, VecDeque};
use std::env;
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

impl From<InvalidConversion> for ParseError {
    fn from(error: InvalidConversion) -> Self {
        ParseError(error.to_string())
    }
}

pub enum Parameter<'ap, T> {
    Opt {
        field: Field<'ap, T>,
        nargs: Nargs,
        name: &'static str,
        short: Option<char>,
        description: Option<&'static str>,
    },
    Arg {
        field: Field<'ap, T>,
        nargs: Nargs,
        name: &'static str,
        description: Option<&'static str>,
    },
}

impl<'ap, T> Parameter<'ap, T> {
    pub fn option(
        generic_capturable: impl GenericCapturable<'ap, T> + CliOption + 'ap,
        name: &'static str,
        short: Option<char>,
    ) -> Self {
        let nargs = generic_capturable.nargs();
        Parameter::Opt {
            field: Field::binding(generic_capturable),
            nargs,
            name,
            short,
            description: None,
        }
    }

    pub fn argument(
        generic_capturable: impl GenericCapturable<'ap, T> + CliArgument + 'ap,
        name: &'static str,
    ) -> Self {
        let nargs = generic_capturable.nargs();
        Parameter::Arg {
            field: Field::binding(generic_capturable),
            nargs,
            name,
            description: None,
        }
    }

    pub fn help(self, message: &'static str) -> Self {
        match self {
            Parameter::Opt {
                field,
                nargs,
                name,
                short,
                ..
            } => Parameter::Opt {
                field,
                nargs,
                name,
                short,
                description: Some(message),
            },
            Parameter::Arg {
                field, nargs, name, ..
            } => Parameter::Arg {
                field,
                nargs,
                name,
                description: Some(message),
            },
        }
    }
}

type OptionParameter = (String, Option<char>, Nargs, Option<&'static str>);
type ArgumentParameter = (String, Nargs, Option<&'static str>);

pub struct ArgumentParser<'ap> {
    program: &'ap str,
    option_parameters: Vec<OptionParameter>,
    argument_parameters: Vec<ArgumentParameter>,
    option_captures: Vec<OptionCapture<'ap>>,
    argument_captures: Vec<ArgumentCapture<'ap>>,
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
            option_parameters: Vec::default(),
            argument_parameters: Vec::default(),
            option_captures: Vec::default(),
            argument_captures: Vec::default(),
        }
    }

    pub fn add<T>(mut self, parameter: Parameter<'ap, T>) -> Self {
        match parameter {
            Parameter::Opt {
                field,
                nargs,
                name,
                short,
                description,
            } => {
                self.option_captures.push((
                    OptionConfig::new(name.to_string(), short.clone(), Bound::from(nargs)),
                    Box::new(field),
                ));
                self.option_parameters
                    .push((name.to_string(), short, nargs, description));
            }
            Parameter::Arg {
                field,
                nargs,
                name,
                description,
            } => {
                self.argument_captures.push((
                    ArgumentConfig::new(name.to_string(), Bound::from(nargs)),
                    Box::new(field),
                ));
                self.argument_parameters
                    .push((name.to_string(), nargs, description));
            }
        };
        self
    }

    pub fn build(self) -> Result<Parser<'ap>, ConfigError> {
        Ok(Parser {
            program: self.program,
            option_parameters: self.option_parameters,
            argument_parameters: self.argument_parameters,
            parse_capture: ParseCapture::new(self.option_captures, self.argument_captures)?,
            user_interface: Box::new(Console::default()),
        })
    }
}

pub struct Parser<'ap> {
    program: &'ap str,
    option_parameters: Vec<OptionParameter>,
    argument_parameters: Vec<ArgumentParameter>,
    parse_capture: ParseCapture<'ap>,
    user_interface: Box<dyn UserInterface>,
}

fn print_help<'ap>(
    program: &'ap str,
    mut options: Vec<OptionParameter>,
    arguments: Vec<ArgumentParameter>,
    user_interface: Box<dyn UserInterface>,
) {
    options.sort_by(|a, b| a.0.cmp(&b.0));
    let help_flags = format!("-{HELP_SHORT}, --{HELP_NAME}");
    let mut summary = vec![format!("[-{HELP_SHORT}]")];
    let mut column_width = help_flags.len();
    let mut grammars: HashMap<String, String> = HashMap::default();

    for (name, short, nargs, _) in &options {
        let grammar = match nargs {
            Nargs::Precisely(0) => "".to_string(),
            Nargs::Precisely(n) => format!(
                " {}",
                (0..*n)
                    .map(|_| name.to_ascii_uppercase())
                    .collect::<Vec<String>>()
                    .join(" ")
            ),
            Nargs::Any => format!(" [{} ...]", name.to_ascii_uppercase()),
            Nargs::AtLeastOne => format!(" {} [...]", name.to_ascii_uppercase()),
        };
        grammars.insert(name.clone(), grammar.clone());
        match short {
            Some(s) => {
                if column_width < name.len() + grammar.len() + 6 {
                    column_width = name.len() + grammar.len() + 6;
                }

                summary.push(format!("[-{s}{grammar}]"));
            }
            None => {
                if column_width < name.len() + grammar.len() + 2 {
                    column_width = name.len() + grammar.len() + 2;
                }

                summary.push(format!("[--{name}{grammar}]"));
            }
        };
    }

    for (name, nargs, _) in &arguments {
        let grammar = match nargs {
            Nargs::Precisely(n) => format!(
                "{}",
                (0..*n)
                    .map(|_| name.to_ascii_uppercase())
                    .collect::<Vec<String>>()
                    .join(" ")
            ),
            Nargs::Any => format!("[{} ...]", name.to_ascii_uppercase()),
            Nargs::AtLeastOne => format!("{} [...]", name.to_ascii_uppercase()),
        };
        grammars.insert(name.clone(), grammar.clone());

        if column_width < grammar.len() {
            column_width = grammar.len();
        }

        summary.push(format!("{grammar}"));
    }

    user_interface.print_help(format!("usage: {program} {}", summary.join(" ")));

    if !arguments.is_empty() {
        user_interface.print_help("".to_string());
        user_interface.print_help("positional arguments:".to_string());

        for (name, _, description) in &arguments {
            let grammar = grammars
                .remove(name)
                .expect("internal error - must have been set");
            let argument_description = match description {
                Some(message) => format!("  {message}"),
                None => "".to_string(),
            };
            user_interface.print_help(format!(" {:column_width$}{argument_description}", grammar));
        }
    }

    user_interface.print_help("".to_string());
    user_interface.print_help("options:".to_string());
    user_interface.print_help(format!(
        " {:column_width$}  Show this help message and exit.",
        help_flags
    ));

    for (name, short, _, description) in &options {
        let grammar = grammars
            .remove(name)
            .expect("internal error - must have been set");
        let option_flags = match short {
            Some(s) => format!("-{s}, --{name}{grammar}"),
            None => format!("--{name}{grammar}"),
        };
        let option_description = match description {
            Some(message) => format!("  {message}"),
            None => "".to_string(),
        };
        user_interface.print_help(format!(
            " {:column_width$}{option_description}",
            option_flags
        ));
    }
}

impl<'ap> Parser<'ap> {
    fn parse_tokens(self, tokens: &[&str]) -> Result<Action, ()> {
        match self.parse_capture.consume(tokens) {
            Ok(Action::RunProgram) => Ok(Action::RunProgram),
            Ok(Action::PrintHelp) => {
                print_help(
                    self.program,
                    self.option_parameters,
                    self.argument_parameters,
                    self.user_interface,
                );
                Ok(Action::PrintHelp)
            }
            Err((offset, parse_error)) => {
                self.user_interface.print_error(parse_error);
                self.user_interface.print_context(tokens, offset);
                Err(())
            }
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

        match result {
            Ok(Action::RunProgram) => {}
            Ok(Action::PrintHelp) => {
                std::process::exit(0);
            }
            Err(()) => {
                std::process::exit(1);
            }
        };
    }
}

enum Action {
    RunProgram,
    PrintHelp,
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
            OptionConfig::new(HELP_NAME.to_string(), Some(HELP_SHORT), Bound::Range(0, 0));
        let mut option_configs = HashSet::from([help_config]);
        let mut argument_configs = VecDeque::default();
        let mut captures: HashMap<String, Box<(dyn AnonymousCapturable + 'ap)>> =
            HashMap::default();

        for (oc, f) in options.into_iter() {
            if captures.insert(oc.name(), f).is_some() {
                return Err(ConfigError(format!(
                    "Cannot duplicate the parameter '{}'.",
                    oc.name()
                )));
            }

            option_configs.insert(oc);
        }

        for (ac, f) in arguments.into_iter() {
            if captures.insert(ac.name(), f).is_some() {
                return Err(ConfigError(format!(
                    "Cannot duplicate the parameter '{}'.",
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

    fn consume(mut self, tokens: &[&str]) -> Result<Action, (usize, ParseError)> {
        // 1. Feed the raw token strings to the matcher.
        let mut fed = 0;

        for next in tokens {
            let next_length = next.len();
            self.token_matcher
                .feed(next)
                .map_err(|e| (fed, ParseError::from(e)))?;
            fed += next_length;
        }

        let matches = match self.token_matcher.close() {
            Ok(matches) | Err((_, _, matches)) if matches.contains(HELP_NAME) => {
                return Ok(Action::PrintHelp);
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
                box_capture
                    .capture(value)
                    .map_err(|e| (*offset, ParseError::from(e)))?;
            }
        }

        Ok(Action::RunProgram)
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
        ap.add(Parameter::option(
            Value::new(&mut variable),
            "variable",
            Some('v'),
        ))
        .build()
        .unwrap()
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
        ap.add(Parameter::option(
            Switch::new(&mut variable, 2),
            "variable",
            Some('v'),
        ))
        .build()
        .unwrap()
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
        ap.add(Parameter::option(
            Collection::new(&mut variable, Nargs::Any),
            "variable",
            Some('v'),
        ))
        .build()
        .unwrap()
        .parse_tokens(tokens.as_slice())
        .unwrap();
        assert_eq!(variable, expected);
    }

    #[test]
    fn ap_argument_value() {
        let ap = ArgumentParser::new("abc");
        let mut variable: u32 = 0;
        ap.add(Parameter::argument(Value::new(&mut variable), "variable"))
            .build()
            .unwrap()
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
        ap.add(Parameter::argument(
            Collection::new(&mut variable, Nargs::Any),
            "variable",
        ))
        .build()
        .unwrap()
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
        ap.add(Parameter::argument(Value::new(&mut variable), "variable"))
            .build()
            .unwrap()
            .parse_tokens(tokens.as_slice())
            .unwrap();
        assert_eq!(variable, 0);
    }
}
