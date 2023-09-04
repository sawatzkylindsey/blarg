use crate::core::{DeriveAttributes, DeriveValue};
use quote::__private::TokenStream;
use quote::{format_ident, quote};
use syn::__private::TokenStream2;

#[derive(Debug, PartialEq, Eq)]
pub struct DeriveParameter {
    pub field_name: syn::Ident,
    pub attributes: DeriveAttributes,
    pub parameter_type: ParameterType,
}

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
                    unreachable!("woops a");
                }
            },
            _ => {
                unreachable!("woops b");
            }
        };

        DeriveParameter {
            field_name: value.ident.clone().unwrap(),
            attributes,
            parameter_type,
        }
    }
}

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

        // match (parameter_type, has_option_attribute) {
        //     (ParameterType::Argument, false) => {
        //         quote! {
        //             clp = clp.add(Parameter::argument(Scalar::new(&mut target.#field_name), #field_name_str));
        //         }
        //     }
        //     (ParameterType::Argument, true) | (ParameterType::Option, _) => {}
        // }
    }
}

fn build_argument(
    parameter_type: ParameterType,
    explicit_collection: bool,
    field_name: syn::Ident,
    field_name_str: String,
    nargs: TokenStream,
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
    nargs: TokenStream,
    short: TokenStream,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParameterType {
    Scalar,
    Optional,
    Collection,
    Switch,
}
