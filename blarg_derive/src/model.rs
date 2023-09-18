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
    pub parameter_type: ParameterType,
}

#[derive(Debug, PartialEq, Eq)]
pub struct DeriveParser {
    pub struct_name: syn::Ident,
    pub program_name: DeriveValue,
    pub parameters: Vec<DeriveParameter>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct DeriveSubParser {
    pub struct_name: syn::Ident,
    pub parameters: Vec<DeriveParameter>,
}
