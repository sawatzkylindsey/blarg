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
pub struct DeriveAttributes {
    pub singletons: HashSet<String>,
    pub pairs: HashMap<String, DeriveValue>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParameterType {
    Collection,
    Optional,
    Scalar,
    Switch,
}

#[derive(Debug, PartialEq, Eq)]
pub struct DeriveParameter {
    pub field_name: syn::Ident,
    pub attributes: DeriveAttributes,
    pub parameter_type: ParameterType,
}

#[derive(Debug, PartialEq, Eq)]
pub struct DeriveParser {
    pub struct_name: syn::Ident,
    pub attributes: DeriveAttributes,
    pub parameters: Vec<DeriveParameter>,
}
