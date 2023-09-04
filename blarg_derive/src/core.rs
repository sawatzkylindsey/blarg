use quote::ToTokens;
use std::collections::{HashMap, HashSet};
use syn::__private::TokenStream2;

#[derive(Debug)]
pub struct DeriveValue {
    pub tokens: TokenStream2,
}

impl PartialEq for DeriveValue {
    fn eq(&self, other: &Self) -> bool {
        true
    }

    fn ne(&self, other: &Self) -> bool {
        true
    }
}

impl Eq for DeriveValue {}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct DeriveAttributes {
    pub singletons: HashSet<String>,
    pub pairs: HashMap<String, DeriveValue>,
}

impl From<&syn::Attribute> for DeriveAttributes {
    fn from(value: &syn::Attribute) -> Self {
        let attributes_parser =
            syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated;
        let attributes_parse = value.parse_args_with(attributes_parser);
        let mut singletons = HashSet::default();
        let mut pairs = HashMap::default();

        for expression in
            attributes_parse.expect("syn::Attribute must parse as comma separated syn::Expr")
        {
            match expression {
                syn::Expr::Assign(assignment) => {
                    let left = assignment.left.to_token_stream();
                    pairs.insert(
                        left.to_string(),
                        DeriveValue {
                            tokens: assignment.right.to_token_stream(),
                        },
                    );
                }
                syn::Expr::Path(path) => {
                    if let Some(ident) = path.path.get_ident() {
                        singletons.insert(ident.to_string());
                    }
                }
                _ => {
                    // TODO
                }
            };
        }

        Self { singletons, pairs }
    }
}
