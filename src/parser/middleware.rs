use std::collections::HashMap;
use std::env;
use std::rc::Rc;

use crate::parser::base::*;
use crate::parser::interface::UserInterface;
use crate::parser::printer::Printer;

pub struct GeneralParser<'ap> {
    program: String,
    command: ParseUnit<'ap>,
    sub_commands: HashMap<String, ParseUnit<'ap>>,
    user_interface: Rc<dyn UserInterface>,
}

impl<'ap> GeneralParser<'ap> {
    pub(crate) fn command(
        program: String,
        command: ParseUnit<'ap>,
        user_interface: Rc<dyn UserInterface>,
    ) -> Self {
        Self {
            program,
            command,
            sub_commands: HashMap::default(),
            user_interface,
        }
    }

    pub(crate) fn sub_command(
        program: String,
        command: ParseUnit<'ap>,
        sub_commands: HashMap<String, ParseUnit<'ap>>,
        user_interface: Rc<dyn UserInterface>,
    ) -> Self {
        Self {
            program,
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
    pub(crate) fn new(parser: Parser<'ap>, printer: Printer) -> Self {
        Self { parser, printer }
    }

    fn invoke(
        self,
        tokens: &[&str],
        program: String,
        user_interface: Rc<dyn UserInterface>,
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
                user_interface.print_error_context(tokens, offset);
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
                            .print_error_context(tokens, variant_offset);
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
