use std::collections::HashMap;
use std::marker::PhantomData;

use crate::api::{Condition, Parameter, ParameterClass};
use crate::parser::{
    ArgumentCapture, ArgumentParameter, ConfigError, ConsoleInterface, GeneralParser,
    OptionCapture, UserInterface,
};
use crate::parser::{OptionParameter, ParseUnit, Parser, Printer};

/// The base command line parser.
///
/// ### Example
/// ```
/// # use blarg_builder as blarg;
/// use blarg::{CommandLineParser};
///
/// let parser = CommandLineParser::new("program")
///     // Configure with CommandLineParser::add and CommandLineParser::branch.
///     .build();
/// parser.parse_tokens(empty::slice()).unwrap();
/// ```
pub struct CommandLineParser<'a> {
    program: String,
    about: Option<String>,
    option_parameters: Vec<OptionParameter>,
    argument_parameters: Vec<ArgumentParameter>,
    option_captures: Vec<OptionCapture<'a>>,
    argument_captures: Vec<ArgumentCapture<'a>>,
    discriminator: Option<String>,
}

impl<'a> CommandLineParser<'a> {
    /// Create a command line parser.
    ///
    /// ### Example
    /// ```
    /// # use blarg_builder as blarg;
    /// use blarg::CommandLineParser;
    ///
    /// let parser = CommandLineParser::new("program")
    ///     .build();
    ///
    /// parser.parse_tokens(vec![].as_slice()).unwrap();
    /// ```
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            about: None,
            option_parameters: Vec::default(),
            argument_parameters: Vec::default(),
            option_captures: Vec::default(),
            argument_captures: Vec::default(),
            discriminator: None,
        }
    }

    /// Document the about message for this command line parser.
    /// If repeated, only the final help message will apply.
    ///
    /// An about message documents the command line parser in full sentence/paragraph format.
    /// We recommend allowing `blarg` to format this field (ex: it is not recommended to use line breaks `'\n'`).
    ///
    /// ### Example
    /// ```
    /// # use blarg_builder as blarg;
    /// use blarg::CommandLineParser;
    ///
    /// let parser = CommandLineParser::new("program")
    ///     .about("--this will get discarded--")
    ///     .about("My program that does awesome stuff.  Check it out!")
    ///     .build();
    ///
    /// parser.parse_tokens(vec![].as_slice()).unwrap();
    /// ```
    pub fn about(mut self, description: impl Into<String>) -> Self {
        self.about.replace(description.into());
        self
    }

    /// Add an argument/option to the command line parser.
    ///
    /// The order of argument parameters corresponds to their positional order during parsing.
    /// The order of option parameters does not affect the command parser semantics.
    ///
    /// ### Example
    /// ```
    /// # use blarg_builder as blarg;
    /// use blarg::{CommandLineParser, Parameter, Scalar};
    ///
    /// let mut a: u32 = 0;
    /// let mut b: u32 = 0;
    /// let parser = CommandLineParser::new("program")
    ///     .add(Parameter::argument(Scalar::new(&mut a), "a"))
    ///     .add(Parameter::argument(Scalar::new(&mut b), "b"))
    ///     .build();
    ///
    /// parser.parse_tokens(vec!["1", "2"].as_slice()).unwrap();
    ///
    /// assert_eq!(a, 1);
    /// assert_eq!(b, 2);
    /// ```
    pub fn add<T>(mut self, parameter: Parameter<'a, T>) -> Self {
        let inner = parameter.consume();
        match inner.class() {
            ParameterClass::Opt => {
                self.option_parameters.push(OptionParameter::from(&inner));
                self.option_captures.push(OptionCapture::from(inner));
            }
            ParameterClass::Arg => {
                self.argument_parameters
                    .push(ArgumentParameter::from(&inner));
                self.argument_captures.push(ArgumentCapture::from(inner));
            }
        }

        self
    }

    /// Branch into a sub-command parser.
    ///
    /// This changes the command line parser into a sub-command style command line parser.
    /// Any parameters added before the branch apply to the root parser.
    ///
    /// Branching is always done with a special `Scalar` argument: [`Condition`].
    ///
    /// ### Example
    /// ```
    /// # use blarg_builder as blarg;
    /// use blarg::{CommandLineParser, Parameter, Scalar, Condition};
    ///
    /// let mut belongs_to_root: u32 = 0;
    /// let mut sub_command: String = "".to_string();
    /// let mut belongs_to_sub_command: u32 = 0;
    /// let parser = CommandLineParser::new("program")
    ///     .add(Parameter::argument(Scalar::new(&mut belongs_to_root), "belongs_to_root"))
    ///     .branch(Condition::new(Scalar::new(&mut sub_command), "sub_command"))
    ///     .command("the-command".to_string(), |sub| {
    ///         sub.add(Parameter::argument(Scalar::new(&mut belongs_to_sub_command), "belongs_to_sub_command"))
    ///     })
    ///     .build();
    ///
    /// parser.parse_tokens(vec!["1", "the-command", "2"].as_slice()).unwrap();
    ///
    /// assert_eq!(belongs_to_root, 1);
    /// assert_eq!(&sub_command, "the-command");
    /// assert_eq!(belongs_to_sub_command, 2);
    /// ```
    pub fn branch<T: std::str::FromStr + std::fmt::Display + PartialEq>(
        mut self,
        condition: Condition<'a, T>,
    ) -> SubCommandParser<'a, T> {
        let parameter = condition.consume();
        if self.discriminator.replace(parameter.name()).is_some() {
            unreachable!("internal error - cannot setup multiple discriminators");
        }

        SubCommandParser::new(self.add(parameter))
    }

    fn build_with_interface(
        self,
        user_interface: Box<dyn UserInterface>,
    ) -> Result<GeneralParser<'a>, ConfigError> {
        let parser = Parser::new(
            self.option_captures,
            self.argument_captures,
            self.discriminator,
        )?;
        let command = ParseUnit::new(
            parser,
            Printer::terminal(
                self.program.clone(),
                self.about,
                self.option_parameters,
                self.argument_parameters,
            ),
        );
        Ok(GeneralParser::command(command, user_interface))
    }

    /// Build the command line parser as a Result.
    /// This finalizes the configuration and checks for errors (ex: a repeated parameter name).
    pub fn build_parser(self) -> Result<GeneralParser<'a>, ConfigError> {
        self.build_with_interface(Box::new(ConsoleInterface::default()))
    }

    /// Build the command line parser.
    /// This finalizes the configuration and checks for errors (ex: a repeated parameter name).
    /// If an error is encountered, exits with error code `1` (via [`std::process::exit`]).
    pub fn build(self) -> GeneralParser<'a> {
        match self.build_parser() {
            Ok(gp) => gp,
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        }
    }
}

/// The sub-command parser.
pub struct SubCommandParser<'a, B: std::fmt::Display> {
    root: CommandLineParser<'a>,
    commands: HashMap<String, CommandLineParser<'a>>,
    deferred_error: Option<ConfigError>,
    _phantom: PhantomData<B>,
}

impl<'a, B: std::str::FromStr + std::fmt::Display + PartialEq> SubCommandParser<'a, B> {
    fn new(root: CommandLineParser<'a>) -> Self {
        Self {
            root,
            commands: HashMap::default(),
            deferred_error: None,
            _phantom: PhantomData,
        }
    }

    /// Setup a sub-command.
    ///
    /// Sub-commands may be added arbitrarily, as long as the correspond to the branching type `B`.
    /// If repeated for the same `variant` of `B`, only the final version will be created on the parser.
    /// The order of sub-commands does not affect the command parser semantics.
    ///
    /// ### Example
    /// ```
    /// # use blarg_builder as blarg;
    /// use blarg::{CommandLineParser, Condition, Parameter, Scalar};
    ///
    /// let mut value_a: u32 = 0;
    /// let mut value_b: u32 = 0;
    /// let mut sub_command: String = "".to_string();
    /// let parser = CommandLineParser::new("program")
    ///     .branch(Condition::new(Scalar::new(&mut sub_command), "sub_command"))
    ///     .command("a".to_string(), |sub| sub.add(Parameter::argument(Scalar::new(&mut value_a), "value_a")))
    ///     .command("b".to_string(), |sub| {
    ///         sub.about("Description for the sub-command 'b'.")
    ///             .add(Parameter::argument(Scalar::new(&mut value_b), "value_b"))
    ///     })
    ///     .build();
    ///
    /// parser.parse_tokens(vec!["a", "1"].as_slice()).unwrap();
    ///
    /// assert_eq!(&sub_command, "a");
    /// assert_eq!(value_a, 1);
    /// assert_eq!(value_b, 0);
    /// ```
    pub fn command(
        mut self,
        variant: B,
        setup_fn: impl FnOnce(SubCommand<'a>) -> SubCommand<'a>,
    ) -> Self {
        let command_str = variant.to_string();

        // Check if the variant does not respect the FromStr-inverts-Display invariant.
        match B::from_str(&command_str) {
            // This is where someone is trying to trick us!
            // The from_str inverts to a valid `B`, however it is not this specific variant.
            Ok(value) if value != variant => {
                self.deferred_error.replace(ConfigError(format!(
                    "parameter '{}' contains invalid sub-command '{command_str}': FromStr does not invert Display.",
                    self.root.discriminator.as_ref().expect("internal error - root must have a discriminator"),
                )));
            }
            // The from_str simply does not invert to a valid `B`.
            Err(_) => {
                self.deferred_error.replace(ConfigError(format!(
                    "parameter '{}' contains invalid sub-command '{command_str}': FromStr does not invert Display.",
                    self.root.discriminator.as_ref().expect("internal error - root must have a discriminator"),
                )));
            }
            _ => {
                // Do nothing.
            }
        }

        let inner = CommandLineParser::new(command_str.clone());
        let sub_command = setup_fn(SubCommand { inner });
        self.commands.insert(command_str, sub_command.inner);
        self
    }

    fn build_with_interface(
        self,
        user_interface: Box<dyn UserInterface>,
    ) -> Result<GeneralParser<'a>, ConfigError> {
        if let Some(error) = self.deferred_error {
            return Err(error);
        }

        let mut sub_commands = HashMap::default();

        for (discriminee, cp) in self.commands.into_iter() {
            let sub_parser = Parser::new(cp.option_captures, cp.argument_captures, None)?;
            let sub_command = ParseUnit::new(
                sub_parser,
                Printer::terminal(
                    format!(
                        "{program} {sub_program}",
                        program = self.root.program,
                        sub_program = cp.program
                    ),
                    cp.about,
                    cp.option_parameters,
                    cp.argument_parameters,
                ),
            );
            sub_commands.insert(discriminee, sub_command);
        }

        let parser = Parser::new(
            self.root.option_captures,
            self.root.argument_captures,
            self.root.discriminator,
        )?;
        let command = ParseUnit::new(
            parser,
            Printer::terminal(
                self.root.program.clone(),
                self.root.about,
                self.root.option_parameters,
                self.root.argument_parameters,
            ),
        );
        Ok(GeneralParser::sub_command(
            // self.root.program,
            command,
            sub_commands,
            user_interface,
        ))
    }

    /// Build the sub-command based command line parser as a Result.
    /// This finalizes the configuration and checks for errors (ex: a repeated parameter name).
    pub fn build_parser(self) -> Result<GeneralParser<'a>, ConfigError> {
        self.build_with_interface(Box::new(ConsoleInterface::default()))
    }

    /// Build the sub-command based command line parser.
    /// This finalizes the configuration and checks for errors (ex: a repeated parameter name).
    /// If an error is encountered, exits with error code `1` (via [`std::process::exit`]).
    pub fn build(self) -> GeneralParser<'a> {
        match self.build_parser() {
            Ok(gp) => gp,
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        }
    }
}

/// A sub-command line parser.
///
/// Used with [`SubCommandParser::command`].
pub struct SubCommand<'a> {
    inner: CommandLineParser<'a>,
}

impl<'a> SubCommand<'a> {
    /// *Available using 'unit_test' crate feature only.*</br></br>
    /// Build a [`SubCommand`] for use in testing.
    ///
    /// ### Example
    /// ```
    /// # use blarg_builder as blarg;
    /// use blarg::{Parameter, Scalar, SubCommand};
    ///
    /// // Function under test.
    /// // We want to make sure the setup_fn is wired up correctly.
    /// pub fn setup_fn<'a>(value: &'a mut u32) -> impl FnOnce(SubCommand<'a>) -> SubCommand<'a> {
    ///     |sub| sub.add(Parameter::argument(Scalar::new(value), "value"))
    /// }
    ///
    /// let mut x: u32 = 1;
    /// let parser = setup_fn(&mut x)(SubCommand::test_dummy()).build_parser().unwrap();
    /// parser.parse_tokens(vec!["2"].as_slice()).unwrap();
    /// assert_eq!(x, 2);
    /// ```
    #[cfg(feature = "unit_test")]
    pub fn test_dummy() -> Self {
        SubCommand {
            inner: CommandLineParser::new("test-dummy"),
        }
    }

    /// *Available using 'unit_test' crate feature only.*</br></br>
    /// Build a [`GeneralParser`] for testing.
    /// See [`SubCommand::test_dummy`] for an example.
    #[cfg(feature = "unit_test")]
    pub fn build_parser(self) -> Result<GeneralParser<'a>, ConfigError> {
        self.inner
            .build_with_interface(Box::new(ConsoleInterface::default()))
    }

    /// Document the about message for this sub-command.
    /// If repeated, only the final help message will apply.
    ///
    /// An about message documents the sub-command in full sentence/paragraph format.
    /// We recommend allowing `blarg` to format this field (ex: it is not recommended to use line breaks `'\n'`).
    ///
    /// See [`SubCommandParser::command`] for usage.
    pub fn about(self, description: impl Into<String>) -> Self {
        SubCommand {
            inner: self.inner.about(description),
        }
    }

    /// Add an argument/option to the sub-command.
    ///
    /// The order of argument parameters corresponds to their positional order during parsing.
    /// The order of option parameters does not affect the sub-command parser semantics.
    ///
    /// See [`SubCommandParser::command`] for usage.
    pub fn add<T>(self, parameter: Parameter<'a, T>) -> Self {
        SubCommand {
            inner: self.inner.add(parameter),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{Collection, Parameter, Scalar, Switch};
    use crate::model::Nargs;
    use crate::parser::util::channel_interface;
    use crate::prelude::Choices;
    use crate::test::assert_contains;
    use rstest::rstest;

    #[test]
    fn empty_build() {
        // Setup
        let clp = CommandLineParser::new("program");

        // Execute
        let parser = clp.build_parser().unwrap();

        // Verify
        assert_eq!(parser.details(), ("program".to_string(), None));
        parser.parse_tokens(empty::slice()).unwrap();
    }

    #[rstest]
    #[case(vec![], false, vec![])]
    #[case(vec!["1"], false, vec![1])]
    #[case(vec!["01"], false, vec![1])]
    #[case(vec!["1", "3", "2"], false, vec![1, 3, 2])]
    #[case(vec!["--flag"], true, vec![])]
    #[case(vec!["--flag", "1"], true, vec![1])]
    #[case(vec!["--flag", "01"], true, vec![1])]
    #[case(vec!["--flag", "1", "3", "2"], true, vec![1, 3, 2])]
    fn build(
        #[case] tokens: Vec<&str>,
        #[case] expected_flag: bool,
        #[case] expected_items: Vec<u32>,
    ) {
        // Setup
        let mut flag: bool = false;
        let mut items: Vec<u32> = Vec::default();
        let clp = CommandLineParser::new("program")
            .about("abc def")
            .add(Parameter::option(
                Switch::new(&mut flag, true),
                "flag",
                Some('f'),
            ))
            .add(Parameter::argument(
                Collection::new(&mut items, Nargs::Any),
                "item",
            ));

        // Execute
        let parser = clp.build_parser().unwrap();

        // Verify
        assert_eq!(
            parser.details(),
            ("program".to_string(), Some("abc def".to_string()))
        );

        // We testing that build sets up the right parser.
        // So the verification involves invoking the parser with the various permutations.
        parser.parse_tokens(tokens.as_slice()).unwrap();
        assert_eq!(flag, expected_flag);
        assert_eq!(items, expected_items);
    }

    #[rstest]
    #[case(vec!["0"], false, 0, vec![], vec![])]
    #[case(vec!["0", "1"], false, 0, vec![1], vec![])]
    #[case(vec!["0", "1", "3", "2"], false, 0, vec![1, 3, 2], vec![])]
    #[case(vec!["1"], false, 1, vec![], vec![])]
    #[case(vec!["1", "1"], false, 1, vec![], vec![1])]
    #[case(vec!["1", "1", "3", "2"], false, 1, vec![], vec![1, 3, 2])]
    #[case(vec!["--flag", "0"], true, 0, vec![], vec![])]
    #[case(vec!["--flag", "0", "1"], true, 0, vec![1], vec![])]
    #[case(vec!["--flag", "0", "1", "3", "2"], true, 0, vec![1, 3, 2], vec![])]
    #[case(vec!["--flag", "1"], true, 1, vec![], vec![])]
    #[case(vec!["--flag", "1", "1"], true, 1, vec![], vec![1])]
    #[case(vec!["--flag", "1", "1", "3", "2"], true, 1, vec![], vec![1, 3, 2])]
    fn branch_build(
        #[case] tokens: Vec<&str>,
        #[case] expected_flag: bool,
        #[case] expected_sub: u32,
        #[case] expected_items_0: Vec<u32>,
        #[case] expected_items_1: Vec<u32>,
    ) {
        // Setup
        let mut flag: bool = false;
        let mut sub: u32 = 0;
        let mut items_0: Vec<u32> = Vec::default();
        let mut items_1: Vec<u32> = Vec::default();
        let clp = CommandLineParser::new("program");
        let scp = clp
            .add(Parameter::option(
                Switch::new(&mut flag, true),
                "flag",
                Some('f'),
            ))
            .branch(Condition::new(Scalar::new(&mut sub), "sub"))
            .command(0, |sub| {
                sub.add(Parameter::argument(
                    Collection::new(&mut items_0, Nargs::Any),
                    "item0",
                ))
            })
            .command(1, |sub| {
                sub.about("abc def").add(Parameter::argument(
                    Collection::new(&mut items_1, Nargs::Any),
                    "item1",
                ))
            });

        // Execute
        let parser = scp.build_parser().unwrap();

        // Verify
        assert_eq!(parser.details(), ("program".to_string(), None));
        assert_eq!(parser.sub_details("x"), None);
        assert_eq!(
            parser.sub_details("0"),
            Some(("program 0".to_string(), None))
        );
        assert_eq!(
            parser.sub_details("1"),
            Some(("program 1".to_string(), Some("abc def".to_string())))
        );

        // We testing that build sets up the right parser.
        // So the verification involves invoking the parser with the various permutations.
        parser.parse_tokens(tokens.as_slice()).unwrap();
        assert_eq!(flag, expected_flag);
        assert_eq!(sub, expected_sub);
        assert_eq!(items_0, expected_items_0);
        assert_eq!(items_1, expected_items_1);
    }

    #[test]
    fn repeat_command_build() {
        // Setup
        let mut sub: u32 = 0;
        let mut items_0: Vec<u32> = Vec::default();
        let mut items_1: Vec<u32> = Vec::default();
        let clp = CommandLineParser::new("program");
        let scp = clp
            .branch(Condition::new(Scalar::new(&mut sub), "sub"))
            .command(0, |sub| {
                sub.add(Parameter::argument(
                    Collection::new(&mut items_0, Nargs::Any),
                    "item0",
                ))
            })
            .command(0, |sub| {
                sub.add(Parameter::argument(
                    Collection::new(&mut items_1, Nargs::Any),
                    "item1",
                ))
            });

        // Execute
        let parser = scp.build_parser().unwrap();

        // Verify
        // We testing that build sets up the right parser.
        // So the verification involves invoking the parser with the various permutations.
        parser.parse_tokens(&["0", "1", "2", "3"]).unwrap();
        assert_eq!(sub, 0);
        assert_eq!(items_0, Vec::default());
        assert_eq!(items_1, vec![1, 2, 3]);
    }

    #[rstest]
    #[case(vec!["abc", "0"], false, "abc", 0, vec![])]
    #[case(vec!["abc", "0", "1"], false, "abc", 0, vec![1])]
    #[case(vec!["abc", "0", "1", "3", "2"], false, "abc", 0, vec![1, 3, 2])]
    #[case(vec!["--flag", "abc", "0"], true, "abc", 0, vec![])]
    #[case(vec!["--flag", "abc", "0", "1"], true, "abc", 0, vec![1])]
    #[case(vec!["--flag", "abc", "0", "1", "3", "2"], true, "abc", 0, vec![1, 3, 2])]
    #[case(vec!["abc", "--flag", "0"], true, "abc", 0, vec![])]
    #[case(vec!["abc", "--flag", "0", "1"], true, "abc", 0, vec![1])]
    #[case(vec!["abc", "--flag", "0", "1", "3", "2"], true, "abc", 0, vec![1, 3, 2])]
    fn root_arguments_branch_build(
        #[case] tokens: Vec<&str>,
        #[case] expected_flag: bool,
        #[case] expected_root: &str,
        #[case] expected_sub: u32,
        #[case] expected_items: Vec<u32>,
    ) {
        // Setup
        let mut flag: bool = false;
        let mut root: String = String::default();
        let mut sub: u32 = 0;
        let mut items: Vec<u32> = Vec::default();
        let clp = CommandLineParser::new("program");
        let scp = clp
            .add(Parameter::option(
                Switch::new(&mut flag, true),
                "flag",
                Some('f'),
            ))
            .add(Parameter::argument(Scalar::new(&mut root), "root"))
            .branch(Condition::new(Scalar::new(&mut sub), "sub"))
            .command(0, |sub| {
                sub.add(Parameter::argument(
                    Collection::new(&mut items, Nargs::Any),
                    "item0",
                ))
            });

        // Execute
        let parser = scp.build_parser().unwrap();

        // Verify
        assert_eq!(parser.details(), ("program".to_string(), None));

        // We testing that build sets up the right parser.
        // So the verification involves invoking the parser with the various permutations.
        parser.parse_tokens(tokens.as_slice()).unwrap();
        assert_eq!(flag, expected_flag);
        assert_eq!(&root, expected_root);
        assert_eq!(sub, expected_sub);
        assert_eq!(items, expected_items);
    }

    #[test]
    fn empty_build_help() {
        // Setup
        let clp = CommandLineParser::new("program");
        let (sender, receiver) = channel_interface();

        // Execute
        let parser = clp.build_with_interface(Box::new(sender)).unwrap();

        // Verify
        // We testing that build sets up the right parser.
        // So the verification involves invoking the parser with --help and spot-checking the output.
        let error_code = parser.parse_tokens(&["--help"]).unwrap_err();
        assert_eq!(error_code, 0);

        let message = receiver.consume_message();
        assert_contains!(message, "usage: program [-h]\n");
    }

    #[test]
    fn build_help() {
        // Setup
        let mut flag: bool = false;
        let mut items: Vec<u32> = Vec::default();
        let mut clp = CommandLineParser::new("program");
        clp = clp
            .add(Parameter::option(
                Switch::new(&mut flag, true),
                "flag",
                Some('f'),
            ))
            .add(Parameter::argument(
                Collection::new(&mut items, Nargs::Any),
                "item",
            ));
        let (sender, receiver) = channel_interface();

        // Execute
        let parser = clp.build_with_interface(Box::new(sender)).unwrap();

        // Verify
        // We testing that build sets up the right parser.
        // So the verification involves invoking the parser with --help and spot-checking the output.
        let error_code = parser.parse_tokens(&["--help"]).unwrap_err();
        assert_eq!(error_code, 0);

        let message = receiver.consume_message();
        assert_contains!(message, "usage: program [-h] [-f] [ITEM ...]\n");
        assert_contains!(message, "-f, --flag");
    }

    #[test]
    fn branch_build_help() {
        // Setup
        let mut flag: bool = false;
        let mut sub: u32 = 0;
        let clp = CommandLineParser::new("program");
        let scp = clp
            .add(Parameter::option(
                Switch::new(&mut flag, true),
                "flag",
                Some('f'),
            ))
            .branch(
                Condition::new(Scalar::new(&mut sub), "sub")
                    .choice(0, "zero")
                    .choice(1, "one"),
            )
            .command(0, |sub| sub)
            .command(1, |sub| sub);
        let (sender, receiver) = channel_interface();

        // Execute
        let parser = scp.build_with_interface(Box::new(sender)).unwrap();

        // Verify
        // We testing that build sets up the right parser.
        // So the verification involves invoking the parser with --help and spot-checking the output.
        let error_code = parser.parse_tokens(&["--help"]).unwrap_err();
        assert_eq!(error_code, 0);

        let message = receiver.consume_message();
        assert_contains!(message, "usage: program [-h] [-f] SUB\n");
        assert_contains!(message, "SUB          {0, 1}");
        assert_contains!(message, "0            zero");
        assert_contains!(message, "1            one");
        assert_contains!(message, "-f, --flag");
    }

    #[test]
    fn sub0_command_build_help() {
        // Setup
        let mut flag: bool = false;
        let mut sub: u32 = 0;
        let mut items: Vec<u32> = Vec::default();
        let mut extra: bool = false;
        let clp = CommandLineParser::new("program");
        let scp = clp
            .add(Parameter::option(
                Switch::new(&mut flag, true),
                "flag",
                Some('f'),
            ))
            .branch(
                Condition::new(Scalar::new(&mut sub), "sub")
                    .choice(0, "zero")
                    .choice(1, "one"),
            )
            .command(0, |sub| sub)
            .command(1, |sub| {
                sub.add(Parameter::argument(
                    Collection::new(&mut items, Nargs::Any),
                    "item",
                ))
                .add(Parameter::option(
                    Switch::new(&mut extra, true),
                    "extra",
                    Some('e'),
                ))
            });
        let (sender, receiver) = channel_interface();

        // Execute
        let parser = scp.build_with_interface(Box::new(sender)).unwrap();

        // Verify
        // We testing that build sets up the right parser.
        // So the verification involves invoking the parser with --help and spot-checking the output.
        let error_code = parser.parse_tokens(&["0", "--help"]).unwrap_err();
        assert_eq!(error_code, 0);

        let message = receiver.consume_message();
        assert_contains!(message, "usage: program 0 [-h]\n");
    }

    #[test]
    fn sub1_command_build_help() {
        // Setup
        let mut flag: bool = false;
        let mut sub: u32 = 0;
        let mut items: Vec<u32> = Vec::default();
        let mut extra: bool = false;
        let clp = CommandLineParser::new("program");
        let scp = clp
            .add(Parameter::option(
                Switch::new(&mut flag, true),
                "flag",
                Some('f'),
            ))
            .branch(
                Condition::new(Scalar::new(&mut sub), "sub")
                    .choice(0, "zero")
                    .choice(1, "one"),
            )
            .command(0, |sub| sub)
            .command(1, |sub| {
                sub.add(Parameter::argument(
                    Collection::new(&mut items, Nargs::Any),
                    "item",
                ))
                .add(Parameter::option(
                    Switch::new(&mut extra, true),
                    "extra",
                    Some('e'),
                ))
            });
        let (sender, receiver) = channel_interface();

        // Execute
        let parser = scp.build_with_interface(Box::new(sender)).unwrap();

        // Verify
        // We testing that build sets up the right parser.
        // So the verification involves invoking the parser with --help and spot-checking the output.
        let error_code = parser.parse_tokens(&["1", "--help"]).unwrap_err();
        assert_eq!(error_code, 0);

        let message = receiver.consume_message();
        assert_contains!(message, "usage: program 1 [-h] [-e] [ITEM ...]\n");
        assert_contains!(message, "-e, --extra");
    }

    #[test]
    fn root_arguments_branch_build_help() {
        // Setup
        let mut flag: bool = false;
        let mut root: String = String::default();
        let mut sub: u32 = 0;
        let mut items: Vec<u32> = Vec::default();
        let clp = CommandLineParser::new("program");
        let scp = clp
            .add(Parameter::option(
                Switch::new(&mut flag, true),
                "flag",
                Some('f'),
            ))
            .add(Parameter::argument(Scalar::new(&mut root), "root"))
            .branch(Condition::new(Scalar::new(&mut sub), "sub"))
            .command(0, |sub| {
                sub.add(Parameter::argument(
                    Collection::new(&mut items, Nargs::Any),
                    "item0",
                ))
            });
        let (sender, receiver) = channel_interface();

        // Execute
        let parser = scp.build_with_interface(Box::new(sender)).unwrap();

        // Verify
        // We testing that build sets up the right parser.
        // So the verification involves invoking the parser with --help and spot-checking the output.
        let error_code = parser.parse_tokens(&["--help"]).unwrap_err();
        assert_eq!(error_code, 0);

        let message = receiver.consume_message();
        assert_contains!(message, "usage: program [-h] [-f] ROOT SUB\n");
    }

    #[test]
    #[cfg(feature = "unit_test")]
    fn test_dummies() {
        // Setup
        pub fn setup_fn<'a>(value: &'a mut u32) -> impl FnOnce(SubCommand<'a>) -> SubCommand<'a> {
            |sub| sub.add(Parameter::argument(Scalar::new(value), "value"))
        }

        let mut x: u32 = 1;
        let parser = setup_fn(&mut x)(SubCommand::test_dummy())
            .build_parser()
            .unwrap();
        let tokens = vec!["2"];

        // Execute
        parser.parse_tokens(tokens.as_slice()).unwrap();

        // Verify
        assert_eq!(x, 2);
    }

    #[derive(PartialEq)]
    enum Nefarious {
        Foo,
        Bar,
    }

    impl std::fmt::Display for Nefarious {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Nefarious::Foo => write!(f, "foo"),
                Nefarious::Bar => write!(f, "bar"),
            }
        }
    }

    impl std::str::FromStr for Nefarious {
        type Err = String;

        fn from_str(value: &str) -> Result<Self, Self::Err> {
            match value.to_lowercase().as_str() {
                "bar" => Ok(Nefarious::Foo),
                _ => Err(format!("unknown: {}", value)),
            }
        }
    }

    #[test]
    fn not_invertable_command() {
        // Setup
        let mut nefarious = Nefarious::Bar;
        let clp = CommandLineParser::new("program");
        let scp = clp
            .branch(Condition::new(Scalar::new(&mut nefarious), "abc"))
            .command(Nefarious::Foo, |sub| sub);
        let (sender, _receiver) = channel_interface();

        // Execute
        let result = scp.build_with_interface(Box::new(sender)).unwrap_err();

        // Verify
        assert_matches!(result, ConfigError(message) => {
            assert_eq!(message, "parameter 'abc' contains invalid sub-command 'foo': FromStr does not invert Display.".to_string());
        });
    }

    #[test]
    fn nefarious_command() {
        // Setup
        let mut nefarious = Nefarious::Bar;
        let clp = CommandLineParser::new("program");
        let scp = clp
            .branch(Condition::new(Scalar::new(&mut nefarious), "abc"))
            .command(Nefarious::Bar, |sub| sub);
        let (sender, _receiver) = channel_interface();

        // Execute
        let result = scp.build_with_interface(Box::new(sender)).unwrap_err();

        // Verify
        assert_matches!(result, ConfigError(message) => {
            assert_eq!(message, "parameter 'abc' contains invalid sub-command 'bar': FromStr does not invert Display.".to_string());
        });
    }
}
