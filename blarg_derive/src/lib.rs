extern crate proc_macro;

mod generate;
mod load;
mod model;

use crate::model::{DeriveParser, DeriveSubParser};
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn;

#[proc_macro_derive(BlargParser, attributes(blarg))]
pub fn parser(input: TokenStream) -> TokenStream {
    // https://doc.rust-lang.org/book/ch19-06-macros.html
    let derive_input: syn::DeriveInput = syn::parse(input).unwrap();

    match DeriveParser::try_from(derive_input) {
        Err(error) => {
            let compile_error = error.to_compile_error();
            quote! {
                #compile_error
            }
            .into()
        }
        Ok(derive_parser) => TokenStream2::from(derive_parser).into(),
    }
}

#[proc_macro_derive(BlargSubParser, attributes(blarg))]
pub fn sub_parser(input: TokenStream) -> TokenStream {
    // https://doc.rust-lang.org/book/ch19-06-macros.html
    let derive_input: syn::DeriveInput = syn::parse(input).unwrap();

    match DeriveSubParser::try_from(derive_input) {
        Err(error) => {
            let compile_error = error.to_compile_error();
            quote! {
                #compile_error
            }
            .into()
        }
        Ok(derive_sub_parser) => TokenStream2::from(derive_sub_parser).into(),
    }
}

#[cfg(test)]
pub(crate) mod test {
    macro_rules! assert_contains {
        ($base:expr, $sub:expr) => {
            assert!(
                $base.contains($sub),
                "'{b}' does not contain '{s}'",
                b = $base,
                s = $sub,
            );
        };
    }

    pub(crate) use assert_contains;
}
