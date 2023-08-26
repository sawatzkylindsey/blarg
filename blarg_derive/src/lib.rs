extern crate proc_macro;

mod core;
mod parameter;
mod parser;

use crate::parser::DeriveParser;
use proc_macro::TokenStream;
use syn;
use syn::__private::TokenStream2;

#[proc_macro_derive(Parser, attributes(parser))]
pub fn parser(input: TokenStream) -> TokenStream {
    // https://doc.rust-lang.org/book/ch19-06-macros.html
    let derive_input: syn::DeriveInput = syn::parse(input).unwrap();

    let cli_parser =
        DeriveParser::try_from(derive_input).expect("Invalid derive Parser configuration.");
    TokenStream2::from(cli_parser).into()
}
