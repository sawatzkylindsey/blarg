extern crate proc_macro;

mod core;
mod parameter;
mod parser;

use crate::parser::DeriveParser;
use proc_macro::TokenStream;
use quote::quote;
use syn;
use syn::__private::TokenStream2;

#[proc_macro_derive(BlargParser, attributes(blarg))]
pub fn parser(input: TokenStream) -> TokenStream {
    // https://doc.rust-lang.org/book/ch19-06-macros.html
    let derive_input: syn::DeriveInput = syn::parse(input).unwrap();

    let cli_parser = DeriveParser::from(derive_input);
    TokenStream2::try_from(cli_parser)
        .unwrap_or_else(|error| {
            let compile_error = error.to_compile_error();
            quote! {
                #compile_error
            }
        })
        .into()
}
