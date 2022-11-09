use std::collections::HashMap;
use std::env;

use crate::parser::base::*;
use crate::parser::interface::UserInterface;
use crate::parser::printer::Printer;

pub struct GeneralParser<'ap> {
    program: String,
    command: ParseUnit<'ap>,
    sub_commands: HashMap<String, ParseUnit<'ap>>,
    user_interface: Box<dyn UserInterface>,
}

impl<'ap> GeneralParser<'ap> {
    pub(crate) fn command(
        program: impl Into<String>,
        command: ParseUnit<'ap>,
        user_interface: Box<dyn UserInterface>,
    ) -> Self {
        Self {
            program: program.into(),
            command,
            sub_commands: HashMap::default(),
            user_interface,
        }
    }

    pub(crate) fn sub_command(
        program: impl Into<String>,
        command: ParseUnit<'ap>,
        sub_commands: HashMap<String, ParseUnit<'ap>>,
        user_interface: Box<dyn UserInterface>,
    ) -> Self {
        Self {
            program: program.into(),
            command,
            sub_commands,
            user_interface,
        }
    }
}

pub(crate) struct ParseUnit<'ap> {
    parser: Parser<'ap>,
    printer: Printer,
}

impl<'ap> ParseUnit<'ap> {
    #[cfg(test)]
    pub(crate) fn empty() -> Self {
        Self::new(Parser::empty(), Printer::empty())
    }

    pub(crate) fn new(parser: Parser<'ap>, printer: Printer) -> Self {
        Self { parser, printer }
    }

    fn invoke(
        self,
        tokens: &[&str],
        program: impl Into<String>,
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
                printer.print_help(program, user_interface);
                ParseResult::Exit(0)
            }
            Err((offset, parse_error)) => {
                user_interface.print_error(parse_error);
                user_interface.print_error_context(offset, tokens);
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

impl<'ap> GeneralParser<'ap> {
    fn parse_tokens(mut self, tokens: &[&str]) -> Result<(), i32> {
        let command_result =
            self.command
                .invoke(tokens, self.program.clone(), &*self.user_interface);

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
                            &*self.user_interface,
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
                        // Either the user specified an invalid variant, OR
                        // the program invalidates the 'Display' inverse-to 'FromStr' / 'FromStr' inverse-to 'Display' requirement.
                        self.user_interface
                            .print_error(ParseError(format!("Unknown sub-command '{variant}'.")));
                        self.user_interface
                            .print_error_context(variant_offset, tokens);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{AnonymousCapture, GenericCapturable, Scalar};
    use crate::matcher::{ArgumentConfig, Bound, OptionConfig};
    use crate::parser::InMemory;
    use rstest::rstest;

    #[test]
    fn invoke_empty() {
        // Setup
        let parse_unit = ParseUnit::empty();
        let interface = InMemory::default();

        // Execute
        let result = parse_unit.invoke(empty::slice(), "program", &interface);

        // Verify
        assert_eq!(result, ParseResult::Complete);
        assert!((*interface.message.borrow()).is_none());
        assert!((*interface.error.borrow()).is_none());
        assert_eq!((*interface.error_context.borrow()), None);
    }

    #[rstest]
    #[case(vec!["1"])]
    #[case(vec!["01"])]
    #[case(vec!["--flag", "1"])]
    fn invoke(#[case] tokens: Vec<&str>) {
        // Setup
        let mut variable: u32 = 0;
        let generic_capture = Scalar::new(&mut variable);
        let config = ArgumentConfig::new("variable", generic_capture.nargs().into());
        let capture = AnonymousCapture::bind(generic_capture);
        let parse_unit = ParseUnit::new(
            Parser::new(
                vec![(
                    OptionConfig::new("flag", None, Bound::Range(0, 0)),
                    Box::new(BlackHole::default()),
                )],
                vec![(config, Box::new(capture))],
                None,
            )
            .unwrap(),
            Printer::empty(),
        );
        let interface = InMemory::default();

        // Execute
        let result = parse_unit.invoke(tokens.as_slice(), "program", &interface);

        // Verify
        assert_eq!(result, ParseResult::Complete);
        assert!((*interface.message.borrow()).is_none());
        assert!((*interface.error.borrow()).is_none());
        assert_eq!((*interface.error_context.borrow()), None);
    }

    #[rstest]
    #[case(vec!["1"], 0, "1", vec![])]
    #[case(vec!["01"], 0, "01", vec![])]
    #[case(vec!["--flag", "1"], 6, "1", vec![])]
    #[case(vec!["1", "a"], 0, "1", vec!["a"])]
    #[case(vec!["01", "a"], 0, "01", vec!["a"])]
    #[case(vec!["--flag", "1", "a"], 6, "1", vec!["a"])]
    #[case(vec!["1", "a", "--b=123"], 0, "1", vec!["a", "--b=123"])]
    #[case(vec!["01", "a", "--b=123"], 0, "01", vec!["a", "--b=123"])]
    #[case(vec!["--flag", "1", "a", "--b=123"], 6, "1", vec!["a", "--b=123"])]
    fn invoke_discriminator(
        #[case] tokens: Vec<&str>,
        #[case] offset: usize,
        #[case] discriminee: &str,
        #[case] remaining: Vec<&str>,
    ) {
        // Setup
        let mut variable: u32 = 0;
        let generic_capture = Scalar::new(&mut variable);
        let config = ArgumentConfig::new("variable", generic_capture.nargs().into());
        let capture = AnonymousCapture::bind(generic_capture);
        let parse_unit = ParseUnit::new(
            Parser::new(
                vec![(
                    OptionConfig::new("flag", None, Bound::Range(0, 0)),
                    Box::new(BlackHole::default()),
                )],
                vec![(config, Box::new(capture))],
                Some("variable".to_string()),
            )
            .unwrap(),
            Printer::empty(),
        );
        let interface = InMemory::default();

        // Execute
        let result = parse_unit.invoke(tokens.as_slice(), "program", &interface);

        // Verify
        assert_eq!(
            result,
            ParseResult::Incomplete {
                variant_offset: offset,
                variant: discriminee.to_string(),
                remaining: remaining.into_iter().map(|s| s.to_string()).collect(),
            }
        );
        assert!((*interface.message.borrow()).is_none());
        assert!((*interface.error.borrow()).is_none());
        assert_eq!((*interface.error_context.borrow()), None);
    }

    #[rstest]
    #[case(vec!["--help"])]
    #[case(vec!["-h"])]
    fn invoke_help(#[case] tokens: Vec<&str>) {
        // Setup
        let parse_unit = ParseUnit::empty();
        let interface = InMemory::default();

        // Execute
        let result = parse_unit.invoke(tokens.as_slice(), "program", &interface);

        // Verify
        assert_eq!(result, ParseResult::Exit(0));

        assert!((*interface.message.borrow())
            .as_ref()
            .unwrap()
            .contains("-h, --help"));
        assert!((*interface.error.borrow()).is_none());
        assert_eq!((*interface.error_context.borrow()), None);
    }

    #[test]
    fn invoke_argument_unmatched() {
        // Setup
        let parse_unit = ParseUnit::empty();
        let interface = InMemory::default();

        // Execute
        let result = parse_unit.invoke(&["unmatched"], "program", &interface);

        // Verify
        assert_eq!(result, ParseResult::Exit(1));

        assert!((*interface.message.borrow()).is_none());
        assert!((*interface.error.borrow())
            .as_ref()
            .unwrap()
            .contains("Parse error"));
        assert_eq!(
            (*interface.error_context.borrow()).as_ref().unwrap(),
            &(0, vec!["unmatched".to_string()])
        );
    }

    #[rstest]
    #[case(vec!["not-u32"], 0)]
    #[case(vec!["--flag", "not-u32"], 6)]
    fn invoke_argument_inconvertable(#[case] tokens: Vec<&str>, #[case] offset: usize) {
        // Setup
        let mut variable: u32 = 0;
        let generic_capture = Scalar::new(&mut variable);
        let config = ArgumentConfig::new("variable", generic_capture.nargs().into());
        let capture = AnonymousCapture::bind(generic_capture);
        let parse_unit = ParseUnit::new(
            Parser::new(
                vec![(
                    OptionConfig::new("flag", None, Bound::Range(0, 0)),
                    Box::new(BlackHole::default()),
                )],
                vec![(config, Box::new(capture))],
                None,
            )
            .unwrap(),
            Printer::empty(),
        );
        let interface = InMemory::default();

        // Execute
        let result = parse_unit.invoke(tokens.as_slice(), "program", &interface);

        // Verify
        assert_eq!(result, ParseResult::Exit(1));

        assert!((*interface.message.borrow()).is_none());
        assert!((*interface.error.borrow())
            .as_ref()
            .unwrap()
            .contains("Parse error"));
        assert_eq!(
            (*interface.error_context.borrow()).as_ref().unwrap(),
            &(offset, tokens.into_iter().map(|s| s.to_string()).collect())
        );
    }
}
