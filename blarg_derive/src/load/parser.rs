use crate::load::incompatible_error;
use crate::model::Hints;
use crate::{
    model::{
        DeriveParameter, DeriveParser, DeriveSubParser, DeriveValue, IntermediateAttributes,
        ParameterType,
    },
    {MACRO_BLARG_PARSER, MACRO_BLARG_SUB_PARSER},
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
        let about = match attributes.pairs.get("about") {
            Some(values) => {
                let tokens = &values
                    .first()
                    .expect("attribute pair 'about' must contain non-empty values")
                    .tokens;
                Some(DeriveValue {
                    tokens: quote! { #tokens },
                })
            }
            None => None,
        };
        let initializer = match attributes.pairs.get("initializer") {
            Some(values) => {
                let tokens = &values
                    .first()
                    .expect("attribute pair 'initializer' must contain non-empty values")
                    .tokens;
                quote! { #tokens }
            }
            None => quote! { default },
        };
        let parser_name = &value.ident;

        let hints = if attributes.singletons.contains("hints_off") {
            if attributes.singletons.contains("hints_on") {
                return Err(incompatible_error(
                    "struct",
                    &parser_name,
                    "#[blarg(hints_on)]",
                    "#[blarg(hints_off)]",
                ));
            } else {
                Hints::Off
            }
        } else {
            Hints::On
        };

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
                            "Invalid - {MACRO_BLARG_PARSER} cannot have multiple conditions: {:?}.",
                            conditions.iter().map(|i| i.to_string()).collect::<Vec<_>>(),
                        ),
                    ));
                }

                let cli_parser = DeriveParser {
                    struct_name: parser_name.clone(),
                    program: DeriveValue {
                        tokens: program.into(),
                    },
                    about,
                    initializer: DeriveValue {
                        tokens: initializer.into(),
                    },
                    parameters,
                    hints,
                };
                // println!("{cli_parser:?}");
                Ok(cli_parser)
            }
            _ => Err(syn::Error::new(
                parser_name.span(),
                format!("Invalid - {MACRO_BLARG_PARSER} only applies to 'struct' data structures."),
            )),
        }
    }
}

impl TryFrom<syn::DeriveInput> for DeriveSubParser {
    type Error = syn::Error;

    fn try_from(value: syn::DeriveInput) -> Result<Self, Self::Error> {
        let mut attributes = IntermediateAttributes::default();
        for attribute in &value.attrs {
            if attribute.path().is_ident("blarg") {
                attributes = IntermediateAttributes::from(attribute);
            }
        }

        let about = match attributes.pairs.get("about") {
            Some(values) => {
                let tokens = &values
                    .first()
                    .expect("attribute pair 'about' must contain non-empty values")
                    .tokens;
                Some(DeriveValue {
                    tokens: quote! { #tokens },
                })
            }
            None => None,
        };
        let parser_name = &value.ident;

        let hints = if attributes.singletons.contains("hints_off") {
            if attributes.singletons.contains("hints_on") {
                return Err(incompatible_error(
                    "struct",
                    &parser_name,
                    "#[blarg(hints_on)]",
                    "#[blarg(hints_off)]",
                ));
            } else {
                Hints::Off
            }
        } else {
            Hints::On
        };

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
                if conditions.len() > 0 {
                    return Err(syn::Error::new(
                        value.ident.span(),
                        format!(
                            "Invalid - {MACRO_BLARG_SUB_PARSER} cannot have any conditions: {:?}.",
                            conditions.iter().map(|i| i.to_string()).collect::<Vec<_>>(),
                        ),
                    ));
                }

                let cli_sub_parser = DeriveSubParser {
                    struct_name: parser_name.clone(),
                    about,
                    parameters,
                    hints,
                };
                // println!("{cli_sub_parser:?}");
                Ok(cli_sub_parser)
            }
            _ => Err(syn::Error::new(
                parser_name.span(),
                format!(
                    "Invalid - {MACRO_BLARG_SUB_PARSER} only applies to 'struct' data structures."
                ),
            )),
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
                program: DeriveValue {
                    tokens: quote! { env!("CARGO_CRATE_NAME") }
                },
                about: None,
                initializer: DeriveValue {
                    tokens: quote! { default }.into_token_stream()
                },
                parameters: Vec::default(),
                hints: Hints::On,
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
                program: DeriveValue {
                    tokens: quote! { env!("CARGO_CRATE_NAME") }
                },
                about: None,
                initializer: DeriveValue {
                    tokens: quote! { default }.into_token_stream()
                },
                parameters: vec![DeriveParameter {
                    field_name: ident("apple"),
                    from_str_type: "usize".to_string(),
                    parameter_type: ParameterType::ScalarArgument,
                    choices: None,
                    help: None,
                }],
                hints: Hints::On,
            }
        );
    }

    #[test]
    fn construct_derive_parser_with_attributes() {
        // Setup
        let input: syn::DeriveInput = syn::parse_str(
            r#"
                #[derive(Default, BlargParser)]
                #[blarg(program = "abc", initializer = qwerty, hints_off, about = "def 123")]
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
                program: DeriveValue {
                    tokens: Literal::string("abc").into_token_stream()
                },
                about: Some(DeriveValue {
                    tokens: Literal::string("def 123").into_token_stream()
                }),
                initializer: DeriveValue {
                    tokens: quote! { qwerty }.into_token_stream()
                },
                parameters: vec![DeriveParameter {
                    field_name: ident("apple"),
                    from_str_type: "usize".to_string(),
                    parameter_type: ParameterType::ScalarArgument,
                    choices: None,
                    help: None,
                }],
                hints: Hints::Off,
            }
        );
    }

    #[test]
    fn construct_derive_parser_hints_offon() {
        // Setup
        let input: syn::DeriveInput = syn::parse_str(
            r#"
                #[derive(Default, BlargParser)]
                #[blarg(hints_off, hints_on)]
                struct Parameters {
                    apple: usize,
                }
            "#,
        )
        .unwrap();

        // Execute
        let error = DeriveParser::try_from(input).unwrap_err();

        // Verify
        assert_eq!(
            error.to_string(),
            "Invalid - struct cannot be both `#[blarg(hints_on)]` and `#[blarg(hints_off)]`."
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
            "Invalid - BlargParser cannot have multiple conditions: [\"apple\", \"banana\"]."
        );
    }

    #[test]
    fn construct_derive_parser_invalid() {
        // Setup
        let input: syn::DeriveInput = syn::parse_str(
            r#"
                #[derive(BlargParser)]
                enum Values { }
            "#,
        )
        .unwrap();

        // Execute
        let error = DeriveParser::try_from(input).unwrap_err();

        // Verify
        assert_eq!(
            error.to_string(),
            "Invalid - BlargParser only applies to 'struct' data structures."
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
                about: None,
                parameters: Vec::default(),
                hints: Hints::On,
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
                about: None,
                parameters: vec![DeriveParameter {
                    field_name: ident("apple"),
                    from_str_type: "usize".to_string(),
                    parameter_type: ParameterType::ScalarArgument,
                    choices: None,
                    help: None,
                }],
                hints: Hints::On,
            }
        );
    }

    #[test]
    fn construct_derive_sub_parser_with_attributes() {
        // Setup
        let input: syn::DeriveInput = syn::parse_str(
            r#"
                #[derive(Default, BlargSubParser)]
                #[blarg(hints_off, about = "def 123")]
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
                about: Some(DeriveValue {
                    tokens: Literal::string("def 123").into_token_stream()
                }),
                parameters: vec![DeriveParameter {
                    field_name: ident("apple"),
                    from_str_type: "usize".to_string(),
                    parameter_type: ParameterType::ScalarArgument,
                    choices: None,
                    help: None,
                }],
                hints: Hints::Off,
            }
        );
    }

    #[test]
    fn construct_derive_sub_parser_hints_offon() {
        // Setup
        let input: syn::DeriveInput = syn::parse_str(
            r#"
                #[derive(Default, BlargSubParser)]
                #[blarg(hints_off, hints_on)]
                struct Parameters {
                    apple: usize,
                }
            "#,
        )
        .unwrap();

        // Execute
        let error = DeriveSubParser::try_from(input).unwrap_err();

        // Verify
        assert_eq!(
            error.to_string(),
            "Invalid - struct cannot be both `#[blarg(hints_on)]` and `#[blarg(hints_off)]`."
        );
    }

    #[test]
    fn construct_derive_sub_parser_with_condition() {
        // Setup
        let input: syn::DeriveInput = syn::parse_str(
            r#"
                #[derive(Default, BlargSubParser)]
                struct Parameters {
                    #[blarg(command = (0, Abc))]
                    apple: usize,
                }
            "#,
        )
        .unwrap();

        // Execute
        let error = DeriveSubParser::try_from(input).unwrap_err();

        // Verify
        // Verify
        assert_eq!(
            error.to_string(),
            "Invalid - BlargSubParser cannot have any conditions: [\"apple\"]."
        );
    }

    #[test]
    fn construct_derive_sub_parser_invalid() {
        // Setup
        let input: syn::DeriveInput = syn::parse_str(
            r#"
                #[derive(BlargSubParser)]
                enum Values { }
            "#,
        )
        .unwrap();

        // Execute
        let error = DeriveSubParser::try_from(input).unwrap_err();

        // Verify
        assert_eq!(
            error.to_string(),
            "Invalid - BlargSubParser only applies to 'struct' data structures."
        );
    }

    fn ident(name: &str) -> syn::Ident {
        syn::Ident::new(name, Span::call_site())
    }
}
