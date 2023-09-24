use crate::load::incompatible_error;
use crate::model::{Command, DeriveParameter, DeriveValue, IntermediateAttributes, ParameterType};
use quote::{quote, ToTokens};

impl TryFrom<&syn::Field> for DeriveParameter {
    type Error = syn::Error;

    fn try_from(value: &syn::Field) -> Result<Self, Self::Error> {
        let mut attributes = IntermediateAttributes::default();

        for attribute in &value.attrs {
            if attribute.path().is_ident("blarg") {
                attributes = IntermediateAttributes::from(attribute);
            }
        }

        let field_name = value.ident.clone().unwrap();
        let explicit_argument = attributes.singletons.contains("argument");
        let explicit_option = attributes.singletons.contains("option");
        let short = match attributes.pairs.get("short") {
            Some(values) => {
                let tokens = values
                    .first()
                    .expect("attribute pair 'short' must contain non-empty values")
                    .tokens
                    .clone();
                Some(DeriveValue { tokens })
            }
            None => None,
        };
        let (explicit_collection, nargs) = match attributes.pairs.get("collection") {
            Some(values) => {
                let tokens = values
                    .first()
                    .expect("attribute pair 'collection' must contain non-empty values")
                    .tokens
                    .clone();
                (true, DeriveValue { tokens })
            }
            None => (
                false,
                DeriveValue {
                    tokens: quote! { Nargs::AtLeastOne },
                },
            ),
        };
        let help = match attributes.pairs.get("help") {
            Some(values) => {
                let tokens = values
                    .first()
                    .expect("attribute pair 'help' must contain non-empty values")
                    .tokens
                    .clone();
                Some(DeriveValue { tokens })
            }
            None => None,
        };
        let commands: Option<&Vec<DeriveValue>> = attributes.pairs.get("command");
        let explicit_command = commands.is_some();

        if explicit_argument && explicit_option {
            return Err(incompatible_error(
                &field_name,
                "#[blarg(argument)]",
                "#[blarg(option)]",
            ));
        }

        if explicit_command && explicit_option {
            return Err(incompatible_error(
                &field_name,
                "#[blarg(command = ..)]",
                "#[blarg(option)]",
            ));
        }

        if explicit_command && explicit_collection {
            return Err(incompatible_error(
                &field_name,
                "#[blarg(command = ..)]",
                "#[blarg(collection = ..)]",
            ));
        }

        let parameter_type = match &value.ty {
            syn::Type::Path(path) => match &path.path.segments.first() {
                Some(segment) => {
                    let ident = segment.ident.to_string();

                    match ident.as_str() {
                        "Option" => {
                            disallow(
                                &field_name,
                                "Option<..>",
                                &[
                                    (&explicit_argument, "argument"),
                                    (&explicit_collection, "#[blarg(collection = ..)]"),
                                    (&explicit_command, "#[blarg(command = ..)]"),
                                ],
                            )?;

                            ParameterType::OptionalOption { short }
                        }
                        "Vec" | "HashSet" => {
                            disallow(
                                &field_name,
                                format!("{}<..>", ident.as_str()),
                                &[(&explicit_command, "#[blarg(command = ..)]")],
                            )?;

                            if explicit_option {
                                ParameterType::CollectionOption { nargs, short }
                            } else {
                                ParameterType::CollectionArgument { nargs }
                            }
                        }
                        "bool" => {
                            disallow(
                                &field_name,
                                "bool",
                                &[(&explicit_command, "#[blarg(command = ..)]")],
                            )?;

                            ParameterType::Switch { short }
                        }
                        _ => {
                            if let Some(cmds) = commands {
                                let commands = cmds
                                    .iter()
                                    .map(|derive_value| build_command(&field_name, derive_value))
                                    .collect::<Result<Vec<_>, _>>()?;
                                ParameterType::Condition { commands }
                            } else if explicit_collection {
                                ParameterType::CollectionArgument { nargs }
                            } else if explicit_option {
                                ParameterType::ScalarOption { short }
                            } else {
                                ParameterType::ScalarArgument
                            }
                        }
                    }
                }
                None => {
                    let tts = &value.to_token_stream();
                    let type_string = quote! {
                        #tts
                    };
                    panic!("Empty field path: {type_string}");
                }
            },
            _ => {
                let tts = &value.ty.to_token_stream();
                let field_string = quote! {
                    #tts
                };
                panic!("Unparseable field: {field_string}");
            }
        };

        Ok(DeriveParameter {
            field_name: value.ident.clone().unwrap(),
            parameter_type,
            help,
        })
    }
}

fn build_command(
    field_name: &syn::Ident,
    derive_value: &DeriveValue,
) -> Result<Command, syn::Error> {
    let expression: syn::Expr = syn::parse2(derive_value.tokens.clone()).unwrap();
    match expression {
        syn::Expr::Tuple(tuple) => match (tuple.elems.first(), tuple.elems.last()) {
            (Some(syn::Expr::Lit(left)), Some(syn::Expr::Path(right))) => Ok(Command {
                variant: DeriveValue {
                    tokens: left.to_token_stream(),
                },
                command_struct: DeriveValue {
                    tokens: right.to_token_stream(),
                },
            }),
            (Some(syn::Expr::Path(left)), Some(syn::Expr::Path(right))) => Ok(Command {
                variant: DeriveValue {
                    tokens: left.to_token_stream(),
                },
                command_struct: DeriveValue {
                    tokens: right.to_token_stream(),
                },
            }),
            _ => {
                let tts = &derive_value.tokens;
                let expression_string = quote! {
                    #tts
                };
                return Err(syn::Error::new(
                        field_name.span(),
                        format!("Invalid - command assignment expecting `(BranchVariant, SubCommandStruct)`, found `{expression_string}`."),
                    ));
            }
        },
        _ => {
            let tts = &derive_value.tokens;
            let expression_string = quote! {
                #tts
            };
            return Err(syn::Error::new(
                    field_name.span(),
                    format!("Invalid - command assignment expecting `(BranchVariant, SubCommandStruct)`, found `{expression_string}`."),
                ));
        }
    }
}

fn disallow(
    field_name: &syn::Ident,
    antecedent: impl Into<String>,
    condition_names: &[(&bool, &str)],
) -> Result<(), syn::Error> {
    for (condition, name) in condition_names {
        if **condition {
            return Err(incompatible_error(
                field_name,
                antecedent,
                format!("#[blarg({name})]").as_str(),
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::DeriveValue;
    use crate::test::assert_contains;
    use proc_macro2::Literal;
    use proc_macro2::Span;
    use quote::ToTokens;
    use syn::{parse_quote, AngleBracketedGenericArguments, PathArguments, PathSegment};

    #[test]
    #[should_panic]
    fn construct_derive_parameter_unknown_type() {
        // Setup
        let input: syn::Field = syn::Field {
            attrs: vec![],
            vis: syn::Visibility::Inherited,
            mutability: syn::FieldMutability::None,
            ident: Some(ident("my_field")),
            colon_token: None,
            ty: syn::Type::Verbatim(Literal::string("moot").into_token_stream()),
        };

        // Execute & verify
        let _ = DeriveParameter::try_from(&input).unwrap();
    }

    #[test]
    #[should_panic]
    fn construct_derive_parameter_empty() {
        // Setup
        let segments = syn::punctuated::Punctuated::new();
        let input: syn::Field = syn::Field {
            attrs: vec![],
            vis: syn::Visibility::Inherited,
            mutability: syn::FieldMutability::None,
            ident: Some(ident("my_field")),
            colon_token: None,
            ty: syn::Type::Path(syn::TypePath {
                qself: None,
                path: syn::Path {
                    leading_colon: None,
                    segments,
                },
            }),
        };

        // Execute & verify
        let _ = DeriveParameter::try_from(&input).unwrap();
    }

    //# Implicit construction

    #[test]
    fn construct_scalar_argument() {
        // Setup
        let mut segments = syn::punctuated::Punctuated::new();
        segments.push_value(PathSegment {
            ident: ident("usize"),
            arguments: PathArguments::None,
        });
        let input: syn::Field = syn::Field {
            attrs: vec![],
            vis: syn::Visibility::Inherited,
            mutability: syn::FieldMutability::None,
            ident: Some(ident("my_field")),
            colon_token: None,
            ty: syn::Type::Path(syn::TypePath {
                qself: None,
                path: syn::Path {
                    leading_colon: None,
                    segments,
                },
            }),
        };

        // Execute
        let derive_parameter = DeriveParameter::try_from(&input).unwrap();

        // Verify
        assert_eq!(
            derive_parameter,
            DeriveParameter {
                field_name: ident("my_field"),
                parameter_type: ParameterType::ScalarArgument,
                help: None,
            }
        );
    }

    #[test]
    fn construct_optional_option() {
        // Setup
        let mut segments = syn::punctuated::Punctuated::new();
        segments.push_value(PathSegment {
            ident: ident("Option"),
            arguments: PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                colon2_token: None,
                lt_token: Default::default(),
                args: Default::default(),
                gt_token: Default::default(),
            }),
        });
        let input: syn::Field = syn::Field {
            attrs: vec![],
            vis: syn::Visibility::Inherited,
            mutability: syn::FieldMutability::None,
            ident: Some(ident("my_field")),
            colon_token: None,
            ty: syn::Type::Path(syn::TypePath {
                qself: None,
                path: syn::Path {
                    leading_colon: None,
                    segments,
                },
            }),
        };

        // Execute
        let derive_parameter = DeriveParameter::try_from(&input).unwrap();

        // Verify
        assert_eq!(
            derive_parameter,
            DeriveParameter {
                field_name: ident("my_field"),
                parameter_type: ParameterType::OptionalOption { short: None },
                help: None,
            }
        );
    }

    #[test]
    fn construct_optional_option_short() {
        // Setup
        let mut segments = syn::punctuated::Punctuated::new();
        segments.push_value(PathSegment {
            ident: ident("Option"),
            arguments: PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                colon2_token: None,
                lt_token: Default::default(),
                args: Default::default(),
                gt_token: Default::default(),
            }),
        });
        let attribute: syn::Attribute = parse_quote! {
            #[blarg(short = 'm')]
        };
        let input: syn::Field = syn::Field {
            attrs: vec![attribute],
            vis: syn::Visibility::Inherited,
            mutability: syn::FieldMutability::None,
            ident: Some(ident("my_field")),
            colon_token: None,
            ty: syn::Type::Path(syn::TypePath {
                qself: None,
                path: syn::Path {
                    leading_colon: None,
                    segments,
                },
            }),
        };

        // Execute
        let derive_parameter = DeriveParameter::try_from(&input).unwrap();

        // Verify
        assert_eq!(
            derive_parameter,
            DeriveParameter {
                field_name: ident("my_field"),
                parameter_type: ParameterType::OptionalOption {
                    short: Some(DeriveValue {
                        tokens: Literal::character('m').into_token_stream(),
                    }),
                },
                help: None,
            }
        );
    }

    #[test]
    fn construct_switch() {
        // Setup
        let mut segments = syn::punctuated::Punctuated::new();
        segments.push_value(PathSegment {
            ident: ident("bool"),
            arguments: PathArguments::None,
        });
        let input: syn::Field = syn::Field {
            attrs: vec![],
            vis: syn::Visibility::Inherited,
            mutability: syn::FieldMutability::None,
            ident: Some(ident("my_field")),
            colon_token: None,
            ty: syn::Type::Path(syn::TypePath {
                qself: None,
                path: syn::Path {
                    leading_colon: None,
                    segments,
                },
            }),
        };

        // Execute
        let derive_parameter = DeriveParameter::try_from(&input).unwrap();

        // Verify
        assert_eq!(
            derive_parameter,
            DeriveParameter {
                field_name: ident("my_field"),
                parameter_type: ParameterType::Switch { short: None },
                help: None,
            }
        );
    }

    #[test]
    fn construct_collection_argument() {
        // Setup
        let mut segments = syn::punctuated::Punctuated::new();
        segments.push_value(PathSegment {
            ident: ident("Vec"),
            arguments: PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                colon2_token: None,
                lt_token: Default::default(),
                args: Default::default(),
                gt_token: Default::default(),
            }),
        });
        let input: syn::Field = syn::Field {
            attrs: vec![],
            vis: syn::Visibility::Inherited,
            mutability: syn::FieldMutability::None,
            ident: Some(ident("my_field")),
            colon_token: None,
            ty: syn::Type::Path(syn::TypePath {
                qself: None,
                path: syn::Path {
                    leading_colon: None,
                    segments,
                },
            }),
        };

        // Execute
        let derive_parameter = DeriveParameter::try_from(&input).unwrap();

        // Verify
        assert_eq!(
            derive_parameter,
            DeriveParameter {
                field_name: ident("my_field"),
                parameter_type: ParameterType::CollectionArgument {
                    nargs: DeriveValue {
                        tokens: quote! { Nargs::AtLeastOne }
                    }
                },
                help: None,
            }
        );
    }

    #[test]
    fn construct_with_help() {
        // Setup
        let mut segments = syn::punctuated::Punctuated::new();
        segments.push_value(PathSegment {
            ident: ident("usize"),
            arguments: PathArguments::None,
        });
        let attribute: syn::Attribute = parse_quote! {
            #[blarg(help = "abc 123")]
        };
        let input: syn::Field = syn::Field {
            attrs: vec![attribute],
            vis: syn::Visibility::Inherited,
            mutability: syn::FieldMutability::None,
            ident: Some(ident("my_field")),
            colon_token: None,
            ty: syn::Type::Path(syn::TypePath {
                qself: None,
                path: syn::Path {
                    leading_colon: None,
                    segments,
                },
            }),
        };

        // Execute
        let derive_parameter = DeriveParameter::try_from(&input).unwrap();

        // Verify
        assert_eq!(
            derive_parameter,
            DeriveParameter {
                field_name: ident("my_field"),
                parameter_type: ParameterType::ScalarArgument,
                help: Some(DeriveValue {
                    tokens: Literal::string("abc 123").to_token_stream(),
                }),
            }
        );
    }

    //# Explicit construction

    #[test]
    fn construct_scalar_option() {
        // Setup
        let mut segments = syn::punctuated::Punctuated::new();
        segments.push_value(PathSegment {
            ident: ident("usize"),
            arguments: PathArguments::None,
        });
        let attribute: syn::Attribute = parse_quote! {
            #[blarg(option)]
        };
        let input: syn::Field = syn::Field {
            attrs: vec![attribute],
            vis: syn::Visibility::Inherited,
            mutability: syn::FieldMutability::None,
            ident: Some(ident("my_field")),
            colon_token: None,
            ty: syn::Type::Path(syn::TypePath {
                qself: None,
                path: syn::Path {
                    leading_colon: None,
                    segments,
                },
            }),
        };

        // Execute
        let derive_parameter = DeriveParameter::try_from(&input).unwrap();

        // Verify
        assert_eq!(
            derive_parameter,
            DeriveParameter {
                field_name: ident("my_field"),
                parameter_type: ParameterType::ScalarOption { short: None },
                help: None,
            }
        );
    }

    #[test]
    fn construct_scalar_option_short() {
        // Setup
        let mut segments = syn::punctuated::Punctuated::new();
        segments.push_value(PathSegment {
            ident: ident("usize"),
            arguments: PathArguments::None,
        });
        let attribute: syn::Attribute = parse_quote! {
            #[blarg(option, short = 'm')]
        };
        let input: syn::Field = syn::Field {
            attrs: vec![attribute],
            vis: syn::Visibility::Inherited,
            mutability: syn::FieldMutability::None,
            ident: Some(ident("my_field")),
            colon_token: None,
            ty: syn::Type::Path(syn::TypePath {
                qself: None,
                path: syn::Path {
                    leading_colon: None,
                    segments,
                },
            }),
        };

        // Execute
        let derive_parameter = DeriveParameter::try_from(&input).unwrap();

        // Verify
        assert_eq!(
            derive_parameter,
            DeriveParameter {
                field_name: ident("my_field"),
                parameter_type: ParameterType::ScalarOption {
                    short: Some(DeriveValue {
                        tokens: Literal::character('m').into_token_stream(),
                    })
                },
                help: None,
            }
        );
    }

    #[test]
    fn construct_condition_lit() {
        // Setup
        let mut segments = syn::punctuated::Punctuated::new();
        segments.push_value(PathSegment {
            ident: ident("usize"),
            arguments: PathArguments::None,
        });
        let attribute: syn::Attribute = parse_quote! {
            #[blarg(command = (0, Abc), command = (1, Def))]
        };
        let input: syn::Field = syn::Field {
            attrs: vec![attribute],
            vis: syn::Visibility::Inherited,
            mutability: syn::FieldMutability::None,
            ident: Some(ident("my_field")),
            colon_token: None,
            ty: syn::Type::Path(syn::TypePath {
                qself: None,
                path: syn::Path {
                    leading_colon: None,
                    segments,
                },
            }),
        };

        // Execute
        let derive_parameter = DeriveParameter::try_from(&input).unwrap();

        // Verify
        assert_eq!(
            derive_parameter,
            DeriveParameter {
                field_name: ident("my_field"),
                parameter_type: ParameterType::Condition {
                    commands: vec![
                        Command {
                            variant: DeriveValue {
                                tokens: Literal::usize_unsuffixed(0).into_token_stream(),
                            },
                            command_struct: DeriveValue {
                                tokens: ident("Abc").to_token_stream(),
                            }
                        },
                        Command {
                            variant: DeriveValue {
                                tokens: Literal::usize_unsuffixed(1).into_token_stream(),
                            },
                            command_struct: DeriveValue {
                                tokens: ident("Def").to_token_stream(),
                            }
                        }
                    ]
                },
                help: None,
            }
        );
    }

    #[test]
    fn construct_condition_path() {
        // Setup
        let mut segments = syn::punctuated::Punctuated::new();
        segments.push_value(PathSegment {
            ident: ident("usize"),
            arguments: PathArguments::None,
        });
        let attribute: syn::Attribute = parse_quote! {
            #[blarg(command = (Foo::Bar, Abc), command = (Foo::Baz, Def))]
        };
        let input: syn::Field = syn::Field {
            attrs: vec![attribute],
            vis: syn::Visibility::Inherited,
            mutability: syn::FieldMutability::None,
            ident: Some(ident("my_field")),
            colon_token: None,
            ty: syn::Type::Path(syn::TypePath {
                qself: None,
                path: syn::Path {
                    leading_colon: None,
                    segments,
                },
            }),
        };

        // Execute
        let derive_parameter = DeriveParameter::try_from(&input).unwrap();

        // Verify
        let foo_bar: syn::Path = parse_quote! { Foo::Bar };
        let foo_baz: syn::Path = parse_quote! { Foo::Baz };
        assert_eq!(
            derive_parameter,
            DeriveParameter {
                field_name: ident("my_field"),
                parameter_type: ParameterType::Condition {
                    commands: vec![
                        Command {
                            variant: DeriveValue {
                                tokens: foo_bar.to_token_stream(),
                            },
                            command_struct: DeriveValue {
                                tokens: ident("Abc").to_token_stream(),
                            }
                        },
                        Command {
                            variant: DeriveValue {
                                tokens: foo_baz.to_token_stream(),
                            },
                            command_struct: DeriveValue {
                                tokens: ident("Def").to_token_stream(),
                            }
                        }
                    ]
                },
                help: None,
            }
        );
    }

    #[test]
    fn construct_collection_option() {
        // Setup
        let mut segments = syn::punctuated::Punctuated::new();
        segments.push_value(PathSegment {
            ident: ident("Vec"),
            arguments: PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                colon2_token: None,
                lt_token: Default::default(),
                args: Default::default(),
                gt_token: Default::default(),
            }),
        });
        let attribute: syn::Attribute = parse_quote! {
            #[blarg(option)]
        };
        let input: syn::Field = syn::Field {
            attrs: vec![attribute],
            vis: syn::Visibility::Inherited,
            mutability: syn::FieldMutability::None,
            ident: Some(ident("my_field")),
            colon_token: None,
            ty: syn::Type::Path(syn::TypePath {
                qself: None,
                path: syn::Path {
                    leading_colon: None,
                    segments,
                },
            }),
        };

        // Execute
        let derive_parameter = DeriveParameter::try_from(&input).unwrap();

        // Verify
        assert_eq!(
            derive_parameter,
            DeriveParameter {
                field_name: ident("my_field"),
                parameter_type: ParameterType::CollectionOption {
                    nargs: DeriveValue {
                        tokens: quote! { Nargs::AtLeastOne }
                    },
                    short: None,
                },
                help: None,
            }
        );
    }

    #[test]
    fn construct_collection_option_short() {
        // Setup
        let mut segments = syn::punctuated::Punctuated::new();
        segments.push_value(PathSegment {
            ident: ident("Vec"),
            arguments: PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                colon2_token: None,
                lt_token: Default::default(),
                args: Default::default(),
                gt_token: Default::default(),
            }),
        });
        let attribute: syn::Attribute = parse_quote! {
            #[blarg(option, short = 'm')]
        };
        let input: syn::Field = syn::Field {
            attrs: vec![attribute],
            vis: syn::Visibility::Inherited,
            mutability: syn::FieldMutability::None,
            ident: Some(ident("my_field")),
            colon_token: None,
            ty: syn::Type::Path(syn::TypePath {
                qself: None,
                path: syn::Path {
                    leading_colon: None,
                    segments,
                },
            }),
        };

        // Execute
        let derive_parameter = DeriveParameter::try_from(&input).unwrap();

        // Verify
        assert_eq!(
            derive_parameter,
            DeriveParameter {
                field_name: ident("my_field"),
                parameter_type: ParameterType::CollectionOption {
                    nargs: DeriveValue {
                        tokens: quote! { Nargs::AtLeastOne }
                    },
                    short: Some(DeriveValue {
                        tokens: Literal::character('m').into_token_stream(),
                    }),
                },
                help: None,
            },
        );
    }

    #[test]
    fn construct_superfluous_short() {
        // Setup
        let mut segments = syn::punctuated::Punctuated::new();
        segments.push_value(PathSegment {
            ident: ident("usize"),
            arguments: PathArguments::None,
        });
        let attribute: syn::Attribute = parse_quote! {
            #[blarg(argument, short = 'c')]
        };
        let input: syn::Field = syn::Field {
            attrs: vec![attribute],
            vis: syn::Visibility::Inherited,
            mutability: syn::FieldMutability::None,
            ident: Some(ident("my_field")),
            colon_token: None,
            ty: syn::Type::Path(syn::TypePath {
                qself: None,
                path: syn::Path {
                    leading_colon: None,
                    segments,
                },
            }),
        };

        // Execute
        let derive_parameter = DeriveParameter::try_from(&input).unwrap();

        // Verify
        assert_eq!(
            derive_parameter,
            DeriveParameter {
                field_name: ident("my_field"),
                parameter_type: ParameterType::ScalarArgument,
                help: None,
            },
        );
    }

    //# Invalid construction

    #[test]
    fn construct_argument_option() {
        // Setup
        let mut segments = syn::punctuated::Punctuated::new();
        segments.push_value(PathSegment {
            ident: ident("usize"),
            arguments: PathArguments::None,
        });
        let attribute: syn::Attribute = parse_quote! {
            #[blarg(argument, option)]
        };
        let input: syn::Field = syn::Field {
            attrs: vec![attribute],
            vis: syn::Visibility::Inherited,
            mutability: syn::FieldMutability::None,
            ident: Some(ident("my_field")),
            colon_token: None,
            ty: syn::Type::Path(syn::TypePath {
                qself: None,
                path: syn::Path {
                    leading_colon: None,
                    segments,
                },
            }),
        };

        // Execute
        let error = DeriveParameter::try_from(&input).unwrap_err();

        // Verify
        assert_contains!(error.to_string(), "Invalid - field cannot be both");
        assert_contains!(error.to_string(), "#[blarg(argument)]");
        assert_contains!(error.to_string(), "#[blarg(option)]");
    }

    #[test]
    fn construct_command_option() {
        // Setup
        let mut segments = syn::punctuated::Punctuated::new();
        segments.push_value(PathSegment {
            ident: ident("usize"),
            arguments: PathArguments::None,
        });
        let attribute: syn::Attribute = parse_quote! {
            #[blarg(command = (0, Abc), option)]
        };
        let input: syn::Field = syn::Field {
            attrs: vec![attribute],
            vis: syn::Visibility::Inherited,
            mutability: syn::FieldMutability::None,
            ident: Some(ident("my_field")),
            colon_token: None,
            ty: syn::Type::Path(syn::TypePath {
                qself: None,
                path: syn::Path {
                    leading_colon: None,
                    segments,
                },
            }),
        };

        // Execute
        let error = DeriveParameter::try_from(&input).unwrap_err();

        // Verify
        assert_contains!(error.to_string(), "Invalid - field cannot be both");
        assert_contains!(error.to_string(), "#[blarg(command = ..)]");
        assert_contains!(error.to_string(), "#[blarg(option)]");
    }

    #[test]
    fn construct_command_collection() {
        // Setup
        let mut segments = syn::punctuated::Punctuated::new();
        segments.push_value(PathSegment {
            ident: ident("usize"),
            arguments: PathArguments::None,
        });
        let attribute: syn::Attribute = parse_quote! {
            #[blarg(command = (0, Abc), collection = Nargs::Any)]
        };
        let input: syn::Field = syn::Field {
            attrs: vec![attribute],
            vis: syn::Visibility::Inherited,
            mutability: syn::FieldMutability::None,
            ident: Some(ident("my_field")),
            colon_token: None,
            ty: syn::Type::Path(syn::TypePath {
                qself: None,
                path: syn::Path {
                    leading_colon: None,
                    segments,
                },
            }),
        };

        // Execute
        let error = DeriveParameter::try_from(&input).unwrap_err();

        // Verify
        assert_contains!(error.to_string(), "Invalid - field cannot be both");
        assert_contains!(error.to_string(), "#[blarg(command = ..)]");
        assert_contains!(error.to_string(), "#[blarg(collection = ..)]");
    }

    #[test]
    fn construct_condition_invalid() {
        // Setup
        let mut segments = syn::punctuated::Punctuated::new();
        segments.push_value(PathSegment {
            ident: ident("usize"),
            arguments: PathArguments::None,
        });
        let attribute: syn::Attribute = parse_quote! {
            #[blarg(command = abc)]
        };
        let input: syn::Field = syn::Field {
            attrs: vec![attribute],
            vis: syn::Visibility::Inherited,
            mutability: syn::FieldMutability::None,
            ident: Some(ident("my_field")),
            colon_token: None,
            ty: syn::Type::Path(syn::TypePath {
                qself: None,
                path: syn::Path {
                    leading_colon: None,
                    segments,
                },
            }),
        };

        // Execute
        let error = DeriveParameter::try_from(&input).unwrap_err();

        // Verify
        assert_contains!(
            error.to_string(),
            "Invalid - command assignment expecting `(BranchVariant, SubCommandStruct)`"
        );
        assert_contains!(error.to_string(), "found `abc`");
    }

    //# Invalid construction via implicit

    #[test]
    fn construct_command_option_implicit() {
        // Setup
        let mut segments = syn::punctuated::Punctuated::new();
        segments.push_value(PathSegment {
            ident: ident("Option"),
            arguments: PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                colon2_token: None,
                lt_token: Default::default(),
                args: Default::default(),
                gt_token: Default::default(),
            }),
        });
        let attribute: syn::Attribute = parse_quote! {
            #[blarg(command = (0, Abc))]
        };
        let input: syn::Field = syn::Field {
            attrs: vec![attribute],
            vis: syn::Visibility::Inherited,
            mutability: syn::FieldMutability::None,
            ident: Some(ident("my_field")),
            colon_token: None,
            ty: syn::Type::Path(syn::TypePath {
                qself: None,
                path: syn::Path {
                    leading_colon: None,
                    segments,
                },
            }),
        };

        // Execute
        let error = DeriveParameter::try_from(&input).unwrap_err();

        // Verify
        assert_contains!(error.to_string(), "Invalid - field cannot be both");
        assert_contains!(error.to_string(), "#[blarg(command = ..)]");
        assert_contains!(error.to_string(), "Option<..>");
    }

    #[test]
    fn construct_argument_option_implicit() {
        // Setup
        let mut segments = syn::punctuated::Punctuated::new();
        segments.push_value(PathSegment {
            ident: ident("Option"),
            arguments: PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                colon2_token: None,
                lt_token: Default::default(),
                args: Default::default(),
                gt_token: Default::default(),
            }),
        });
        let attribute: syn::Attribute = parse_quote! {
            #[blarg(argument)]
        };
        let input: syn::Field = syn::Field {
            attrs: vec![attribute],
            vis: syn::Visibility::Inherited,
            mutability: syn::FieldMutability::None,
            ident: Some(ident("my_field")),
            colon_token: None,
            ty: syn::Type::Path(syn::TypePath {
                qself: None,
                path: syn::Path {
                    leading_colon: None,
                    segments,
                },
            }),
        };

        // Execute
        let error = DeriveParameter::try_from(&input).unwrap_err();

        // Verify
        assert_contains!(error.to_string(), "Invalid - field cannot be both");
        assert_contains!(error.to_string(), "#[blarg(argument)]");
        assert_contains!(error.to_string(), "Option<..>");
    }

    #[test]
    fn construct_collection_option_implicit() {
        // Setup
        let mut segments = syn::punctuated::Punctuated::new();
        segments.push_value(PathSegment {
            ident: ident("Option"),
            arguments: PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                colon2_token: None,
                lt_token: Default::default(),
                args: Default::default(),
                gt_token: Default::default(),
            }),
        });
        let attribute: syn::Attribute = parse_quote! {
            #[blarg(collection = asdf)]
        };
        let input: syn::Field = syn::Field {
            attrs: vec![attribute],
            vis: syn::Visibility::Inherited,
            mutability: syn::FieldMutability::None,
            ident: Some(ident("my_field")),
            colon_token: None,
            ty: syn::Type::Path(syn::TypePath {
                qself: None,
                path: syn::Path {
                    leading_colon: None,
                    segments,
                },
            }),
        };

        // Execute
        let error = DeriveParameter::try_from(&input).unwrap_err();

        // Verify
        assert_contains!(error.to_string(), "Invalid - field cannot be both");
        assert_contains!(error.to_string(), "#[blarg(collection = ..)]");
        assert_contains!(error.to_string(), "Option<..>");
    }

    #[test]
    fn construct_command_collection_implicit_vec() {
        // Setup
        let mut segments = syn::punctuated::Punctuated::new();
        segments.push_value(PathSegment {
            ident: ident("Vec"),
            arguments: PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                colon2_token: None,
                lt_token: Default::default(),
                args: Default::default(),
                gt_token: Default::default(),
            }),
        });
        let attribute: syn::Attribute = parse_quote! {
            #[blarg(command = (0, Abc))]
        };
        let input: syn::Field = syn::Field {
            attrs: vec![attribute],
            vis: syn::Visibility::Inherited,
            mutability: syn::FieldMutability::None,
            ident: Some(ident("my_field")),
            colon_token: None,
            ty: syn::Type::Path(syn::TypePath {
                qself: None,
                path: syn::Path {
                    leading_colon: None,
                    segments,
                },
            }),
        };

        // Execute
        let error = DeriveParameter::try_from(&input).unwrap_err();

        // Verify
        assert_contains!(error.to_string(), "Invalid - field cannot be both");
        assert_contains!(error.to_string(), "#[blarg(command = ..)]");
        assert_contains!(error.to_string(), "Vec<..>");
    }

    #[test]
    fn construct_command_collection_implicit_hashset() {
        // Setup
        let mut segments = syn::punctuated::Punctuated::new();
        segments.push_value(PathSegment {
            ident: ident("HashSet"),
            arguments: PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                colon2_token: None,
                lt_token: Default::default(),
                args: Default::default(),
                gt_token: Default::default(),
            }),
        });
        let attribute: syn::Attribute = parse_quote! {
            #[blarg(command = (0, Abc))]
        };
        let input: syn::Field = syn::Field {
            attrs: vec![attribute],
            vis: syn::Visibility::Inherited,
            mutability: syn::FieldMutability::None,
            ident: Some(ident("my_field")),
            colon_token: None,
            ty: syn::Type::Path(syn::TypePath {
                qself: None,
                path: syn::Path {
                    leading_colon: None,
                    segments,
                },
            }),
        };

        // Execute
        let error = DeriveParameter::try_from(&input).unwrap_err();

        // Verify
        assert_contains!(error.to_string(), "Invalid - field cannot be both");
        assert_contains!(error.to_string(), "#[blarg(command = ..)]");
        assert_contains!(error.to_string(), "HashSet<..>");
    }

    #[test]
    fn construct_command_switch_implicit() {
        // Setup
        let mut segments = syn::punctuated::Punctuated::new();
        segments.push_value(PathSegment {
            ident: ident("bool"),
            arguments: PathArguments::None,
        });
        let attribute: syn::Attribute = parse_quote! {
            #[blarg(command = (0, Abc))]
        };
        let input: syn::Field = syn::Field {
            attrs: vec![attribute],
            vis: syn::Visibility::Inherited,
            mutability: syn::FieldMutability::None,
            ident: Some(ident("my_field")),
            colon_token: None,
            ty: syn::Type::Path(syn::TypePath {
                qself: None,
                path: syn::Path {
                    leading_colon: None,
                    segments,
                },
            }),
        };

        // Execute
        let error = DeriveParameter::try_from(&input).unwrap_err();

        // Verify
        assert_contains!(error.to_string(), "Invalid - field cannot be both");
        assert_contains!(error.to_string(), "#[blarg(command = ..)]");
        assert_contains!(error.to_string(), "bool");
    }

    fn ident(name: &str) -> syn::Ident {
        syn::Ident::new(name, Span::call_site())
    }
}
