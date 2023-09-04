use quote::quote;
use syn::__private::TokenStream2;

use crate::core::{DeriveAttributes, DeriveValue};
use crate::parameter::DeriveParameter;

#[derive(Debug, PartialEq, Eq)]
pub struct DeriveParser {
    pub struct_name: syn::Ident,
    pub attributes: DeriveAttributes,
    pub parameters: Vec<DeriveParameter>,
}

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

impl TryFrom<DeriveParser> for TokenStream2 {
    type Error = syn::Error;

    fn try_from(value: DeriveParser) -> Result<Self, Self::Error> {
        let DeriveParser {
            struct_name,
            attributes,
            parameters,
        } = value;
        let program_name = match attributes.pairs.get("program") {
            Some(DeriveValue { tokens }) => quote! { #tokens },
            None => quote! { env!("CARGO_CRATE_NAME") },
        };

        let clp = if parameters.is_empty() {
            quote! {
                let clp = CommandLineParser::new(#program_name);
            }
        } else {
            let fields = parameters
                .into_iter()
                .map(TokenStream2::try_from)
                .collect::<Result<Vec<_>, _>>()?;

            quote! {
                let mut clp = CommandLineParser::new(#program_name);
                #( #fields )*
            }
        };

        Ok(quote! {
            impl #struct_name {
                fn parse() -> #struct_name {
                    let mut target = #struct_name::default();
                    #clp
                    let parser = clp.build().expect("Invalid CommandLineParser configuration");
                    parser.parse();
                    target
                }
            }
        }
        .into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn construct_direct_parser() {
        // Setup
        let di: syn::DeriveInput = syn::parse_str(
            r#"
                #[derive(Default, BlargParser)]
                struct Parameters {
                    apple: usize,
                }
            "#,
        )
        .unwrap();

        // Execute
        let cli_parser = DeriveParser::from(di);

        // Verify
    }
}
