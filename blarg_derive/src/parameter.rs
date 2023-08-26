use quote::quote;
use syn::__private::TokenStream2;

#[derive(Debug)]
pub struct DeriveParameter {
    pub name: syn::Ident,
    pub parameter_type: ParameterType,
}

impl From<&syn::Field> for DeriveParameter {
    fn from(value: &syn::Field) -> Self {
        let parameter_type = match &value.ty {
            syn::Type::Path(path) => match &path.path.segments.first() {
                Some(segment) => {
                    if segment.ident == "Option" {
                        ParameterType::Option
                    } else {
                        ParameterType::Argument
                    }
                }
                None => ParameterType::Argument,
            },
            _ => ParameterType::Argument,
        };

        DeriveParameter {
            name: value.ident.clone().unwrap(),
            parameter_type,
        }
    }
}

impl From<DeriveParameter> for TokenStream2 {
    fn from(value: DeriveParameter) -> Self {
        let DeriveParameter {
            name,
            parameter_type,
        } = value;
        let name_str = format!("{name}");

        match parameter_type {
            ParameterType::Argument => {
                quote! {
                    clp = clp.add(Parameter::argument(Scalar::new(&mut target.#name), #name_str));
                }
            }
            ParameterType::Option => {
                quote! {
                    clp = clp.add(Parameter::option(Optional::new(&mut target.#name), #name_str, None));
                }
            }
        }
    }
}

#[derive(Debug)]
pub enum ParameterType {
    Argument,
    Option,
}
