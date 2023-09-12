use crate::model::{DeriveAttributes, DeriveParameter, ParameterType};
use quote::{quote, ToTokens};

impl From<&syn::Field> for DeriveParameter {
    fn from(value: &syn::Field) -> Self {
        let mut attributes = DeriveAttributes::default();

        for attribute in &value.attrs {
            if attribute.path().is_ident("blarg") {
                attributes = DeriveAttributes::from(attribute);
            }
        }

        let parameter_type = match &value.ty {
            syn::Type::Path(path) => match &path.path.segments.first() {
                Some(segment) => {
                    let ident = segment.ident.to_string();

                    match ident.as_str() {
                        "Option" => ParameterType::Optional,
                        "Vec" | "HashSet" => ParameterType::Collection,
                        "bool" => ParameterType::Switch,
                        _ => ParameterType::Scalar,
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

        DeriveParameter {
            field_name: value.ident.clone().unwrap(),
            attributes,
            parameter_type,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::DeriveValue;
    use proc_macro2::Literal;
    use proc_macro2::Span;
    use quote::ToTokens;
    use std::collections::{HashMap, HashSet};
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
        let _ = DeriveParameter::from(&input);
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
        let _ = DeriveParameter::from(&input);
    }

    #[test]
    fn construct_derive_parameter() {
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
        let derive_parameter = DeriveParameter::from(&input);

        // Verify
        assert_eq!(
            derive_parameter,
            DeriveParameter {
                field_name: ident("my_field"),
                attributes: DeriveAttributes::default(),
                parameter_type: ParameterType::Scalar,
            }
        );
    }

    #[test]
    fn construct_derive_parameter_collection() {
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
        let derive_parameter = DeriveParameter::from(&input);

        // Verify
        assert_eq!(
            derive_parameter,
            DeriveParameter {
                field_name: ident("my_field"),
                attributes: DeriveAttributes::default(),
                parameter_type: ParameterType::Collection,
            }
        );
    }

    #[test]
    fn construct_derive_parameter_option() {
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
        let derive_parameter = DeriveParameter::from(&input);

        // Verify
        assert_eq!(
            derive_parameter,
            DeriveParameter {
                field_name: ident("my_field"),
                attributes: DeriveAttributes::default(),
                parameter_type: ParameterType::Optional,
            }
        );
    }

    #[test]
    fn construct_derive_parameter_switch() {
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
        let derive_parameter = DeriveParameter::from(&input);

        // Verify
        assert_eq!(
            derive_parameter,
            DeriveParameter {
                field_name: ident("my_field"),
                attributes: DeriveAttributes::default(),
                parameter_type: ParameterType::Switch,
            }
        );
    }

    #[test]
    fn construct_derive_parameter_with_attributes() {
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
        let derive_parameter = DeriveParameter::from(&input);

        // Verify
        assert_eq!(
            derive_parameter,
            DeriveParameter {
                field_name: ident("my_field"),
                attributes: DeriveAttributes {
                    singletons: HashSet::from(["argument".to_string()]),
                    pairs: HashMap::from([(
                        "short".to_string(),
                        DeriveValue {
                            tokens: Literal::character('c').into_token_stream(),
                        }
                    )])
                },
                parameter_type: ParameterType::Scalar,
            }
        );
    }

    fn ident(name: &str) -> syn::Ident {
        syn::Ident::new(name, Span::call_site())
    }
}
