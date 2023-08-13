extern crate proc_macro;

mod api;

use crate::api::build_parameters;
use proc_macro::TokenStream;
use quote::quote;
use syn;

#[proc_macro_derive(Parser)]
pub fn parser(input: TokenStream) -> TokenStream {
    // https://doc.rust-lang.org/book/ch19-06-macros.html
    let ast: syn::DeriveInput = syn::parse(input).unwrap();
    let struct_name = &ast.ident;

    match &ast.data {
        syn::Data::Struct(ds) => {
            let cli_parameters = match ds {
                syn::DataStruct {
                    fields: syn::Fields::Named(ref fields),
                    ..
                } => build_parameters(&fields),
                syn::DataStruct { .. } => Vec::default(),
            };
            let clp = if cli_parameters.is_empty() {
                quote! {
                    let clp = CommandLineParser::new(env!("CARGO_CRATE_NAME"));
                }
            } else {
                let fields = cli_parameters
                    .iter()
                    .map(|dp| {
                        let param_name = &dp.name;
                        let param_name_str = format!("{param_name}");
                        quote! {
                            clp = clp.add(Parameter::argument(Scalar::new(&mut target.#param_name), #param_name_str));
                        }
                    })
                    .collect::<Vec<_>>();

                quote! {
                    let mut clp = CommandLineParser::new(env!("CARGO_CRATE_NAME"));
                    #( #fields )*
                }
            };
            let gen = quote! {
                impl #struct_name {
                    fn parse() -> #struct_name {
                        let mut target = #struct_name::default();
                        #clp
                        let parser = clp.build().expect("Invalid CommandLineParser configuration");
                        parser.parse();
                        target
                    }
                }
            };

            gen.into()
        }
        _ => {
            todo!()
        }
    }
}
