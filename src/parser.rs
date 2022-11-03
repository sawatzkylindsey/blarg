use std::collections::{HashMap, HashSet, VecDeque};
use std::convert::TryFrom;
use std::env;
use std::marker::PhantomData;
use std::rc::Rc;
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

enum ParameterInner<'ap, T> {
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

pub struct Condition<'ap, T> {
    arg_parameter: Parameter<'ap, T>,
}

impl<'ap, T: std::str::FromStr> Condition<'ap, T> {
    pub fn new(value: Scalar<'ap, T>, name: &'static str) -> Self {
        Self {
            arg_parameter: Parameter::argument(value, name),
        }
    }

    fn name(&self) -> String {
        if let ParameterInner::Arg { name, .. } = self.arg_parameter.0 {
            name.to_string()
        } else {
            panic!("internal error - argument must always be ParameterInner::Arg");
        }
    }
}

pub struct Parameter<'ap, T>(ParameterInner<'ap, T>);

impl<'ap, T> Parameter<'ap, T> {
    pub fn option(
        capturable: impl GenericCapturable<'ap, T> + CliOption + 'ap,
        name: &'static str,
        short: Option<char>,
    ) -> Self {
        let nargs = capturable.nargs();
        Self(ParameterInner::Opt {
            field: Field::binding(capturable),
            nargs,
            name,
            short,
            description: None,
        })
    }

    pub fn argument(
        capturable: impl GenericCapturable<'ap, T> + CliArgument + 'ap,
        name: &'static str,
    ) -> Self {
        let nargs = capturable.nargs();
        Self(ParameterInner::Arg {
            field: Field::binding(capturable),
            nargs,
            name,
            description: None,
        })
    }

    pub fn help(self, message: &'static str) -> Self {
        match self.0 {
            ParameterInner::Opt {
                field,
                nargs,
                name,
                short,
                ..
            } => Self(ParameterInner::Opt {
                field,
                nargs,
                name,
                short,
                description: Some(message),
            }),
            ParameterInner::Arg {
                field, nargs, name, ..
            } => Self(ParameterInner::Arg {
                field,
                nargs,
                name,
                description: Some(message),
            }),
        }
    }
}

type OptionParameter = (String, Option<char>, Nargs, Option<&'static str>);
type ArgumentParameter = (String, Nargs, Option<&'static str>);

pub struct CommandParser<'ap> {
    program: String,
    option_parameters: Vec<OptionParameter>,
    argument_parameters: Vec<ArgumentParameter>,
    option_captures: Vec<OptionCapture<'ap>>,
    argument_captures: Vec<ArgumentCapture<'ap>>,
    discriminator: Option<String>,
}

impl<'ap> CommandParser<'ap> {
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            option_parameters: Vec::default(),
            argument_parameters: Vec::default(),
            option_captures: Vec::default(),
            argument_captures: Vec::default(),
            discriminator: None,
        }
    }

    pub fn add<T>(mut self, parameter: Parameter<'ap, T>) -> Self {
        match parameter.0 {
            ParameterInner::Opt {
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
            ParameterInner::Arg {
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

    pub fn branch<T: std::str::FromStr + std::fmt::Display>(
        mut self,
        condition: Condition<'ap, T>,
    ) -> SubCommandParser<'ap, T> {
        if self.discriminator.replace(condition.name()).is_some() {
            panic!("internal error - cannot setup multiple discriminators");
        }

        SubCommandParser::new(self.add(condition.arg_parameter))
    }

    pub fn build(self) -> Result<GeneralParser<'ap>, ConfigError> {
        Ok(GeneralParser {
            program: self.program.clone(),
            command: ParseUnit::try_from(self)?,
            sub_commands: HashMap::default(),
            user_interface: Rc::new(Console::default()),
        })
    }
}

pub struct SubCommandParser<'ap, B: std::fmt::Display> {
    root: CommandParser<'ap>,
    commands: HashMap<String, CommandParser<'ap>>,
    _phantom: PhantomData<B>,
}

impl<'ap, B: std::fmt::Display> SubCommandParser<'ap, B> {
    pub fn new(root: CommandParser<'ap>) -> Self {
        Self {
            root,
            commands: HashMap::default(),
            _phantom: PhantomData,
        }
    }

    pub fn add<T>(mut self, sub_command: B, parameter: Parameter<'ap, T>) -> Self {
        let command_str = sub_command.to_string();
        let cp = self
            .commands
            .remove(&command_str)
            .unwrap_or_else(|| CommandParser::new(command_str.clone()));
        self.commands.insert(command_str, cp.add(parameter));
        self
    }

    pub fn build(self) -> Result<GeneralParser<'ap>, ConfigError> {
        let mut sub_commands = HashMap::default();

        for (command, cp) in self.commands.into_iter() {
            sub_commands.insert(command, ParseUnit::try_from(cp)?);
        }

        Ok(GeneralParser {
            program: self.root.program.clone(),
            command: ParseUnit::try_from(self.root)?,
            sub_commands,
            user_interface: Rc::new(Console::default()),
        })
    }
}

impl<'ap> TryFrom<CommandParser<'ap>> for ParseUnit<'ap> {
    type Error = ConfigError;

    fn try_from(value: CommandParser<'ap>) -> Result<Self, Self::Error> {
        Ok(Self {
            options: value.option_parameters,
            arguments: value.argument_parameters,
            parser: Parser::new(
                value.option_captures,
                value.argument_captures,
                value.discriminator,
            )?,
        })
    }
}

fn print_help(
    program: String,
    mut options: Vec<OptionParameter>,
    arguments: Vec<ArgumentParameter>,
    user_interface: Rc<dyn UserInterface>,
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

    user_interface.print(format!("usage: {program} {}", summary.join(" ")));

    if !arguments.is_empty() {
        user_interface.print("".to_string());
        user_interface.print("positional arguments:".to_string());

        for (name, _, description) in &arguments {
            let grammar = grammars
                .remove(name)
                .expect("internal error - must have been set");
            let argument_description = match description {
                Some(message) => format!("  {message}"),
                None => "".to_string(),
            };
            user_interface.print(format!(" {:column_width$}{argument_description}", grammar));
        }
    }

    user_interface.print("".to_string());
    user_interface.print("options:".to_string());
    user_interface.print(format!(
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
        user_interface.print(format!(
            " {:column_width$}{option_description}",
            option_flags
        ));
    }
}

pub struct GeneralParser<'ap> {
    program: String,
    command: ParseUnit<'ap>,
    sub_commands: HashMap<String, ParseUnit<'ap>>,
    user_interface: Rc<dyn UserInterface>,
}

struct ParseUnit<'ap> {
    options: Vec<OptionParameter>,
    arguments: Vec<ArgumentParameter>,
    parser: Parser<'ap>,
}

impl<'ap> ParseUnit<'ap> {
    fn invoke(
        self,
        tokens: &[&str],
        program: String,
        user_interface: Rc<dyn UserInterface>,
    ) -> ParseResult {
        match self.parser.consume(tokens) {
            Ok(Action::Continue {
                discriminee,
                remaining,
            }) => match discriminee {
                Some((offset, variant)) => ParseResult::Incomplete {
                    variant_offset: offset,
                    variant,
                    remaining,
                },
                None => ParseResult::Complete,
            },
            Ok(Action::PrintHelp) => {
                print_help(program, self.options, self.arguments, user_interface);
                ParseResult::Exit(0)
            }
            Err((offset, parse_error)) => {
                user_interface.print_error(parse_error);
                user_interface.print_context(tokens, offset);
                ParseResult::Exit(1)
            }
        }
    }
}

enum ParseResult {
    Complete,
    Incomplete {
        variant_offset: usize,
        variant: String,
        remaining: Vec<String>,
    },
    Exit(i32),
}

impl<'ap> GeneralParser<'ap> {
    fn parse_tokens(mut self, tokens: &[&str]) -> Result<(), i32> {
        let command_result = self.command.invoke(
            tokens,
            self.program.clone(),
            Rc::clone(&self.user_interface),
        );

        match command_result {
            ParseResult::Complete => Ok(()),
            ParseResult::Incomplete {
                variant_offset,
                variant,
                remaining,
            } => {
                match self.sub_commands.remove(&variant) {
                    Some(sub_command) => {
                        match sub_command.invoke(
                            remaining
                                .iter()
                                .map(AsRef::as_ref)
                                .collect::<Vec<&str>>()
                                .as_slice(),
                            format!("{program} {variant}", program = self.program),
                            Rc::clone(&self.user_interface),
                        ) {
                            ParseResult::Complete => Ok(()),
                            ParseResult::Incomplete { .. } => {
                                panic!("internal error - sub-command parse must complete/exit.")
                            }
                            ParseResult::Exit(code) => Err(code),
                        }
                    }
                    None => {
                        // The varaint isn't amongst the sub-commands.
                        // Either the user specified an invalid variant, OR
                        // the program invalidates the 'Display' inverse-to 'FromStr' / 'FromStr' inverse-to 'Display' requirement.
                        self.user_interface
                            .print_error(ParseError(format!("Unknown sub-command '{variant}'.")));
                        self.user_interface.print_context(tokens, variant_offset);
                        Err(1)
                    }
                }
            }
            ParseResult::Exit(code) => Err(code),
        }
    }

    pub fn parse(self) {
        let command_input: Vec<String> = env::args().skip(1).collect();
        match self.parse_tokens(
            command_input
                .iter()
                .map(AsRef::as_ref)
                .collect::<Vec<&str>>()
                .as_slice(),
        ) {
            Ok(()) => {}
            Err(exit_code) => {
                std::process::exit(exit_code);
            }
        };
    }
}

enum Action {
    Continue {
        discriminee: Option<OffsetValue>,
        remaining: Vec<String>,
    },
    PrintHelp,
}

struct Parser<'ap> {
    token_matcher: TokenMatcher,
    captures: HashMap<String, Box<(dyn AnonymousCapturable + 'ap)>>,
    discriminator: Option<String>,
}

// We need a (dyn .. [ignoring T] ..) here in order to put all the fields of varying types T under one collection.
// In other words, we want the bottom of the object graph to include the types T, but up here we want to work across all T.
type OptionCapture<'ap> = (OptionConfig, Box<(dyn AnonymousCapturable + 'ap)>);
type ArgumentCapture<'ap> = (ArgumentConfig, Box<(dyn AnonymousCapturable + 'ap)>);
const HELP_NAME: &'static str = "help";
const HELP_SHORT: char = 'h';

impl<'ap> Parser<'ap> {
    fn new(
        options: Vec<OptionCapture<'ap>>,
        arguments: Vec<ArgumentCapture<'ap>>,
        discriminator: Option<String>,
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
            discriminator,
        })
    }

    fn consume(mut self, tokens: &[&str]) -> Result<Action, (usize, ParseError)> {
        let mut token_iter = tokens.iter();
        let minimal_consume = self.discriminator.is_some();
        // 1. Feed the raw token strings to the matcher.
        let mut fed = 0;

        loop {
            match token_iter.next() {
                Some(token) => {
                    let token_length = token.len();
                    self.token_matcher
                        .feed(token)
                        .map_err(|e| (fed, ParseError::from(e)))?;
                    fed += token_length;

                    if minimal_consume && self.token_matcher.can_close() {
                        break;
                    }
                }
                None => break,
            }
        }

        let matches = match self.token_matcher.close() {
            Ok(matches) | Err((_, _, matches)) if matches.contains(HELP_NAME) => {
                return Ok(Action::PrintHelp);
            }
            Ok(matches) => Ok(matches),
            Err((offset, e, _)) => Err((offset, ParseError::from(e))),
        }?;

        let mut discriminee: Option<OffsetValue> = None;

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

            if let Some(ref target) = self.discriminator {
                if target == &match_tokens.name {
                    match &match_tokens.values[..] {
                        [(offset, value)] => {
                            if discriminee.replace((*offset, value.clone())).is_some() {
                                panic!(
                                    "internal error - discriminator cannot have multiple matches"
                                );
                            }
                        }
                        _ => {
                            panic!(
                                "internal error - discriminator must result it precisely 1 token"
                            );
                        }
                    }
                }
            }
        }

        Ok(Action::Continue {
            discriminee,
            remaining: token_iter.map(|s| s.to_string()).collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[test]
    fn ap_empty() {
        let ap = CommandParser::new("abc");
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
        let ap = CommandParser::new("abc");
        let mut variable: u32 = 0;
        ap.add(Parameter::option(
            Scalar::new(&mut variable),
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
        let ap = CommandParser::new("abc");
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
        let ap = CommandParser::new("abc");
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
        let ap = CommandParser::new("abc");
        let mut variable: u32 = 0;
        ap.add(Parameter::argument(Scalar::new(&mut variable), "variable"))
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
        let ap = CommandParser::new("abc");
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
        let ap = CommandParser::new("abc");
        let mut variable: u32 = 0;
        let exit_code = ap
            .add(Parameter::argument(Scalar::new(&mut variable), "variable"))
            .build()
            .unwrap()
            .parse_tokens(tokens.as_slice())
            .unwrap_err();
        assert_eq!(exit_code, 0);
        assert_eq!(variable, 0);
    }

    //fn make_input(tokens: Vec<&str>) -> Vec<String> {
}
