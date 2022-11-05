use std::collections::HashMap;
use std::marker::PhantomData;
use std::rc::Rc;

use crate::api::capture::*;
use crate::api::field::*;
use crate::matcher::{ArgumentConfig, Bound, OptionConfig};
use crate::model::Nargs;
use crate::parser::{
    AnonymousCapturable, ArgumentCapture, ArgumentParameter, ConfigError, OptionCapture,
    OptionParameter, ParseError, ParseUnit, Parser, Printer,
};
use crate::parser::{Console, GeneralParser};

enum ParameterInner<'ap, T> {
    Opt {
        field: AnonymousCapture<'ap, T>,
        nargs: Nargs,
        name: &'static str,
        short: Option<char>,
        description: Option<&'static str>,
    },
    Arg {
        field: AnonymousCapture<'ap, T>,
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
            unreachable!("internal error - argument must always be ParameterInner::Arg");
        }
    }
}

pub struct Parameter<'ap, T>(ParameterInner<'ap, T>);

impl<'ap, T> Parameter<'ap, T> {
    pub fn option(
        capture_field: impl GenericCapturable<'ap, T> + CliOption + 'ap,
        name: &'static str,
        short: Option<char>,
    ) -> Self {
        let nargs = capture_field.nargs();
        Self(ParameterInner::Opt {
            field: AnonymousCapture::bind(capture_field),
            nargs,
            name,
            short,
            description: None,
        })
    }

    pub fn argument(
        capture_field: impl GenericCapturable<'ap, T> + CliArgument + 'ap,
        name: &'static str,
    ) -> Self {
        let nargs = capture_field.nargs();
        Self(ParameterInner::Arg {
            field: AnonymousCapture::bind(capture_field),
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

pub(crate) struct AnonymousCapture<'ap, T: 'ap> {
    capture_field: Box<dyn GenericCapturable<'ap, T> + 'ap>,
}

impl<'ap, T> AnonymousCapture<'ap, T> {
    pub(crate) fn bind(capture_field: impl GenericCapturable<'ap, T> + 'ap) -> Self {
        Self {
            capture_field: Box::new(capture_field),
        }
    }
}

impl<'ap, T> AnonymousCapturable for AnonymousCapture<'ap, T> {
    fn matched(&mut self) {
        self.capture_field.matched();
    }

    fn capture(&mut self, value: &str) -> Result<(), ParseError> {
        self.capture_field.capture(value).map_err(ParseError::from)
    }
}

impl From<InvalidConversion> for ParseError {
    fn from(error: InvalidConversion) -> Self {
        ParseError(error.to_string())
    }
}

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
            unreachable!("internal error - cannot setup multiple discriminators");
        }

        SubCommandParser::new(self.add(condition.arg_parameter))
    }

    pub fn build(self) -> Result<GeneralParser<'ap>, ConfigError> {
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
            Rc::new(Console::default()),
        ))
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
            Rc::new(Console::default()),
        ))
    }
}
