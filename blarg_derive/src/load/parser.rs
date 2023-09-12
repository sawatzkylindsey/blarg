use crate::model::{DeriveAttributes, DeriveParameter, DeriveParser};

impl From<syn::DeriveInput> for DeriveParser {
    fn from(value: syn::DeriveInput) -> Self {
        let mut attributes = DeriveAttributes::default();

        for attribute in &value.attrs {
            if attribute.path().is_ident("blarg") {
                attributes = DeriveAttributes::from(attribute);
            }
        }

        let parser_name = &value.ident;

        match &value.data {
            syn::Data::Struct(ds) => {
                let parameters = match ds {
                    syn::DataStruct {
                        fields: syn::Fields::Named(ref fields),
                        ..
                    } => fields.named.iter().map(DeriveParameter::from).collect(),
                    syn::DataStruct { .. } => Vec::default(),
                };
                let cli_parser = DeriveParser {
                    struct_name: parser_name.clone(),
                    attributes,
                    parameters,
                };
                // println!("{cli_parser:?}");
                cli_parser
            }
            _ => {
                todo!()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{DeriveValue, ParameterType};
    use proc_macro2::Literal;
    use proc_macro2::Span;
    use quote::ToTokens;
    use std::collections::{HashMap, HashSet};

    #[test]
    fn construct_derive_parser_empty() {
        // Setup
        let input: syn::DeriveInput = syn::parse_str(
            r#"
                #[derive(Default, BlargParser)]
                struct Parameters { }
            "#,
        )
        .unwrap();

        // Execute
        let derive_parser = DeriveParser::from(input);

        // Verify
        assert_eq!(
            derive_parser,
            DeriveParser {
                struct_name: ident("Parameters"),
                attributes: DeriveAttributes::default(),
                parameters: Vec::default(),
            }
        );
    }

    #[test]
    fn construct_derive_parser() {
        // Setup
        let input: syn::DeriveInput = syn::parse_str(
            r#"
                #[derive(Default, BlargParser)]
                struct Parameters {
                    apple: usize,
                }
            "#,
        )
        .unwrap();

        // Execute
        let derive_parser = DeriveParser::from(input);

        // Verify
        assert_eq!(
            derive_parser,
            DeriveParser {
                struct_name: ident("Parameters"),
                attributes: DeriveAttributes::default(),
                parameters: vec![DeriveParameter {
                    field_name: ident("apple"),
                    attributes: DeriveAttributes {
                        singletons: HashSet::default(),
                        pairs: HashMap::default()
                    },
                    parameter_type: ParameterType::Scalar,
                }],
            }
        );
    }

    #[test]
    fn construct_derive_parser_with_attributes() {
        // Setup
        let input: syn::DeriveInput = syn::parse_str(
            r#"
                #[derive(Default, BlargParser)]
                #[blarg(program = "abc")]
                struct Parameters {
                    apple: usize,
                }
            "#,
        )
        .unwrap();

        // Execute
        let derive_parser = DeriveParser::from(input);

        // Verify
        assert_eq!(
            derive_parser,
            DeriveParser {
                struct_name: ident("Parameters"),
                attributes: DeriveAttributes {
                    singletons: HashSet::default(),
                    pairs: HashMap::from([(
                        "program".to_string(),
                        DeriveValue {
                            tokens: Literal::string("abc").into_token_stream(),
                        }
                    )])
                },
                parameters: vec![DeriveParameter {
                    field_name: ident("apple"),
                    attributes: DeriveAttributes {
                        singletons: HashSet::default(),
                        pairs: HashMap::default()
                    },
                    parameter_type: ParameterType::Scalar,
                }],
            }
        );
    }

    fn ident(name: &str) -> syn::Ident {
        syn::Ident::new(name, Span::call_site())
    }
}
