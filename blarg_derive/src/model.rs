use proc_macro2::TokenStream as TokenStream2;
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
pub struct DeriveValue {
    pub tokens: TokenStream2,
}

impl PartialEq for DeriveValue {
    fn eq(&self, other: &Self) -> bool {
        let st = &self.tokens.to_string();
        let ot = &other.tokens.to_string();
        st == ot
    }

    fn ne(&self, other: &Self) -> bool {
        !self.eq(other)
    }
}

impl Eq for DeriveValue {}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct IntermediateAttributes {
    pub singletons: HashSet<String>,
    pub pairs: HashMap<String, Vec<DeriveValue>>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Command {
    pub variant: DeriveValue,
    pub command_struct: DeriveValue,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ParameterType {
    CollectionArgument {
        nargs: DeriveValue,
    },
    ScalarArgument,

    CollectionOption {
        nargs: DeriveValue,
        short: Option<DeriveValue>,
    },
    OptionalOption {
        short: Option<DeriveValue>,
    },
    ScalarOption {
        short: Option<DeriveValue>,
    },

    Switch {
        short: Option<DeriveValue>,
    },

    Condition {
        commands: Vec<Command>,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub struct DeriveParameter {
    pub field_name: syn::Ident,
    pub from_str_type: String,
    pub parameter_type: ParameterType,
    pub choices: Option<DeriveValue>,
    pub help: Option<DeriveValue>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Hints {
    On,
    Off,
}

#[derive(Debug, PartialEq, Eq)]
pub struct DeriveParser {
    pub struct_name: syn::Ident,
    pub program: DeriveValue,
    pub initializer: DeriveValue,
    pub parameters: Vec<DeriveParameter>,
    pub hints: Hints,
}

#[derive(Debug, PartialEq, Eq)]
pub struct DeriveSubParser {
    pub struct_name: syn::Ident,
    pub parameters: Vec<DeriveParameter>,
    pub hints: Hints,
}

#[derive(Debug, PartialEq, Eq)]
pub struct DeriveChoices {
    pub struct_name: syn::Ident,
    pub variants: Vec<DeriveVariant>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct DeriveVariant {
    pub field_name: syn::Ident,
    pub hidden: bool,
    pub help: Option<DeriveValue>,
}
