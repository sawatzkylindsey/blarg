use quote::quote;
use syn::__private::TokenStream2;

use crate::core::{DeriveAttributes, DeriveValue};
use crate::parameter::DeriveParameter;

#[derive(Debug)]
pub struct DeriveParser {
    pub struct_name: syn::Ident,
    pub blarg: DeriveAttributes,
    pub parameters: Vec<DeriveParameter>,
}

impl TryFrom<syn::DeriveInput> for DeriveParser {
    type Error = syn::parse::Error;

    fn try_from(value: syn::DeriveInput) -> Result<Self, Self::Error> {
        let mut blarg = DeriveAttributes::default();

        for attribute in &value.attrs {
            if attribute.path().is_ident("blarg") {
                blarg = DeriveAttributes::from(attribute);
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
                    blarg,
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

impl From<DeriveParser> for TokenStream2 {
    fn from(value: DeriveParser) -> Self {
        let DeriveParser {
            struct_name,
            blarg,
            parameters,
        } = value;
        let program_name = match blarg.pairs.get("program") {
            Some(DeriveValue::Literal(ts)) => quote! { #ts },
            None => quote! { env!("CARGO_CRATE_NAME") },
        };

        let clp = if parameters.is_empty() {
            quote! {
                let clp = CommandLineParser::new(#program_name);
            }
        } else {
            let fields = parameters
                .into_iter()
                .map(TokenStream2::from)
                .collect::<Vec<_>>();

            quote! {
                let mut clp = CommandLineParser::new(#program_name);
                #( #fields )*
            }
        };

        quote! {
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
        .into()
    }
}
