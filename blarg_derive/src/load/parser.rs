use crate::model::{
    DeriveParameter, DeriveParser, DeriveSubParser, DeriveValue, IntermediateAttributes,
    ParameterType,
};
use quote::quote;

impl TryFrom<syn::DeriveInput> for DeriveParser {
    type Error = syn::Error;

    fn try_from(value: syn::DeriveInput) -> Result<Self, Self::Error> {
        let mut attributes = IntermediateAttributes::default();

        for attribute in &value.attrs {
            if attribute.path().is_ident("blarg") {
                attributes = IntermediateAttributes::from(attribute);
            }
        }

        let program = match attributes.pairs.get("program") {
            Some(values) => {
                let tokens = &values
                    .first()
                    .expect("attribute pair 'program' must contain non-empty values")
                    .tokens;
                quote! { #tokens }
            }
            None => quote! { env!("CARGO_CRATE_NAME") },
        };
        let parser_name = &value.ident;

        match &value.data {
            syn::Data::Struct(ds) => {
                let parameters = match ds {
                    syn::DataStruct {
                        fields: syn::Fields::Named(ref fields),
                        ..
                    } => fields
                        .named
                        .iter()
                        .map(DeriveParameter::try_from)
                        .collect::<Result<Vec<_>, _>>()?,
                    syn::DataStruct { .. } => Vec::default(),
                };

                let conditions: Vec<&syn::Ident> = parameters
                    .iter()
                    .filter_map(|p| match &p.parameter_type {
                        ParameterType::Condition { .. } => Some(&p.field_name),
                        _ => None,
                    })
                    .collect();
                if conditions.len() > 1 {
                    return Err(syn::Error::new(
                        value.ident.span(),
                        format!(
                            "Invalid - parser cannot have multiple conditions: {:?}.",
                            conditions.iter().map(|i| i.to_string()).collect::<Vec<_>>(),
                        ),
                    ));
                }

                let cli_parser = DeriveParser {
                    struct_name: parser_name.clone(),
                    program_name: DeriveValue {
                        tokens: program.into(),
                    },
                    parameters,
                };
                // println!("{cli_parser:?}");
                Ok(cli_parser)
            }
            _ => {
                todo!()
            }
        }
    }
}

impl TryFrom<syn::DeriveInput> for DeriveSubParser {
    type Error = syn::Error;

    fn try_from(value: syn::DeriveInput) -> Result<Self, Self::Error> {
        let parser_name = &value.ident;

        match &value.data {
            syn::Data::Struct(ds) => {
                let parameters = match ds {
                    syn::DataStruct {
                        fields: syn::Fields::Named(ref fields),
                        ..
                    } => fields
                        .named
                        .iter()
                        .map(DeriveParameter::try_from)
                        .collect::<Result<Vec<_>, _>>()?,
                    syn::DataStruct { .. } => Vec::default(),
                };
                let cli_sub_parser = DeriveSubParser {
                    struct_name: parser_name.clone(),
                    parameters,
                };
                // println!("{cli_sub_parser:?}");
                Ok(cli_sub_parser)
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
        let derive_parser = DeriveParser::try_from(input).unwrap();

        // Verify
        assert_eq!(
            derive_parser,
            DeriveParser {
                struct_name: ident("Parameters"),
                program_name: DeriveValue {
                    tokens: quote! { env!("CARGO_CRATE_NAME") }
                },
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
        let derive_parser = DeriveParser::try_from(input).unwrap();

        // Verify
        assert_eq!(
            derive_parser,
            DeriveParser {
                struct_name: ident("Parameters"),
                program_name: DeriveValue {
                    tokens: quote! { env!("CARGO_CRATE_NAME") }
                },
                parameters: vec![DeriveParameter {
                    field_name: ident("apple"),
                    parameter_type: ParameterType::ScalarArgument,
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
        let derive_parser = DeriveParser::try_from(input).unwrap();

        // Verify
        assert_eq!(
            derive_parser,
            DeriveParser {
                struct_name: ident("Parameters"),
                program_name: DeriveValue {
                    tokens: Literal::string("abc").into_token_stream()
                },
                parameters: vec![DeriveParameter {
                    field_name: ident("apple"),
                    parameter_type: ParameterType::ScalarArgument,
                }],
            }
        );
    }

    #[test]
    fn construct_derive_parser_multiple_conditions() {
        // Setup
        let input: syn::DeriveInput = syn::parse_str(
            r#"
                #[derive(Default, BlargParser)]
                struct Parameters {
                    #[blarg(command = (0, Abc))]
                    apple: usize,
                    #[blarg(command = (1, Def))]
                    banana: usize,
                }
            "#,
        )
        .unwrap();

        // Execute
        let error = DeriveParser::try_from(input).unwrap_err();

        // Verify
        assert_eq!(
            error.to_string(),
            "Invalid - parser cannot have multiple conditions: [\"apple\", \"banana\"]."
        );
    }

    #[test]
    fn construct_derive_sub_parser_empty() {
        // Setup
        let input: syn::DeriveInput = syn::parse_str(
            r#"
                #[derive(Default, BlargSubParser)]
                struct Parameters { }
            "#,
        )
        .unwrap();

        // Execute
        let derive_sub_parser = DeriveSubParser::try_from(input).unwrap();

        // Verify
        assert_eq!(
            derive_sub_parser,
            DeriveSubParser {
                struct_name: ident("Parameters"),
                parameters: Vec::default(),
            }
        );
    }

    #[test]
    fn construct_derive_sub_parser() {
        // Setup
        let input: syn::DeriveInput = syn::parse_str(
            r#"
                #[derive(Default, BlargSubParser)]
                struct Parameters {
                    apple: usize,
                }
            "#,
        )
        .unwrap();

        // Execute
        let derive_sub_parser = DeriveSubParser::try_from(input).unwrap();

        // Verify
        assert_eq!(
            derive_sub_parser,
            DeriveSubParser {
                struct_name: ident("Parameters"),
                parameters: vec![DeriveParameter {
                    field_name: ident("apple"),
                    parameter_type: ParameterType::ScalarArgument,
                }],
            }
        );
    }

    fn ident(name: &str) -> syn::Ident {
        syn::Ident::new(name, Span::call_site())
    }
}
