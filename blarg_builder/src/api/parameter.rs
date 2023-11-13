use crate::api::{CliArgument, CliOption, GenericCapturable, Scalar};
use crate::matcher::{ArgumentConfig, Bound, OptionConfig};
use crate::model::Nargs;
use crate::parser::{
    AnonymousCapturable, ArgumentCapture, ArgumentParameter, OptionCapture, OptionParameter,
};
use crate::prelude::Choices;
use crate::InvalidCapture;
use std::collections::HashMap;

pub(crate) struct AnonymousCapture<'a, T: 'a> {
    field: Box<dyn GenericCapturable<'a, T> + 'a>,
}

impl<'a, T> AnonymousCapture<'a, T> {
    pub(crate) fn bind(field: impl GenericCapturable<'a, T> + 'a) -> Self {
        Self {
            field: Box::new(field),
        }
    }
}

impl<'a, T> AnonymousCapturable for AnonymousCapture<'a, T> {
    fn matched(&mut self) {
        self.field.matched();
    }

    fn capture(&mut self, value: &str) -> Result<(), InvalidCapture> {
        self.field.capture(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ParameterClass {
    Opt,
    Arg,
}

pub(super) struct ParameterInner<'a, T> {
    class: ParameterClass,
    field: AnonymousCapture<'a, T>,
    nargs: Nargs,
    name: String,
    short: Option<char>,
    help: Option<String>,
    meta: Option<Vec<String>>,
    choices: HashMap<String, String>,
}

impl<'a, T> ParameterInner<'a, T> {
    pub(super) fn class(&self) -> ParameterClass {
        self.class
    }
}

impl<'a, T> std::fmt::Debug for ParameterInner<'a, T> {
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
        let help = if let Some(d) = &self.help {
            format!(", {d}")
        } else {
            "".to_string()
        };

        write!(
            f,
            "{class}[{t}, {nargs}, {name},{short} {help}]",
            t = std::any::type_name::<T>(),
            nargs = self.nargs,
        )
    }
}

impl<'a, T> From<&ParameterInner<'a, T>> for OptionConfig {
    fn from(value: &ParameterInner<'a, T>) -> Self {
        OptionConfig::new(
            value.name.clone(),
            value.short.clone(),
            Bound::from(value.nargs),
        )
    }
}

impl<'a, T> From<ParameterInner<'a, T>> for OptionCapture<'a> {
    fn from(value: ParameterInner<'a, T>) -> Self {
        let config = OptionConfig::from(&value);
        let ParameterInner { field, .. } = value;
        (config, Box::new(field))
    }
}

impl<'a, T> From<&ParameterInner<'a, T>> for OptionParameter {
    fn from(value: &ParameterInner<'a, T>) -> Self {
        OptionParameter::new(
            value.name.clone(),
            value.short.clone(),
            value.nargs,
            value.help.clone(),
            value.meta.clone(),
            value.choices.clone(),
        )
    }
}

impl<'a, T> From<&ParameterInner<'a, T>> for ArgumentConfig {
    fn from(value: &ParameterInner<'a, T>) -> Self {
        ArgumentConfig::new(value.name.clone(), Bound::from(value.nargs))
    }
}

impl<'a, T> From<ParameterInner<'a, T>> for ArgumentCapture<'a> {
    fn from(value: ParameterInner<'a, T>) -> Self {
        let config = ArgumentConfig::from(&value);
        let ParameterInner { field, .. } = value;
        (config, Box::new(field))
    }
}

impl<'a, T> From<&ParameterInner<'a, T>> for ArgumentParameter {
    fn from(value: &ParameterInner<'a, T>) -> Self {
        ArgumentParameter::new(
            value.name.clone(),
            value.nargs,
            value.help.clone(),
            value.meta.clone(),
            value.choices.clone(),
        )
    }
}

/// The condition argument with which to branch the parser.
/// Used with [`CommandLineParser::branch`](./struct.CommandLineParser.html#method.branch).
///
/// There is an implicit (non-compile time) requirement for the type `T` of a `Condition`:
/// > The implementations of `std::str::FromStr` must invert `std::fmt::Display`.
///
/// This sounds scary and onerous, but most types will naturally adhere to this requirement.
/// Consider rusts implementation for `bool`, where this is requirement holds:
/// ```
/// # use std::str::FromStr;
/// assert_eq!(bool::from_str("true").unwrap().to_string(), "true");
/// assert_eq!(bool::from_str("false").unwrap().to_string(), "false");
/// ```
///
/// However, not all types will necessarily adhere to this requirement.
/// Observe the following example enum:
/// ```
/// # use std::str::FromStr;
/// // Implement FromStr to be case-insensitive.
/// // Implement Display.
/// enum FooBar {
///     Foo,
///     Bar,
/// }
/// # impl FromStr for FooBar {
/// #    type Err = String;
/// #    fn from_str(value: &str) -> Result<Self, Self::Err> {
/// #        match value.to_lowercase().as_str() {
/// #            "foo" => Ok(FooBar::Foo),
/// #            "bar" => Ok(FooBar::Bar),
/// #            _ => Err(format!("unknown: {}", value)),
/// #        }
/// #    }
/// # }
/// # impl std::fmt::Display for FooBar {
/// #   fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
/// #       match self {
/// #           FooBar::Foo => write!(f, "Foo"),
/// #           FooBar::Bar => write!(f, "Bar"),
/// #       }
/// #   }
/// # }
/// assert_eq!(FooBar::from_str("Foo").unwrap().to_string(), "Foo");
/// // FromStr does not invert Display!
/// assert_ne!(FooBar::from_str("foo").unwrap().to_string(), "foo");
/// ```
pub struct Condition<'a, T>(Parameter<'a, T>);

impl<'a, T: std::str::FromStr + std::fmt::Display> Condition<'a, T> {
    /// Create a condition parameter.
    ///
    /// ### Example
    /// ```
    /// # use blarg_builder as blarg;
    /// use blarg::{Condition, Scalar};
    /// use std::str::FromStr;
    ///
    /// // Be sure to implement `std::str::FromStr` so that it inverts `std::fmt::Display`.
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
    pub fn new(value: Scalar<'a, T>, name: &'static str) -> Self {
        Condition(Parameter::argument(value, name))
    }

    /// Document the help message for this sub-command condition.
    /// If repeated, only the final message will apply to the sub-command condition.
    ///
    /// A help message describes the condition in full sentence/paragraph format.
    /// We recommend allowing `blarg` to format this field (ex: it is not recommended to use line breaks `'\n'`).
    ///
    /// See also:
    /// * [`Condition::meta`]
    /// * [`Condition::choice`]
    ///
    /// ### Example
    /// ```
    /// # use blarg_builder as blarg;
    /// use blarg::{Condition, Scalar};
    ///
    /// let mut case: u32 = 0;
    /// Condition::new(Scalar::new(&mut case), "case")
    ///     .help("--this will get discarded--")
    ///     .help("Choose the 'case' to execute.  Description may include multiple sentences.");
    /// ```
    pub fn help(self, description: impl Into<String>) -> Self {
        let inner = self.0;
        Self(inner.help(description))
    }

    /// Document the meta message(s) for this sub-command condition.
    /// If repeated, only the final message will apply to the sub-command condition.
    ///
    /// Meta message(s) describe short format extra details about the condition.
    /// We recommend non-sentence information for this field.
    ///
    /// See also:
    /// * [`Condition::help`]
    /// * [`Condition::choice`]
    ///
    /// ### Example
    /// ```
    /// # use blarg_builder as blarg;
    /// use blarg::{Condition, Scalar};
    ///
    /// let mut case: u32 = 0;
    /// Condition::new(Scalar::new(&mut case), "case")
    ///     .meta(vec!["--this will get discarded--"])
    ///     .meta(vec!["final extra", "details"]);
    /// ```
    pub fn meta(self, description: Vec<impl Into<String>>) -> Self {
        let inner = self.0;
        Self(inner.meta(description))
    }

    pub(super) fn consume(self) -> Parameter<'a, T> {
        self.0
    }
}

impl<'a, T: std::str::FromStr + std::fmt::Display> Choices<T> for Condition<'a, T> {
    /// Document a choice's help message for the sub-command condition.
    /// If repeated for the same `variant` of `T`, only the final message will apply to the sub-command condition.
    /// Repeat using different variants to document multiple choices.
    /// Needn't be exhaustive.
    ///
    /// A choice help message describes the variant in full sentence/paragraph format.
    /// We recommend allowing `blarg` to format this field (ex: it is not recommended to use line breaks `'\n'`).
    ///
    /// Notice, the documented or un-documented choices *do not* affect the actual command parser semantics.
    /// To actually limit the command parser semantics, be sure to use an enum.
    ///
    /// See also:
    /// * [`Condition::help`]
    /// * [`Condition::meta`]
    ///
    /// ### Example
    /// ```
    /// # use blarg_builder as blarg;
    /// use blarg::{prelude::*, Condition, Scalar};
    /// use std::str::FromStr;
    ///
    /// // Be sure to implement `std::str::FromStr` so that it inverts `std::fmt::Display`.
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
    ///     .choice(FooBar::Bar, "Do bar'y things.  Description may include multiple sentences.");
    /// ```
    fn choice(self, variant: T, description: impl Into<String>) -> Self {
        let inner = self.0;
        Self(inner.choice(variant, description))
    }
}

/// An argument/option for the command parser.
/// Used with [`CommandLineParser::add`](./struct.CommandLineParser.html#method.add) and [`SubCommand::add`](./struct.SubCommand.html#method.add).
pub struct Parameter<'a, T>(ParameterInner<'a, T>);

impl<'a, T> Parameter<'a, T> {
    /// Create an option parameter.
    ///
    /// ### Example
    /// ```
    /// # use blarg_builder as blarg;
    /// use blarg::{Parameter, Switch};
    ///
    /// let mut verbose: bool = false;
    /// Parameter::option(Switch::new(&mut verbose, true), "verbose", Some('v'));
    /// ```
    pub fn option(
        field: impl GenericCapturable<'a, T> + CliOption + 'a,
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
            help: None,
            meta: None,
            choices: HashMap::default(),
        })
    }

    /// Create an argument parameter.
    ///
    /// ### Example
    /// ```
    /// # use blarg_builder as blarg;
    /// use blarg::{Parameter, Scalar};
    ///
    /// let mut verbose: bool = false;
    /// Parameter::argument(Scalar::new(&mut verbose), "verbose");
    /// ```
    pub fn argument(
        field: impl GenericCapturable<'a, T> + CliArgument + 'a,
        name: impl Into<String>,
    ) -> Self {
        let nargs = field.nargs();
        Self(ParameterInner {
            class: ParameterClass::Arg,
            field: AnonymousCapture::bind(field),
            nargs,
            name: name.into(),
            short: None,
            help: None,
            meta: None,
            choices: HashMap::default(),
        })
    }

    /// Document the help message for this parameter.
    /// If repeated, only the final message will apply to the parameter.
    ///
    /// A help message describes the parameter in full sentence/paragraph format.
    /// We recommend allowing `blarg` to format this field (ex: it is not recommended to use line breaks `'\n'`).
    ///
    /// See also:
    /// * [`Parameter::meta`]
    /// * [`Parameter::choice`]
    ///
    /// ### Example
    /// ```
    /// # use blarg_builder as blarg;
    /// use blarg::{Parameter, Scalar};
    ///
    /// let mut verbose: bool = false;
    /// Parameter::argument(Scalar::new(&mut verbose), "verbose")
    ///     .help("--this will get discarded--")
    ///     .help("Make the program output verbose.  Description may include multiple sentences.");
    /// ```
    pub fn help(self, description: impl Into<String>) -> Self {
        let mut inner = self.0;
        inner.help = Some(description.into());
        Self(inner)
    }

    /// Document the meta message(s) for this parameter.
    /// If repeated, only the final message will apply to the parameter.
    ///
    /// Meta message(s) describe short format extra details about the parameter.
    /// We recommend non-sentence information for this field.
    ///
    /// See also:
    /// * [`Parameter::help`]
    /// * [`Parameter::choice`]
    ///
    /// ### Example
    /// ```
    /// # use blarg_builder as blarg;
    /// use blarg::{Parameter, Scalar};
    ///
    /// let mut verbose: bool = false;
    /// Parameter::argument(Scalar::new(&mut verbose), "verbose")
    ///     .meta(vec!["--this will be discarded--"])
    ///     .meta(vec!["final extra", "details"]);
    /// ```
    pub fn meta(self, descriptions: Vec<impl Into<String>>) -> Self {
        let mut inner = self.0;
        inner.meta = Some(descriptions.into_iter().map(|s| s.into()).collect());
        Self(inner)
    }

    pub(super) fn name(&self) -> String {
        self.0.name.clone()
    }

    pub(super) fn consume(self) -> ParameterInner<'a, T> {
        self.0
    }
}

impl<'a, T: std::fmt::Display> Choices<T> for Parameter<'a, T> {
    /// Document a choice's help message for this parameter.
    /// If repeated for the same `variant` of `T`, only the final message will apply to the parameter.
    /// Repeat using different variants to document multiple choices.
    /// Needn't be exhaustive.
    ///
    /// A choice help message describes the variant in full sentence/paragraph format.
    /// We recommend allowing `blarg` to format this field (ex: it is not recommended to use line breaks `'\n'`).
    ///
    /// Notice, the documented or un-documented choices *do not* affect the actual command parser semantics.
    /// To actually limit the command parser semantics, be sure to use an enum.
    ///
    /// See also:
    /// * [`Parameter::help`]
    /// * [`Parameter::meta`]
    ///
    /// ### Example
    /// ```
    /// # use blarg_builder as blarg;
    /// use blarg::{prelude::*, Parameter, Scalar};
    /// use std::str::FromStr;
    ///
    /// let mut door: u32 = 0;
    /// Parameter::argument(Scalar::new(&mut door), "door")
    ///     .choice(1, "--this will get discarded--")
    ///     .choice(1, "Enter door #1.")
    ///     .choice(2, "Enter door #2.  Description may include multiple sentences.");
    /// ```
    fn choice(self, variant: T, description: impl Into<String>) -> Self {
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
        assert_eq!(option.help, None);
        assert_eq!(option.meta, None);
        assert_eq!(option.choices, HashMap::default());
    }

    #[test]
    fn option_short() {
        let mut flag: bool = false;
        let option = Parameter::option(Switch::new(&mut flag, true), "flag", Some('f')).consume();

        assert_eq!(option.class, ParameterClass::Opt);
        assert_eq!(option.name, "flag");
        assert_eq!(option.short, Some('f'));
        assert_eq!(option.help, None);
        assert_eq!(option.meta, None);
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
        assert_eq!(option.help, Some("help message".to_string()));
        assert_eq!(option.meta, None);
        assert_eq!(option.choices, HashMap::default());
    }

    #[test]
    fn option_meta() {
        let mut flag: bool = false;
        let option = Parameter::option(Switch::new(&mut flag, true), "flag", None)
            .meta(vec!["meta message"])
            .consume();

        assert_eq!(option.class, ParameterClass::Opt);
        assert_eq!(option.name, "flag".to_string());
        assert_eq!(option.short, None);
        assert_eq!(option.help, None);
        assert_eq!(option.meta, Some(vec!["meta message".to_string()]));
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
        assert_eq!(option.help, None);
        assert_eq!(option.meta, None);
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
        assert_eq!(argument.help, None);
        assert_eq!(argument.meta, None);
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
        assert_eq!(argument.help, Some("help message".to_string()));
        assert_eq!(argument.meta, None);
        assert_eq!(argument.choices, HashMap::default());
    }

    #[test]
    fn argument_meta() {
        let mut item: bool = false;
        let argument = Parameter::argument(Scalar::new(&mut item), "item")
            .meta(vec!["meta message"])
            .consume();

        assert_eq!(argument.class, ParameterClass::Arg);
        assert_eq!(argument.name, "item".to_string());
        assert_eq!(argument.short, None);
        assert_eq!(argument.help, None);
        assert_eq!(argument.meta, Some(vec!["meta message".to_string()]));
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
            .meta(vec!["meta"])
            .consume();

        assert_eq!(argument.class, ParameterClass::Arg);
        assert_eq!(argument.name, "item".to_string());
        assert_eq!(argument.short, None);
        assert_eq!(argument.help, Some("help".to_string()));
        assert_eq!(argument.meta, Some(vec!["meta".to_string()]));
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
            .meta(vec!["meta"])
            .consume();
        let argument = condition.consume();

        assert_eq!(argument.class, ParameterClass::Arg);
        assert_eq!(argument.name, "item".to_string());
        assert_eq!(argument.short, None);
        assert_eq!(argument.help, Some("help".to_string()));
        assert_eq!(argument.meta, Some(vec!["meta".to_string()]));
        assert_eq!(
            argument.choices,
            HashMap::from([
                ("true".to_string(), "e".to_string()),
                ("false".to_string(), "d".to_string())
            ])
        );
    }
}
