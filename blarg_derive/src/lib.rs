extern crate proc_macro;

mod generate;
mod load;
mod model;

use crate::model::{DeriveChoices, DeriveParser, DeriveSubParser};
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn;

pub(crate) const MACRO_BLARG_PARSER: &str = "BlargParser";

/// The primary derive macro which turns a struct into a `CommandLineParser`.
///
/// Supports the following struct attributes:
/// * `#[blarg(program = "..")]` explicitly sets the name of your Cli program.
/// When unspecified, defaults to the name of the cargo crate.
/// * `#[blarg(initializer = F)]` instructs `blarg` to use the initializer method `F`.
/// This allows for a separation between the `Default` method vs. *initial* values of the struct, which follows from `blargs`'s stance on [default & initials](../index.html#defaults--initials).
/// When unspecified, `blarg` falls back to the initializer method `default`.
/// * `#[blarg(hints_off)]` disables the type/initial documentation hints.
/// When unspecified, `blarg` automatically generates type/initial documentation via the "meta" documentation mechanism ([parameter meta](../struct.Parameter.html#method.meta) or [condition meta](../struct.Condition.html#method.meta)).
///
/// Refer to [blarg::derive](../derive/index.html#parameter-configuration) to configure the fields of this struct.
///
/// ### Example
/// ```ignore
/// #[derive(BlargParser)]
/// #[blarg(program = "my-program", initializer = init, hints_off)]
/// struct MyCli {
/// }
///
/// impl MyCli {
///     fn init() -> Self {
///         todo!()
///     }
/// }
/// ```
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

pub(crate) const MACRO_BLARG_SUB_PARSER: &str = "BlargSubParser";

/// The derive macro which turns a struct into a `SubCommandParser`.
///
/// Supports the following struct attributes:
/// * `#[blarg(hints_off)]` disables the type/initial documentation hints.
/// When unspecified, `blarg` automatically generates type/initial documentation via the "meta" documentation mechanism ([parameter meta](../struct.Parameter.html#method.meta) or [condition meta](../struct.Condition.html#method.meta)).
///
/// Additionally, take note: the *initializer* method is inherited from that of the [`BlargParser`].
///
/// Refer to [blarg::derive](../derive/index.html#parameter-configuration) to configure the fields of this struct.
///
/// ### Example
/// ```ignore
/// #[derive(BlargSubParser)]
/// #[blarg(hints_off)]
/// struct MySubCli {
/// }
///
/// // Assuming the `BlargParser` struct uses `#[blarg(initializer = init)]`, then we must also implement `init` on the sub-command struct.
/// impl MySubCli {
///     fn init() -> Self {
///         todo!()
///     }
/// }
/// ```
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

pub(crate) const MACRO_BLARG_CHOICES: &str = "BlargChoices";

/// Derive macro specific to generate a choices [help message](../derive/index.html#help-messages).
///
/// Supports the no enum attributes.
///
/// Refer to [blarg::derive](../derive/index.html#choices) to configure the variants of this enum.
///
/// ### Example
/// ```ignore
/// #[derive(BlargChoices)]
/// enum MyEnum {
///     A,
///     B,
/// }
/// ```
#[proc_macro_derive(BlargChoices, attributes(blarg))]
pub fn choices(input: TokenStream) -> TokenStream {
    // https://doc.rust-lang.org/book/ch19-06-macros.html
    let derive_input: syn::DeriveInput = syn::parse(input).unwrap();

    match DeriveChoices::try_from(derive_input) {
        Err(error) => {
            let compile_error = error.to_compile_error();
            quote! {
                #compile_error
            }
            .into()
        }
        Ok(derive_choices) => TokenStream2::from(derive_choices).into(),
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
