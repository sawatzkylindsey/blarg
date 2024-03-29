use std::collections::HashMap;
use std::env;

use crate::parser::base::*;
use crate::parser::interface::UserInterface;
use crate::parser::printer::Printer;
use crate::parser::ErrorContext;

/// The configured command line parser.
/// Built via [`CommandLineParser::build`](./struct.CommandLineParser.html#method.build) or [`SubCommandParser::build`](./struct.SubCommandParser.html#method.build).
pub struct GeneralParser<'a> {
    command: ParseUnit<'a>,
    sub_commands: HashMap<String, ParseUnit<'a>>,
    user_interface: Box<dyn UserInterface>,
}

impl<'a> std::fmt::Debug for GeneralParser<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GeneralParser{..}").finish()
    }
}

impl<'a> GeneralParser<'a> {
    pub(crate) fn command(command: ParseUnit<'a>, user_interface: Box<dyn UserInterface>) -> Self {
        Self {
            command,
            sub_commands: HashMap::default(),
            user_interface,
        }
    }

    pub(crate) fn sub_command(
        command: ParseUnit<'a>,
        sub_commands: HashMap<String, ParseUnit<'a>>,
        user_interface: Box<dyn UserInterface>,
    ) -> Self {
        Self {
            command,
            sub_commands,
            user_interface,
        }
    }
}

pub(crate) struct ParseUnit<'a> {
    parser: Parser<'a>,
    printer: Printer,
}

impl<'a> ParseUnit<'a> {
    #[cfg(test)]
    pub(crate) fn empty() -> Self {
        Self::new(Parser::empty(), Printer::empty())
    }

    pub(crate) fn new(parser: Parser<'a>, printer: Printer) -> Self {
        Self { parser, printer }
    }

    fn invoke(
        self,
        tokens: &[&str],
        user_interface: &(impl UserInterface + ?Sized),
    ) -> ParseResult {
        let ParseUnit { parser, printer } = self;

        match parser.consume(tokens) {
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
                printer.print_help(user_interface);
                ParseResult::Exit(0)
            }
            Err((offset, parse_error)) => {
                user_interface.print_error(parse_error);
                user_interface.print_error_context(ErrorContext::new(offset, tokens));
                ParseResult::Exit(1)
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum ParseResult {
    Complete,
    Incomplete {
        variant_offset: usize,
        variant: String,
        remaining: Vec<String>,
    },
    Exit(i32),
}

impl<'a> GeneralParser<'a> {
    #[cfg(test)]
    pub fn details(&self) -> (String, Option<String>) {
        (
            self.command.printer.program.clone(),
            self.command.printer.about.clone(),
        )
    }

    #[cfg(test)]
    pub fn sub_details(&self, variant: &str) -> Option<(String, Option<String>)> {
        self.sub_commands.get(variant).map(|parse_unit| {
            (
                parse_unit.printer.program.clone(),
                parse_unit.printer.about.clone(),
            )
        })
    }

    /// Run the command line parser against the input tokens.
    /// Help messages are printed on `stdout`, while error messages are printed on `stderr`.
    ///
    /// The parser will process the input tokens based off the [`CommandLineParser`](./struct.CommandLineParser.html)/[`SubCommandParser`](./struct.SubCommandParser.html) configuration.
    /// Parsing happens in two phases:
    /// 1. Token matching aligns the tokens to arguments and options.
    /// All tokens must be matched successfully in order to proceed to the next phase.
    /// 2. Token capturing parses the tokens by their respective types `T`.
    /// This phase will actually mutate your program variables.
    ///
    /// If at any point the parser encounters an error (ex: un-matched token, un-capturable token, etc), it will return with `Err(1)`.
    ///
    /// If the help switch (`-h` or `--help`) is encountered, the parser will display the help message and return with `Err(0)`.
    /// This skips the phase #2 capturing.
    ///
    /// In the case of a sub-command based parser, a third phase is introduced where the parser is branched into the sub-command.
    /// After branching, the token matching and token capturing phases are repeated for the sub-command.
    /// In effect, the input tokens are partitioned based off the branching `Condition`.
    pub fn parse_tokens(self, tokens: &[&str]) -> Result<(), i32> {
        let GeneralParser {
            command,
            mut sub_commands,
            user_interface,
        } = self;
        let command_result = command.invoke(tokens, &*user_interface);

        match command_result {
            ParseResult::Complete => Ok(()),
            ParseResult::Incomplete {
                variant_offset,
                variant,
                remaining,
            } => {
                match sub_commands.remove(&variant) {
                    Some(sub_command) => {
                        match sub_command.invoke(
                            remaining
                                .iter()
                                .map(AsRef::as_ref)
                                .collect::<Vec<&str>>()
                                .as_slice(),
                            &*user_interface,
                        ) {
                            ParseResult::Complete => Ok(()),
                            ParseResult::Incomplete { .. } => {
                                unreachable!(
                                    "internal error - sub-command parse must complete/exit."
                                )
                            }
                            ParseResult::Exit(code) => Err(code),
                        }
                    }
                    None => {
                        // The variant isn't amongst the sub-commands.
                        user_interface.print_error(ParseError::BranchingPhase(format!(
                            "unknown sub-command '{variant}'."
                        )));
                        user_interface
                            .print_error_context(ErrorContext::new(variant_offset, tokens));
                        Err(1)
                    }
                }
            }
            ParseResult::Exit(code) => Err(code),
        }
    }

    /// Run the command line parser against the Cli [`env::args`].
    /// Help messages are printed on `stdout`, while error messages are printed on `stderr`.
    ///
    /// The parser will process the input tokens based off the [`CommandLineParser`](./struct.CommandLineParser.html)/[`SubCommandParser`](./struct.SubCommandParser.html) configuration.
    /// Parsing happens in two phases:
    /// 1. Token matching aligns the tokens to arguments and options.
    /// All tokens must be matched successfully in order to proceed to the next phase.
    /// 2. Token capturing parses the tokens by their respective types `T`.
    /// This phase will actually mutate your program variables.
    ///
    /// If at any point the parser encounters an error (ex: un-matched token, un-capturable token, etc), it will exit with error code `1` (via [`std::process::exit`]).
    ///
    /// If the help switch (`-h` or `--help`) is encountered, the parser will display the help message and exit with error code `0`.
    /// This skips the phase #2 capturing.
    ///
    /// In the case of a sub-command based parser, a third phase is introduced where the parser is branched into the sub-command.
    /// After branching, the token matching and token capturing phases are repeated for the sub-command.
    /// In effect, the input tokens are partitioned based off the branching `Condition`.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{AnonymousCapture, GenericCapturable, Scalar};
    use crate::matcher::{ArgumentConfig, Bound, OptionConfig};
    use crate::parser::test::BlackHole;
    use crate::parser::util::{channel_interface, InMemoryInterface};
    use crate::test::assert_contains;
    use rstest::rstest;

    #[rstest]
    #[case(vec!["1"], 0, "1", vec![])]
    #[case(vec!["01"], 0, "01", vec![])]
    #[case(vec!["--flag", "1"], 6, "1", vec![])]
    #[case(vec!["1", "a"], 0, "1", vec!["a"])]
    #[case(vec!["01", "a"], 0, "01", vec!["a"])]
    #[case(vec!["--flag", "1", "a"], 6, "1", vec!["a"])]
    #[case(vec!["1", "a", "--abc=123"], 0, "1", vec!["a", "--abc=123"])]
    #[case(vec!["01", "a", "--abc=123"], 0, "01", vec!["a", "--abc=123"])]
    #[case(vec!["--flag", "1", "a", "--abc=123"], 6, "1", vec!["a", "--abc=123"])]
    fn invoke_discriminator(
        #[case] tokens: Vec<&str>,
        #[case] offset: usize,
        #[case] discriminee: &str,
        #[case] remaining: Vec<&str>,
    ) {
        // Setup
        let config = ArgumentConfig::new("variable", Bound::Range(1, 1));
        let parse_unit = ParseUnit::new(
            Parser::new(
                vec![(
                    OptionConfig::new("flag", None, Bound::Range(0, 0)),
                    Box::new(BlackHole::default()),
                )],
                vec![(config, Box::new(BlackHole::default()))],
                Some("variable".to_string()),
            )
            .unwrap(),
            Printer::empty(),
        );
        let interface = InMemoryInterface::default();

        // Execute
        let result = parse_unit.invoke(tokens.as_slice(), &interface);

        // Verify
        assert_eq!(
            result,
            ParseResult::Incomplete {
                variant_offset: offset,
                variant: discriminee.to_string(),
                remaining: remaining.into_iter().map(|s| s.to_string()).collect(),
            }
        );

        let (message, error, error_context) = interface.consume();
        assert_eq!(message, None);
        assert_eq!(error, None);
        assert_eq!(error_context, None);
    }

    #[test]
    fn parse_tokens_empty() {
        // Setup
        let (sender, receiver) = channel_interface();
        let general_parser = GeneralParser::command(ParseUnit::empty(), Box::new(sender));

        // Execute
        general_parser.parse_tokens(empty::slice()).unwrap();

        // Verify
        let (message, error, error_context) = receiver.consume();
        assert_eq!(message, None);
        assert_eq!(error, None);
        assert_eq!(error_context, None);
    }

    #[rstest]
    #[case(vec!["1"])]
    #[case(vec!["01"])]
    #[case(vec!["--flag", "1"])]
    fn parse_tokens(#[case] tokens: Vec<&str>) {
        // Setup
        let parse_unit = ParseUnit::new(
            Parser::new(
                vec![(
                    OptionConfig::new("flag", None, Bound::Range(0, 0)),
                    Box::new(BlackHole::default()),
                )],
                vec![(
                    ArgumentConfig::new("variable", Bound::Range(1, 1)),
                    Box::new(BlackHole::default()),
                )],
                None,
            )
            .unwrap(),
            Printer::empty(),
        );
        let (sender, receiver) = channel_interface();
        let general_parser = GeneralParser::command(parse_unit, Box::new(sender));

        // Execute
        general_parser.parse_tokens(tokens.as_slice()).unwrap();

        // Verify
        let (message, error, error_context) = receiver.consume();
        assert_eq!(message, None);
        assert_eq!(error, None);
        assert_eq!(error_context, None);
    }

    #[rstest]
    #[case(vec!["--help"])]
    #[case(vec!["-h"])]
    fn parse_tokens_help(#[case] tokens: Vec<&str>) {
        // Setup
        let parse_unit = ParseUnit::empty();
        let (sender, receiver) = channel_interface();
        let general_parser = GeneralParser::command(parse_unit, Box::new(sender));

        // Execute
        let error_code = general_parser.parse_tokens(tokens.as_slice()).unwrap_err();

        // Verify
        assert_eq!(error_code, 0);

        let message = receiver.consume_message();
        assert_contains!(message, "usage: EMPTY [-h]");
        assert_contains!(message, "-h, --help");
    }

    #[rstest]
    #[case(vec!["not-u32"], 0)]
    #[case(vec!["--flag", "not-u32"], 6)]
    fn parse_tokens_argument_inconvertable(#[case] tokens: Vec<&str>, #[case] offset: usize) {
        // Setup
        let mut variable: u32 = 0;
        let generic_capture = Scalar::new(&mut variable);
        let parse_unit = ParseUnit::new(
            Parser::new(
                vec![(
                    OptionConfig::new("flag", None, Bound::Range(0, 0)),
                    Box::new(BlackHole::default()),
                )],
                vec![(
                    ArgumentConfig::new("variable", generic_capture.nargs().into()),
                    Box::new(AnonymousCapture::bind(generic_capture)),
                )],
                None,
            )
            .unwrap(),
            Printer::empty(),
        );
        let (sender, receiver) = channel_interface();
        let general_parser = GeneralParser::command(parse_unit, Box::new(sender));

        // Execute
        let error_code = general_parser.parse_tokens(tokens.as_slice()).unwrap_err();

        // Verify
        assert_eq!(error_code, 1);

        let (message, error, error_context) = receiver.consume();
        assert_eq!(message, None);
        let error = error.unwrap();
        assert_contains!(error, "Parse error");
        let error_context = error_context.unwrap();
        assert_eq!(error_context, ErrorContext::new(offset, &tokens));
    }

    #[rstest]
    #[case(vec!["1"])]
    #[case(vec!["--flag", "1"])]
    fn sub_command_empty(#[case] tokens: Vec<&str>) {
        // Setup
        let parse_unit = ParseUnit::new(
            Parser::new(
                vec![(
                    OptionConfig::new("flag", None, Bound::Range(0, 0)),
                    Box::new(BlackHole::default()),
                )],
                vec![(
                    ArgumentConfig::new("variable", Bound::Range(1, 1)),
                    Box::new(BlackHole::default()),
                )],
                Some("variable".to_string()),
            )
            .unwrap(),
            Printer::empty(),
        );
        let sub_commands = HashMap::from([("1".to_string(), ParseUnit::empty())]);
        let (sender, receiver) = channel_interface();
        let general_parser = GeneralParser::sub_command(parse_unit, sub_commands, Box::new(sender));

        // Execute
        general_parser.parse_tokens(tokens.as_slice()).unwrap();

        // Verify
        let (message, error, error_context) = receiver.consume();
        assert_eq!(message, None);
        assert_eq!(error, None);
        assert_eq!(error_context, None);
    }

    #[rstest]
    #[case(vec!["1", "a"])]
    #[case(vec!["--flag", "1", "a"])]
    #[case(vec!["1", "a", "--abc=123"])]
    #[case(vec!["--flag", "1", "a", "--abc=123"])]
    fn sub_command(#[case] tokens: Vec<&str>) {
        // Setup
        let parse_unit = ParseUnit::new(
            Parser::new(
                vec![(
                    OptionConfig::new("flag", None, Bound::Range(0, 0)),
                    Box::new(BlackHole::default()),
                )],
                vec![(
                    ArgumentConfig::new("variable", Bound::Range(1, 1)),
                    Box::new(BlackHole::default()),
                )],
                Some("variable".to_string()),
            )
            .unwrap(),
            Printer::empty(),
        );
        let sub_commands = HashMap::from([(
            "1".to_string(),
            ParseUnit::new(
                Parser::new(
                    vec![(
                        OptionConfig::new("abc", None, Bound::Range(1, 1)),
                        Box::new(BlackHole::default()),
                    )],
                    vec![(
                        ArgumentConfig::new("item", Bound::Range(1, 1)),
                        Box::new(BlackHole::default()),
                    )],
                    None,
                )
                .unwrap(),
                Printer::empty(),
            ),
        )]);
        let (sender, receiver) = channel_interface();
        let general_parser = GeneralParser::sub_command(parse_unit, sub_commands, Box::new(sender));

        // Execute
        general_parser.parse_tokens(tokens.as_slice()).unwrap();

        // Verify
        let (message, error, error_context) = receiver.consume();
        assert_eq!(message, None);
        assert_eq!(error, None);
        assert_eq!(error_context, None);
    }

    #[rstest]
    #[case(vec!["1", "--help"])]
    #[case(vec!["--flag", "1", "--help"])]
    #[case(vec!["1", "-h"])]
    #[case(vec!["--flag", "1", "-h"])]
    fn sub_command_help(#[case] tokens: Vec<&str>) {
        // Setup
        let parse_unit = ParseUnit::new(
            Parser::new(
                vec![(
                    OptionConfig::new("flag", None, Bound::Range(0, 0)),
                    Box::new(BlackHole::default()),
                )],
                vec![(
                    ArgumentConfig::new("variable", Bound::Range(1, 1)),
                    Box::new(BlackHole::default()),
                )],
                Some("variable".to_string()),
            )
            .unwrap(),
            Printer::empty(),
        );
        let sub_commands = HashMap::from([("1".to_string(), ParseUnit::empty())]);
        let (sender, receiver) = channel_interface();
        let general_parser = GeneralParser::sub_command(parse_unit, sub_commands, Box::new(sender));

        // Execute
        let error_code = general_parser.parse_tokens(tokens.as_slice()).unwrap_err();

        // Verify
        assert_eq!(error_code, 0);

        let message = receiver.consume_message();
        assert_contains!(message, "usage: EMPTY [-h]");
        assert_contains!(message, "-h, --help");
    }

    #[rstest]
    #[case(vec!["1", "not-u32"], 0, vec!["not-u32"])]
    #[case(vec!["--flag", "1", "not-u32"], 0, vec!["not-u32"])]
    #[case(vec!["1", "--abc=123", "not-u32"], 9, vec!["--abc=123", "not-u32"])]
    #[case(vec!["--flag", "1", "--abc=123", "not-u32"], 9, vec!["--abc=123", "not-u32"])]
    fn sub_command_inconvertable(
        #[case] tokens: Vec<&str>,
        #[case] offset: usize,
        #[case] context: Vec<&str>,
    ) {
        // Setup
        let parse_unit = ParseUnit::new(
            Parser::new(
                vec![(
                    OptionConfig::new("flag", None, Bound::Range(0, 0)),
                    Box::new(BlackHole::default()),
                )],
                vec![(
                    ArgumentConfig::new("variable", Bound::Range(1, 1)),
                    Box::new(BlackHole::default()),
                )],
                Some("variable".to_string()),
            )
            .unwrap(),
            Printer::empty(),
        );
        let mut item: u32 = 0;
        let generic_capture = Scalar::new(&mut item);
        let sub_commands = HashMap::from([(
            "1".to_string(),
            ParseUnit::new(
                Parser::new(
                    vec![(
                        OptionConfig::new("abc", None, Bound::Range(1, 1)),
                        Box::new(BlackHole::default()),
                    )],
                    vec![(
                        ArgumentConfig::new("item", generic_capture.nargs().into()),
                        Box::new(AnonymousCapture::bind(generic_capture)),
                    )],
                    None,
                )
                .unwrap(),
                Printer::empty(),
            ),
        )]);
        let (sender, receiver) = channel_interface();
        let general_parser = GeneralParser::sub_command(parse_unit, sub_commands, Box::new(sender));

        // Execute
        let error_code = general_parser.parse_tokens(tokens.as_slice()).unwrap_err();

        // Verify
        assert_eq!(error_code, 1);

        let (message, error, error_context) = receiver.consume();
        assert_eq!(message, None);
        let error = error.unwrap();
        assert_contains!(error, "Parse error");
        let error_context = error_context.unwrap();
        assert_eq!(error_context, ErrorContext::new(offset, &context));
    }

    #[rstest]
    #[case(vec!["1"], 0)]
    #[case(vec!["01"], 0)]
    #[case(vec!["--flag", "1"], 6)]
    fn sub_command_not_found(#[case] tokens: Vec<&str>, #[case] offset: usize) {
        // Setup
        let parse_unit = ParseUnit::new(
            Parser::new(
                vec![(
                    OptionConfig::new("flag", None, Bound::Range(0, 0)),
                    Box::new(BlackHole::default()),
                )],
                vec![(
                    ArgumentConfig::new("variable", Bound::Range(1, 1)),
                    Box::new(BlackHole::default()),
                )],
                Some("variable".to_string()),
            )
            .unwrap(),
            Printer::empty(),
        );
        let sub_commands = HashMap::default();
        let (sender, receiver) = channel_interface();
        let general_parser = GeneralParser::sub_command(parse_unit, sub_commands, Box::new(sender));

        // Execute
        let error_code = general_parser.parse_tokens(tokens.as_slice()).unwrap_err();

        // Verify
        assert_eq!(error_code, 1);

        let (message, error, error_context) = receiver.consume();
        assert_eq!(message, None);
        let error = error.unwrap();
        assert_contains!(error, "unknown sub-command");
        let error_context = error_context.unwrap();
        assert_eq!(error_context, ErrorContext::new(offset, &tokens));
    }
}
