extern crate proc_macro;

mod generate;
mod load;
mod model;

use crate::model::DeriveParser;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn;

#[proc_macro_derive(BlargParser, attributes(blarg))]
pub fn parser(input: TokenStream) -> TokenStream {
    // https://doc.rust-lang.org/book/ch19-06-macros.html
    let derive_input: syn::DeriveInput = syn::parse(input).unwrap();

    let derive_parser = DeriveParser::from(derive_input);
    TokenStream2::try_from(derive_parser)
        .unwrap_or_else(|error| {
            let compile_error = error.to_compile_error();
            quote! {
                #compile_error
            }
        })
        .into()
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
