use crate::model::{DeriveAttributes, DeriveValue};
use quote::{quote, ToTokens};
use std::collections::{HashMap, HashSet};

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
                    let tts = expression.to_token_stream();
                    let expression_string = quote! {
                        #tts
                    };
                    panic!("Unparseable attribute: {expression_string}");
                }
            };
        }

        Self { singletons, pairs }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proc_macro2::Literal;
    use quote::ToTokens;
    use std::collections::{HashMap, HashSet};
    use syn::parse_quote;

    #[test]
    fn construct_derive_attributes_empty() {
        // Setup
        let attribute: syn::Attribute = parse_quote! {
            #[blarg()]
        };

        // Execute
        let derive_attributes = DeriveAttributes::from(&attribute);

        // Verify
        assert_eq!(
            derive_attributes,
            DeriveAttributes {
                singletons: HashSet::default(),
                pairs: HashMap::default()
            }
        );
    }

    #[test]
    fn construct_derive_attributes() {
        // Setup
        let attribute: syn::Attribute = parse_quote! {
            #[blarg(abc, qwerty = "123")]
        };

        // Execute
        let derive_attributes = DeriveAttributes::from(&attribute);

        // Verify
        assert_eq!(
            derive_attributes,
            DeriveAttributes {
                singletons: HashSet::from(["abc".to_string()]),
                pairs: HashMap::from([(
                    "qwerty".to_string(),
                    DeriveValue {
                        tokens: Literal::string("123").into_token_stream(),
                    }
                )])
            }
        );
    }

    #[test]
    #[should_panic]
    fn construct_derive_attributes_invalid() {
        // Setup
        let attribute: syn::Attribute = parse_quote! {
            #[blarg]
        };

        // Execute & verify
        let _ = DeriveAttributes::from(&attribute);
    }

    #[test]
    #[should_panic]
    fn construct_derive_attributes_invalid_expression() {
        // Setup
        let attribute: syn::Attribute = parse_quote! {
            #[blarg(let boo = "boo")]
        };

        // Execute & verify
        let _ = DeriveAttributes::from(&attribute);
    }
}
