use crate::model::{DeriveParameter, DeriveValue, ParameterType};
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};

impl TryFrom<DeriveParameter> for TokenStream2 {
    type Error = syn::Error;

    fn try_from(value: DeriveParameter) -> Result<Self, Self::Error> {
        let DeriveParameter {
            field_name,
            attributes,
            parameter_type,
        } = value;
        let field_name_str = format!("{field_name}");

        let explicit_argument = attributes.singletons.contains("argument");
        let explicit_option = attributes.singletons.contains("option");

        if explicit_argument && explicit_option {
            return Err(syn::Error::new(
                field_name.span(),
                "Invalid - field cannot be both `#[blarg(option)]` and `#[blarg(argument)]`.",
            ));
        }

        let (explicit_collection, nargs) = match attributes.pairs.get("collection") {
            Some(DeriveValue { tokens }) => (true, quote! { #tokens }),
            None => (false, quote! { Nargs::AtLeastOne }),
        };
        let short = match attributes.pairs.get("short") {
            Some(DeriveValue { tokens }) => quote! { Some(#tokens) },
            None => quote! { None },
        };

        let token_stream = if explicit_argument {
            build_argument(
                parameter_type,
                explicit_collection,
                field_name,
                field_name_str,
                nargs,
            )
        } else if explicit_option {
            Ok(build_option(
                parameter_type,
                explicit_collection,
                field_name,
                field_name_str,
                nargs,
                short,
            ))
        } else if parameter_type == ParameterType::Optional
            || parameter_type == ParameterType::Switch
        {
            Ok(build_option(
                parameter_type,
                explicit_collection,
                field_name,
                field_name_str,
                nargs,
                short,
            ))
        } else {
            build_argument(
                parameter_type,
                explicit_collection,
                field_name,
                field_name_str,
                nargs,
            )
        };

        token_stream
    }
}

fn build_argument(
    parameter_type: ParameterType,
    explicit_collection: bool,
    field_name: syn::Ident,
    field_name_str: String,
    nargs: TokenStream2,
) -> Result<TokenStream2, syn::Error> {
    if explicit_collection {
        Ok(quote! {
            clp = clp.add(Parameter::argument(Collection::new(&mut target.#field_name, #nargs), #field_name_str));
        })
    } else {
        match parameter_type {
            ParameterType::Collection => Ok(quote! {
                clp = clp.add(Parameter::argument(Collection::new(&mut target.#field_name, #nargs), #field_name_str));
            }),
            ParameterType::Scalar => Ok(quote! {
                clp = clp.add(Parameter::argument(Scalar::new(&mut target.#field_name), #field_name_str));
            }),
            _ => Err(syn::Error::new(
                field_name.span(),
                "Invalid Parameter::argument - did you mean to use `#[blarg(option)]`?",
            )),
        }
    }
}

fn build_option(
    parameter_type: ParameterType,
    explicit_collection: bool,
    field_name: syn::Ident,
    field_name_str: String,
    nargs: TokenStream2,
    short: TokenStream2,
) -> TokenStream2 {
    if explicit_collection {
        quote! {
            clp = clp.add(Parameter::option(Collection::new(&mut target.#field_name, #nargs), #field_name_str, #short));
        }
    } else {
        match parameter_type {
            ParameterType::Collection => quote! {
                clp = clp.add(Parameter::option(Collection::new(&mut target.#field_name, #nargs), #field_name_str, #short));
            },
            ParameterType::Optional => quote! {
                clp = clp.add(Parameter::option(Optional::new(&mut target.#field_name), #field_name_str, #short));
            },
            ParameterType::Scalar => quote! {
                clp = clp.add(Parameter::option(Scalar::new(&mut target.#field_name), #field_name_str, #short));
            },
            ParameterType::Switch => {
                let field_name_target = format_ident!("{field_name}_target");
                quote! {
                    let #field_name_target = target.#field_name.clone();
                    clp = clp.add(Parameter::option(Switch::new(&mut target.#field_name, !#field_name_target), #field_name_str, #short));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::DeriveAttributes;
    use crate::test::assert_contains;
    use proc_macro2::Literal;
    use proc_macro2::Span;
    use quote::ToTokens;
    use std::collections::{HashMap, HashSet};

    #[test]
    fn render_derive_parameter_scalar() {
        // Setup
        let parameter = DeriveParameter {
            field_name: ident("my_field"),
            attributes: Default::default(),
            parameter_type: ParameterType::Scalar,
        };

        // Execute
        let token_stream = TokenStream2::try_from(parameter).unwrap();

        // Verify
        assert_eq!(
            token_stream.to_string(),
            "clp = clp . add (Parameter :: argument (Scalar :: new (& mut target . my_field) , \"my_field\")) ;"
        );
    }

    #[test]
    fn render_derive_parameter_collection() {
        // Setup
        let parameter = DeriveParameter {
            field_name: ident("my_field"),
            attributes: Default::default(),
            parameter_type: ParameterType::Collection,
        };

        // Execute
        let token_stream = TokenStream2::try_from(parameter).unwrap();

        // Verify
        assert_eq!(
            token_stream.to_string(),
            "clp = clp . add (Parameter :: argument (Collection :: new (& mut target . my_field , Nargs :: AtLeastOne) , \"my_field\")) ;"
        );
    }

    #[test]
    fn render_derive_parameter_optional() {
        // Setup
        let parameter = DeriveParameter {
            field_name: ident("my_field"),
            attributes: Default::default(),
            parameter_type: ParameterType::Optional,
        };

        // Execute
        let token_stream = TokenStream2::try_from(parameter).unwrap();

        // Verify
        assert_eq!(
            token_stream.to_string(),
            "clp = clp . add (Parameter :: option (Optional :: new (& mut target . my_field) , \"my_field\" , None)) ;"
        );
    }

    #[test]
    fn render_derive_parameter_switch() {
        // Setup
        let parameter = DeriveParameter {
            field_name: ident("my_field"),
            attributes: Default::default(),
            parameter_type: ParameterType::Switch,
        };

        // Execute
        let token_stream = TokenStream2::try_from(parameter).unwrap();

        // Verify
        assert_eq!(
            token_stream.to_string(),
            "let my_field_target = target . my_field . clone () ; clp = clp . add (Parameter :: option (Switch :: new (& mut target . my_field , ! my_field_target) , \"my_field\" , None)) ;"
        );
    }

    #[test]
    fn render_derive_parameter_scalar_with_argument() {
        // Setup
        let parameter = DeriveParameter {
            field_name: ident("my_field"),
            attributes: DeriveAttributes {
                singletons: HashSet::from(["argument".to_string()]),
                pairs: Default::default(),
            },
            parameter_type: ParameterType::Scalar,
        };

        // Execute
        let token_stream = TokenStream2::try_from(parameter).unwrap();

        // Verify
        assert_eq!(
            token_stream.to_string(),
            "clp = clp . add (Parameter :: argument (Scalar :: new (& mut target . my_field) , \"my_field\")) ;"
        );
    }

    #[test]
    fn render_derive_parameter_scalar_with_option() {
        // Setup
        let parameter = DeriveParameter {
            field_name: ident("my_field"),
            attributes: DeriveAttributes {
                singletons: HashSet::from(["option".to_string()]),
                pairs: Default::default(),
            },
            parameter_type: ParameterType::Scalar,
        };

        // Execute
        let token_stream = TokenStream2::try_from(parameter).unwrap();

        // Verify
        assert_eq!(
            token_stream.to_string(),
            "clp = clp . add (Parameter :: option (Scalar :: new (& mut target . my_field) , \"my_field\" , None)) ;"
        );
    }

    #[test]
    fn render_derive_parameter_scalar_with_option_short() {
        // Setup
        let parameter = DeriveParameter {
            field_name: ident("my_field"),
            attributes: DeriveAttributes {
                singletons: HashSet::from(["option".to_string()]),
                pairs: HashMap::from([(
                    "short".to_string(),
                    DeriveValue {
                        tokens: Literal::character('c').into_token_stream(),
                    },
                )]),
            },
            parameter_type: ParameterType::Scalar,
        };

        // Execute
        let token_stream = TokenStream2::try_from(parameter).unwrap();

        // Verify
        assert_eq!(
            token_stream.to_string(),
            "clp = clp . add (Parameter :: option (Scalar :: new (& mut target . my_field) , \"my_field\" , Some ('c'))) ;"
        );
    }

    #[test]
    fn render_derive_parameter_collection_with_argument() {
        // Setup
        let parameter = DeriveParameter {
            field_name: ident("my_field"),
            attributes: DeriveAttributes {
                singletons: HashSet::from(["argument".to_string()]),
                pairs: HashMap::default(),
            },
            parameter_type: ParameterType::Collection,
        };

        // Execute
        let token_stream = TokenStream2::try_from(parameter).unwrap();

        // Verify
        assert_eq!(
            token_stream.to_string(),
            "clp = clp . add (Parameter :: argument (Collection :: new (& mut target . my_field , Nargs :: AtLeastOne) , \"my_field\")) ;"
        );
    }

    #[test]
    fn render_derive_parameter_collection_with_option() {
        // Setup
        let parameter = DeriveParameter {
            field_name: ident("my_field"),
            attributes: DeriveAttributes {
                singletons: HashSet::from(["option".to_string()]),
                pairs: HashMap::default(),
            },
            parameter_type: ParameterType::Collection,
        };

        // Execute
        let token_stream = TokenStream2::try_from(parameter).unwrap();

        // Verify
        assert_eq!(
            token_stream.to_string(),
            "clp = clp . add (Parameter :: option (Collection :: new (& mut target . my_field , Nargs :: AtLeastOne) , \"my_field\" , None)) ;"
        );
    }

    #[test]
    fn render_derive_parameter_explicit_collection() {
        // Setup
        let parameter = DeriveParameter {
            field_name: ident("my_field"),
            attributes: DeriveAttributes {
                singletons: HashSet::default(),
                pairs: HashMap::from([(
                    "collection".to_string(),
                    DeriveValue {
                        tokens: quote! { Nargs::Any },
                    },
                )]),
            },
            parameter_type: ParameterType::Scalar,
        };

        // Execute
        let token_stream = TokenStream2::try_from(parameter).unwrap();

        // Verify
        assert_eq!(
            token_stream.to_string(),
            "clp = clp . add (Parameter :: argument (Collection :: new (& mut target . my_field , Nargs :: Any) , \"my_field\")) ;"
        );
    }

    #[test]
    fn render_derive_parameter_explicit_collection_option() {
        // Setup
        let parameter = DeriveParameter {
            field_name: ident("my_field"),
            attributes: DeriveAttributes {
                singletons: HashSet::from(["option".to_string()]),
                pairs: HashMap::from([(
                    "collection".to_string(),
                    DeriveValue {
                        tokens: quote! { Nargs::Any },
                    },
                )]),
            },
            parameter_type: ParameterType::Scalar,
        };

        // Execute
        let token_stream = TokenStream2::try_from(parameter).unwrap();

        // Verify
        assert_eq!(
            token_stream.to_string(),
            "clp = clp . add (Parameter :: option (Collection :: new (& mut target . my_field , Nargs :: Any) , \"my_field\" , None)) ;"
        );
    }

    #[test]
    fn render_derive_parameter_argument_option() {
        // Setup
        let parameter = DeriveParameter {
            field_name: ident("my_field"),
            attributes: DeriveAttributes {
                singletons: HashSet::from(["argument".to_string(), "option".to_string()]),
                pairs: Default::default(),
            },
            parameter_type: ParameterType::Scalar,
        };

        // Execute
        let error = TokenStream2::try_from(parameter).unwrap_err();

        // Verify
        assert_contains!(error.to_string(), "Invalid");
        assert_contains!(error.to_string(), "blarg(option)");
        assert_contains!(error.to_string(), "blarg(argument)");
    }

    #[test]
    fn render_derive_parameter_option_argument() {
        // Setup
        let parameter = DeriveParameter {
            field_name: ident("my_field"),
            attributes: DeriveAttributes {
                singletons: HashSet::from(["argument".to_string()]),
                pairs: Default::default(),
            },
            parameter_type: ParameterType::Optional,
        };

        // Execute
        let error = TokenStream2::try_from(parameter).unwrap_err();

        // Verify
        assert_contains!(error.to_string(), "Invalid");
        assert_contains!(error.to_string(), "blarg(option)");
    }

    fn ident(name: &str) -> syn::Ident {
        syn::Ident::new(name, Span::call_site())
    }
}
