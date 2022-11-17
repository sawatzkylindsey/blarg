use crate::api::{CliArgument, CliOption, GenericCapturable, Scalar};
use crate::matcher::{ArgumentConfig, Bound, OptionConfig};
use crate::model::Nargs;
use crate::parser::{
    AnonymousCapturable, ArgumentCapture, ArgumentParameter, OptionCapture, OptionParameter,
    ParseError,
};
use std::collections::HashMap;

pub(crate) struct AnonymousCapture<'ap, T: 'ap> {
    field: Box<dyn GenericCapturable<'ap, T> + 'ap>,
}

impl<'ap, T> AnonymousCapture<'ap, T> {
    pub(crate) fn bind(field: impl GenericCapturable<'ap, T> + 'ap) -> Self {
        Self {
            field: Box::new(field),
        }
    }
}

impl<'ap, T> AnonymousCapturable for AnonymousCapture<'ap, T> {
    fn matched(&mut self) {
        self.field.matched();
    }

    fn capture(&mut self, value: &str) -> Result<(), ParseError> {
        self.field.capture(value).map_err(ParseError::from)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ParameterClass {
    Opt,
    Arg,
}

pub(super) struct ParameterInner<'ap, T> {
    class: ParameterClass,
    field: AnonymousCapture<'ap, T>,
    nargs: Nargs,
    name: String,
    short: Option<char>,
    description: Option<String>,
    choices: HashMap<String, String>,
}

impl<'ap, T> ParameterInner<'ap, T> {
    pub(super) fn class(&self) -> ParameterClass {
        self.class
    }
}

impl<'ap, T> std::fmt::Debug for ParameterInner<'ap, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let class = match &self.class {
            ParameterClass::Opt => "Opt",
            ParameterClass::Arg => "Arg",
        };
        let name = match &self.class {
            ParameterClass::Opt => format!("--{n}", n = self.name),
            ParameterClass::Arg => format!("{n}", n = self.name),
        };
        let short = match &self.class {
            ParameterClass::Opt => match &self.short {
                Some(s) => format!(" -{s},"),
                None => "".to_string(),
            },
            ParameterClass::Arg => "".to_string(),
        };
        let description = if let Some(d) = &self.description {
            format!(", {d}")
        } else {
            "".to_string()
        };

        write!(
            f,
            "{class}[{t}, {nargs}, {name},{short} {description}]",
            t = std::any::type_name::<T>(),
            nargs = self.nargs,
        )
    }
}

impl<'ap, T> From<&ParameterInner<'ap, T>> for OptionConfig {
    fn from(value: &ParameterInner<'ap, T>) -> Self {
        OptionConfig::new(
            value.name.clone(),
            value.short.clone(),
            Bound::from(value.nargs),
        )
    }
}

impl<'ap, T> From<ParameterInner<'ap, T>> for OptionCapture<'ap> {
    fn from(value: ParameterInner<'ap, T>) -> Self {
        let config = OptionConfig::from(&value);
        let ParameterInner { field, .. } = value;
        (config, Box::new(field))
    }
}

impl<'ap, T> From<&ParameterInner<'ap, T>> for OptionParameter {
    fn from(value: &ParameterInner<'ap, T>) -> Self {
        OptionParameter::new(
            value.name.clone(),
            value.short.clone(),
            value.nargs,
            value.description.clone(),
            value.choices.clone(),
        )
    }
}

impl<'ap, T> From<&ParameterInner<'ap, T>> for ArgumentConfig {
    fn from(value: &ParameterInner<'ap, T>) -> Self {
        ArgumentConfig::new(value.name.clone(), Bound::from(value.nargs))
    }
}

impl<'ap, T> From<ParameterInner<'ap, T>> for ArgumentCapture<'ap> {
    fn from(value: ParameterInner<'ap, T>) -> Self {
        let config = ArgumentConfig::from(&value);
        let ParameterInner { field, .. } = value;
        (config, Box::new(field))
    }
}

impl<'ap, T> From<&ParameterInner<'ap, T>> for ArgumentParameter {
    fn from(value: &ParameterInner<'ap, T>) -> Self {
        ArgumentParameter::new(
            value.name.clone(),
            value.nargs,
            value.description.clone(),
            value.choices.clone(),
        )
    }
}

/// The condition argument with which to branch the parser.
/// Used with `CommandParser::branch`.
pub struct Condition<'ap, T>(Parameter<'ap, T>);

impl<'ap, T: std::str::FromStr + std::fmt::Display> Condition<'ap, T> {
    /// Create a condition parameter.
    ///
    /// ### Example
    /// ```
    /// use blarg::{Condition, Scalar};
    /// use std::str::FromStr;
    ///
    /// // Be sure to implement `std::fmt::Display` and `std::str::FromStr`.
    /// enum FooBar {
    ///     Foo,
    ///     Bar,
    /// }
    /// # impl std::fmt::Display for FooBar {
    /// #     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    /// #         match self {
    /// #              FooBar::Foo => write!(f, "foo"),
    /// #             FooBar::Bar => write!(f, "bar"),
    /// #         }
    /// #     }
    /// # }
    /// # impl FromStr for FooBar {
    /// #     type Err = String;
    /// #
    /// #     fn from_str(value: &str) -> Result<Self, Self::Err> {
    /// #         match value.to_lowercase().as_str() {
    /// #             "foo" => Ok(FooBar::Foo),
    /// #             "bar" => Ok(FooBar::Bar),
    /// #             _ => Err(format!("unknown: {}", value)),
    /// #         }
    /// #     }
    /// # }
    ///
    /// let mut foo_bar: FooBar = FooBar::Foo;
    /// Condition::new(Scalar::new(&mut foo_bar), "foo_bar");
    /// // .. parse()
    /// match foo_bar {
    ///     FooBar::Foo => println!("Do foo'y things."),
    ///     FooBar::Bar => println!("Do bar'y things."),
    /// };
    /// ```
    pub fn new(value: Scalar<'ap, T>, name: &'static str) -> Self {
        Condition(Parameter::argument(value, name))
    }

    /// Document a choice in the help message for the sub-command condition.
    /// If repeated for the same `variant`, only the final message will apply to the sub-command condition.
    /// Repeat using different variants to document multiple choices.
    /// Needn't be exhaustive.
    ///
    /// Notice, the documented or un-documented choices *do not* affect the command parser semantics.
    ///
    /// ### Example
    /// ```
    /// use blarg::{Condition, Scalar};
    /// use std::str::FromStr;
    ///
    /// // Be sure to implement `std::fmt::Display` and `std::str::FromStr`.
    /// enum FooBar {
    ///     Foo,
    ///     Bar,
    /// }
    /// # impl std::fmt::Display for FooBar {
    /// #     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    /// #         match self {
    /// #              FooBar::Foo => write!(f, "foo"),
    /// #             FooBar::Bar => write!(f, "bar"),
    /// #         }
    /// #     }
    /// # }
    /// # impl FromStr for FooBar {
    /// #     type Err = String;
    /// #
    /// #     fn from_str(value: &str) -> Result<Self, Self::Err> {
    /// #         match value.to_lowercase().as_str() {
    /// #             "foo" => Ok(FooBar::Foo),
    /// #             "bar" => Ok(FooBar::Bar),
    /// #             _ => Err(format!("unknown: {}", value)),
    /// #         }
    /// #     }
    /// # }
    ///
    /// let mut foo_bar: FooBar = FooBar::Foo;
    /// Condition::new(Scalar::new(&mut foo_bar), "foo_bar")
    ///     .choice(FooBar::Foo, "--this will get discarded--")
    ///     .choice(FooBar::Foo, "Do foo'y things.")
    ///     .choice(FooBar::Bar, "Do bar'y things.");
    /// ```
    pub fn choice(self, variant: T, description: impl Into<String>) -> Self {
        let inner = self.0;
        Self(inner.choice(variant, description))
    }

    /// Document the help message for this sub-command condition.
    /// If repeated, only the final message will apply to the sub-command condition.
    ///
    /// ### Example
    /// ```
    /// use blarg::{Condition, Scalar};
    ///
    /// let mut case: u32 = 0;
    /// Condition::new(Scalar::new(&mut case), "case")
    ///     .help("--this will get discarded--")
    ///     .help("Choose the 'case' to execute.");
    /// ```
    pub fn help(self, description: impl Into<String>) -> Self {
        let inner = self.0;
        Self(inner.help(description))
    }

    pub(super) fn consume(self) -> Parameter<'ap, T> {
        self.0
    }
}

/// An argument/option for the `CommandParser`.
/// Used with `CommandParser::add` and `SubCommandParser::add`.
pub struct Parameter<'ap, T>(ParameterInner<'ap, T>);

impl<'ap, T> Parameter<'ap, T> {
    /// Create an option parameter.
    ///
    /// ### Example
    /// ```
    /// use blarg::{Parameter, Switch};
    ///
    /// let mut verbose: bool = false;
    /// Parameter::option(Switch::new(&mut verbose, true), "verbose", Some('v'));
    /// ```
    pub fn option(
        field: impl GenericCapturable<'ap, T> + CliOption + 'ap,
        name: impl Into<String>,
        short: Option<char>,
    ) -> Self {
        let nargs = field.nargs();
        Self(ParameterInner {
            class: ParameterClass::Opt,
            field: AnonymousCapture::bind(field),
            nargs,
            name: name.into(),
            short,
            description: None,
            choices: HashMap::default(),
        })
    }

    /// Create an argument parameter.
    ///
    /// ### Example
    /// ```
    /// use blarg::{Parameter, Scalar};
    ///
    /// let mut verbose: bool = false;
    /// Parameter::argument(Scalar::new(&mut verbose), "verbose");
    /// ```
    pub fn argument(
        field: impl GenericCapturable<'ap, T> + CliArgument + 'ap,
        name: impl Into<String>,
    ) -> Self {
        let nargs = field.nargs();
        Self(ParameterInner {
            class: ParameterClass::Arg,
            field: AnonymousCapture::bind(field),
            nargs,
            name: name.into(),
            short: None,
            description: None,
            choices: HashMap::default(),
        })
    }

    /// Document the help message for this parameter.
    /// If repeated, only the final message will apply to the parameter.
    ///
    /// ### Example
    /// ```
    /// use blarg::{Parameter, Scalar};
    ///
    /// let mut verbose: bool = false;
    /// Parameter::argument(Scalar::new(&mut verbose), "verbose")
    ///     .help("--this will get discarded--")
    ///     .help("Make the program output verbose.");
    /// ```
    pub fn help(self, description: impl Into<String>) -> Self {
        let mut inner = self.0;
        inner.description = Some(description.into());
        Self(inner)
    }

    pub(super) fn name(&self) -> String {
        self.0.name.clone()
    }

    pub(super) fn consume(self) -> ParameterInner<'ap, T> {
        self.0
    }
}

impl<'ap, T: std::fmt::Display> Parameter<'ap, T> {
    /// Document a choice in the help message for this parameter.
    /// If repeated for the same `variant`, only the final message will apply to the parameter.
    /// Repeat using different variants to document multiple choices.
    /// Needn't be exhaustive.
    ///
    /// Notice, the documented or un-documented choices *do not* affect the command parser semantics.
    ///
    /// ### Example
    /// ```
    /// use blarg::{Parameter, Scalar};
    /// use std::str::FromStr;
    ///
    /// let mut door: u32 = 0;
    /// Parameter::argument(Scalar::new(&mut door), "door")
    ///     .choice(1, "--this will get discarded--")
    ///     .choice(1, "Enter door #1.")
    ///     .choice(2, "Enter door #2.");
    /// ```
    pub fn choice(self, variant: T, description: impl Into<String>) -> Self {
        let mut inner = self.0;
        inner
            .choices
            .insert(variant.to_string(), description.into());
        Self(inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{Parameter, Switch};

    #[test]
    fn option() {
        let mut flag: bool = false;
        let option = Parameter::option(Switch::new(&mut flag, true), "flag", None).consume();

        assert_eq!(option.class, ParameterClass::Opt);
        assert_eq!(option.name, "flag");
        assert_eq!(option.short, None);
        assert_eq!(option.description, None);
        assert_eq!(option.choices, HashMap::default());
    }

    #[test]
    fn option_short() {
        let mut flag: bool = false;
        let option = Parameter::option(Switch::new(&mut flag, true), "flag", Some('f')).consume();

        assert_eq!(option.class, ParameterClass::Opt);
        assert_eq!(option.name, "flag");
        assert_eq!(option.short, Some('f'));
        assert_eq!(option.description, None);
        assert_eq!(option.choices, HashMap::default());
    }

    #[test]
    fn option_help() {
        let mut flag: bool = false;
        let option = Parameter::option(Switch::new(&mut flag, true), "flag", None)
            .help("help message")
            .consume();

        assert_eq!(option.class, ParameterClass::Opt);
        assert_eq!(option.name, "flag".to_string());
        assert_eq!(option.short, None);
        assert_eq!(option.description, Some("help message".to_string()));
        assert_eq!(option.choices, HashMap::default());
    }

    #[test]
    fn option_choice() {
        let mut flag: bool = false;
        let option = Parameter::option(Switch::new(&mut flag, true), "flag", None)
            .choice(true, "b")
            .choice(false, "d")
            .choice(true, "e")
            .consume();

        assert_eq!(option.class, ParameterClass::Opt);
        assert_eq!(option.name, "flag".to_string());
        assert_eq!(option.short, None);
        assert_eq!(option.description, None);
        assert_eq!(
            option.choices,
            HashMap::from([
                ("true".to_string(), "e".to_string()),
                ("false".to_string(), "d".to_string())
            ])
        );
    }

    #[test]
    fn argument() {
        let mut item: bool = false;
        let argument = Parameter::argument(Scalar::new(&mut item), "item").consume();

        assert_eq!(argument.class, ParameterClass::Arg);
        assert_eq!(argument.name, "item".to_string());
        assert_eq!(argument.short, None);
        assert_eq!(argument.description, None);
        assert_eq!(argument.choices, HashMap::default());
    }

    #[test]
    fn argument_help() {
        let mut item: bool = false;
        let argument = Parameter::argument(Scalar::new(&mut item), "item")
            .help("help message")
            .consume();

        assert_eq!(argument.class, ParameterClass::Arg);
        assert_eq!(argument.name, "item".to_string());
        assert_eq!(argument.short, None);
        assert_eq!(argument.description, Some("help message".to_string()));
        assert_eq!(argument.choices, HashMap::default());
    }

    #[test]
    fn argument_choice() {
        let mut item: bool = false;
        let argument = Parameter::argument(Scalar::new(&mut item), "item")
            .choice(true, "b")
            .choice(false, "d")
            .choice(true, "e")
            .help("help")
            .consume();

        assert_eq!(argument.class, ParameterClass::Arg);
        assert_eq!(argument.name, "item".to_string());
        assert_eq!(argument.short, None);
        assert_eq!(argument.description, Some("help".to_string()));
        assert_eq!(
            argument.choices,
            HashMap::from([
                ("true".to_string(), "e".to_string()),
                ("false".to_string(), "d".to_string())
            ])
        );
    }

    #[test]
    fn condition() {
        let mut item: bool = false;
        let condition = Condition::new(Scalar::new(&mut item), "item")
            .choice(true, "b")
            .choice(false, "d")
            .choice(true, "e")
            .help("help")
            .consume();
        let argument = condition.consume();

        assert_eq!(argument.class, ParameterClass::Arg);
        assert_eq!(argument.name, "item".to_string());
        assert_eq!(argument.short, None);
        assert_eq!(argument.description, Some("help".to_string()));
        assert_eq!(
            argument.choices,
            HashMap::from([
                ("true".to_string(), "e".to_string()),
                ("false".to_string(), "d".to_string())
            ])
        );
    }
}
