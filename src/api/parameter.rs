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

pub struct Condition<'ap, T>(Parameter<'ap, T>);

impl<'ap, T: std::str::FromStr + std::fmt::Display> Condition<'ap, T> {
    pub fn new(value: Scalar<'ap, T>, name: &'static str) -> Self {
        Condition(Parameter::argument(value, name))
    }

    pub fn choice(self, variant: T, description: impl Into<String>) -> Self {
        let inner = self.0;
        Self(inner.choice(variant, description))
    }

    pub fn help(self, description: impl Into<String>) -> Self {
        let inner = self.0;
        Self(inner.help(description))
    }

    pub(super) fn consume(self) -> Parameter<'ap, T> {
        self.0
    }
}

pub struct Parameter<'ap, T>(ParameterInner<'ap, T>);

impl<'ap, T> Parameter<'ap, T> {
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
