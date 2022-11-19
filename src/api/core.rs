use std::collections::HashMap;
use std::marker::PhantomData;

use crate::api::capture::*;
use crate::api::{Condition, Parameter, ParameterClass};
use crate::parser::{
    ArgumentCapture, ArgumentParameter, ConfigError, ConsoleInterface, GeneralParser,
    OptionCapture, UserInterface,
};
use crate::parser::{OptionParameter, ParseError, ParseUnit, Parser, Printer};

impl From<InvalidConversion> for ParseError {
    fn from(error: InvalidConversion) -> Self {
        ParseError(error.to_string())
    }
}

/// The base command line parser.
///
/// ### Example
/// ```
/// use blarg::{CommandLineParser};
///
/// let parser = CommandLineParser::new("program")
///     // Configure with CommandLineParser::add and CommandLineParser::branch.
///     .build()
///     .expect("The parser configuration must be valid (ex: no parameter name repeats).");
/// parser.parse_tokens(empty::slice()).unwrap();
/// ```
pub struct CommandLineParser<'ap> {
    program: String,
    option_parameters: Vec<OptionParameter>,
    argument_parameters: Vec<ArgumentParameter>,
    option_captures: Vec<OptionCapture<'ap>>,
    argument_captures: Vec<ArgumentCapture<'ap>>,
    discriminator: Option<String>,
}

impl<'ap> CommandLineParser<'ap> {
    /// Create a command line parser.
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

    /// Add an argument/option to the command line parser.
    ///
    /// The order of argument parameters corresponds to their positional order during parsing.
    /// The order of option parameters does not matter.
    ///
    /// ### Example
    /// ```no_run
    /// use blarg::{CommandLineParser, Parameter, Scalar};
    ///
    /// let mut a: u32 = 0;
    /// let mut b: u32 = 0;
    /// let parser = CommandLineParser::new("program")
    ///     .add(Parameter::argument(Scalar::new(&mut a), "a"))
    ///     .add(Parameter::argument(Scalar::new(&mut b), "b"))
    ///     .build()
    ///     .expect("The parser configuration must be valid (ex: no parameter name repeats).");
    ///
    /// parser.parse_tokens(vec!["1", "2"].as_slice()).unwrap();
    ///
    /// assert_eq!(a, 1);
    /// assert_eq!(b, 2);
    /// ```
    pub fn add<T>(mut self, parameter: Parameter<'ap, T>) -> Self {
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
    /// Any parameters added before the branch apply to the root parser, while subsequent parameters from this point forward apply to the sub-commands.
    ///
    /// Branching is always done with a `Scalar` `Parameter::argument` - aka: [`Condition`].
    ///
    /// ### Example
    /// ```
    /// use blarg::{CommandLineParser, Parameter, Scalar, Condition};
    ///
    /// let mut belongs_to_root: u32 = 0;
    /// let mut sub_command: String = "".to_string();
    /// let mut belongs_to_sub_command: u32 = 0;
    /// let parser = CommandLineParser::new("program")
    ///     .add(Parameter::argument(Scalar::new(&mut belongs_to_root), "belongs_to_root"))
    ///     .branch(Condition::new(Scalar::new(&mut sub_command), "sub_command"))
    ///     .add("the-command".to_string(), Parameter::argument(Scalar::new(&mut belongs_to_sub_command), "belongs_to_sub_command"))
    ///     .build()
    ///     .expect("The parser configuration must be valid (ex: no parameter name repeats).");
    ///
    /// parser.parse_tokens(vec!["1", "the-command", "2"].as_slice()).unwrap();
    ///
    /// assert_eq!(belongs_to_root, 1);
    /// assert_eq!(&sub_command, "the-command");
    /// assert_eq!(belongs_to_sub_command, 2);
    /// ```
    pub fn branch<T: std::str::FromStr + std::fmt::Display>(
        mut self,
        condition: Condition<'ap, T>,
    ) -> SubCommandParser<'ap, T> {
        let parameter = condition.consume();
        if self.discriminator.replace(parameter.name()).is_some() {
            unreachable!("internal error - cannot setup multiple discriminators");
        }

        SubCommandParser::new(self.add(parameter))
    }

    fn build_with_interface(
        self,
        user_interface: Box<dyn UserInterface>,
    ) -> Result<GeneralParser<'ap>, ConfigError> {
        let parser = Parser::new(
            self.option_captures,
            self.argument_captures,
            self.discriminator,
        )?;
        let command = ParseUnit::new(
            parser,
            Printer::new(self.option_parameters, self.argument_parameters),
        );
        Ok(GeneralParser::command(
            self.program,
            command,
            user_interface,
        ))
    }

    /// Build the command line parser.
    /// This finalizes the configuration and checks for errors (ex: a repeated parameter name).
    pub fn build(self) -> Result<GeneralParser<'ap>, ConfigError> {
        self.build_with_interface(Box::new(ConsoleInterface::default()))
    }
}

/// The sub-command parser.
pub struct SubCommandParser<'ap, B: std::fmt::Display> {
    root: CommandLineParser<'ap>,
    commands: HashMap<String, CommandLineParser<'ap>>,
    _phantom: PhantomData<B>,
}

impl<'ap, B: std::fmt::Display> SubCommandParser<'ap, B> {
    fn new(root: CommandLineParser<'ap>) -> Self {
        Self {
            root,
            commands: HashMap::default(),
            _phantom: PhantomData,
        }
    }

    /// Add an argument/option to the sub-command parser.
    ///
    /// The order of argument parameters corresponds to their positional order during parsing.
    /// The order of option parameters does not matter.
    ///
    /// ### Example
    /// ```no_run
    /// use blarg::{CommandLineParser, Condition, Parameter, Scalar};
    ///
    /// let mut value_a: u32 = 0;
    /// let mut value_b: u32 = 0;
    /// let mut sub_command: String = "".to_string();
    /// let parser = CommandLineParser::new("program")
    ///     .branch(Condition::new(Scalar::new(&mut sub_command), "sub_command"))
    ///     .add("a".to_string(), Parameter::argument(Scalar::new(&mut value_a), "value_a"))
    ///     .add("b".to_string(), Parameter::argument(Scalar::new(&mut value_b), "value_b"))
    ///     .build()
    ///     .expect("The parser configuration must be valid (ex: no parameter name repeats).");
    ///
    /// parser.parse_tokens(vec!["a", "1"].as_slice()).unwrap();
    ///
    /// assert_eq!(&sub_command, "a");
    /// assert_eq!(value_a, 1);
    /// assert_eq!(value_b, 0);
    /// ```
    pub fn add<T>(mut self, sub_command: B, parameter: Parameter<'ap, T>) -> Self {
        let command_str = sub_command.to_string();
        let clp = self
            .commands
            .remove(&command_str)
            .unwrap_or_else(|| CommandLineParser::new(command_str.clone()));
        self.commands.insert(command_str, clp.add(parameter));
        self
    }

    fn build_with_interface(
        self,
        user_interface: Box<dyn UserInterface>,
    ) -> Result<GeneralParser<'ap>, ConfigError> {
        let mut sub_commands = HashMap::default();

        for (discriminee, cp) in self.commands.into_iter() {
            let sub_parser = Parser::new(cp.option_captures, cp.argument_captures, None)?;
            let sub_command = ParseUnit::new(
                sub_parser,
                Printer::new(cp.option_parameters, cp.argument_parameters),
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
            Printer::new(self.root.option_parameters, self.root.argument_parameters),
        );
        Ok(GeneralParser::sub_command(
            self.root.program,
            command,
            sub_commands,
            user_interface,
        ))
    }

    /// Build the sub-command based command line parser.
    /// This finalizes the configuration and checks for errors (ex: a repeated parameter name).
    pub fn build(self) -> Result<GeneralParser<'ap>, ConfigError> {
        self.build_with_interface(Box::new(ConsoleInterface::default()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{Collection, Parameter, Scalar, Switch};
    use crate::model::Nargs;
    use crate::parser::util::channel_interface;
    use crate::test::assert_contains;
    use rstest::rstest;

    #[test]
    fn empty_build() {
        // Setup
        let clp = CommandLineParser::new("program");

        // Execute
        let parser = clp.build().unwrap();

        // Verify
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

        // Execute
        let parser = clp.build().unwrap();

        // Verify
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
            .add(
                0,
                Parameter::argument(Collection::new(&mut items_0, Nargs::Any), "item0"),
            )
            .add(
                1,
                Parameter::argument(Collection::new(&mut items_1, Nargs::Any), "item1"),
            );

        // Execute
        let parser = scp.build().unwrap();

        // Verify
        // We testing that build sets up the right parser.
        // So the verification involves invoking the parser with the various permutations.
        parser.parse_tokens(tokens.as_slice()).unwrap();
        assert_eq!(flag, expected_flag);
        assert_eq!(sub, expected_sub);
        assert_eq!(items_0, expected_items_0);
        assert_eq!(items_1, expected_items_1);
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
            .add(
                0,
                Parameter::argument(Collection::new(&mut items, Nargs::Any), "item0"),
            );

        // Execute
        let parser = scp.build().unwrap();

        // Verify
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
    }

    #[test]
    fn branch_build_help() {
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
            .branch(
                Condition::new(Scalar::new(&mut sub), "sub")
                    .choice(0, "zero")
                    .choice(1, "one"),
            )
            .add(
                0,
                Parameter::argument(Collection::new(&mut items_0, Nargs::Any), "item0"),
            )
            .add(
                1,
                Parameter::argument(Collection::new(&mut items_1, Nargs::Any), "item1"),
            );
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
        assert_contains!(message, "SUB         {0, 1}");
        assert_contains!(message, "0           zero");
        assert_contains!(message, "1           one");
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
            .add(
                0,
                Parameter::argument(Collection::new(&mut items, Nargs::Any), "item0"),
            );
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
}