use quote::ToTokens;
use std::collections::HashMap;
use syn::__private::TokenStream2;

#[derive(Debug)]
pub enum DeriveValue {
    Literal(TokenStream2),
}

#[derive(Debug, Default)]
pub struct DeriveAttributes {
    pub pairs: HashMap<String, DeriveValue>,
}

impl From<&syn::Attribute> for DeriveAttributes {
    fn from(value: &syn::Attribute) -> Self {
        let attributes_parser =
            syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated;
        let attributes_parse = value.parse_args_with(attributes_parser);
        let mut pairs = HashMap::default();

        for expression in
            attributes_parse.expect("syn::Attribute must parse as comma separated syn::Expr")
        {
            match expression {
                syn::Expr::Assign(assignment) => {
                    let left = assignment.left.to_token_stream();
                    pairs.insert(
                        left.to_string(),
                        DeriveValue::Literal(assignment.right.to_token_stream()),
                    );
                }
                _ => {
                    // TODO
                }
            };
        }

        Self { pairs }
    }
}
