use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use crate::model::{DeriveParser, DeriveValue};

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
    use crate::model::{DeriveAttributes, DeriveParameter, ParameterType};
    use proc_macro2::Literal;
    use proc_macro2::Span;
    use quote::ToTokens;
    use std::collections::HashMap;

    #[test]
    fn render_derive_parameter_empty() {
        // Setup
        let parser = DeriveParser {
            struct_name: ident("my_struct"),
            attributes: Default::default(),
            parameters: vec![],
        };

        // Execute
        let token_stream = TokenStream2::try_from(parser).unwrap();

        // Verify
        assert_eq!(
            simple_format(token_stream.to_string()),
            r#"impl my_struct {
 fn parse () -> my_struct {
 let mut target = my_struct :: default () ;
 let clp = CommandLineParser :: new (env ! ("CARGO_CRATE_NAME")) ;
 let parser = clp . build () . expect ("Invalid CommandLineParser configuration") ;
 parser . parse () ;
 target }
 }
"#,
        );
    }

    #[test]
    fn render_derive_parameter() {
        // Setup
        let parser = DeriveParser {
            struct_name: ident("my_struct"),
            attributes: DeriveAttributes {
                singletons: Default::default(),
                pairs: HashMap::from([(
                    "program".to_string(),
                    DeriveValue {
                        tokens: Literal::string("abc").into_token_stream(),
                    },
                )]),
            },
            parameters: vec![DeriveParameter {
                field_name: ident("my_field"),
                attributes: Default::default(),
                parameter_type: ParameterType::Scalar,
            }],
        };

        // Execute
        let token_stream = TokenStream2::try_from(parser).unwrap();

        // Verify
        assert_eq!(
            simple_format(token_stream.to_string()),
            r#"impl my_struct {
 fn parse () -> my_struct {
 let mut target = my_struct :: default () ;
 let mut clp = CommandLineParser :: new ("abc") ;
 clp = clp . add (Parameter :: argument (Scalar :: new (& mut target . my_field) , "my_field")) ;
 let parser = clp . build () . expect ("Invalid CommandLineParser configuration") ;
 parser . parse () ;
 target }
 }
"#,
        );
    }

    fn ident(name: &str) -> syn::Ident {
        syn::Ident::new(name, Span::call_site())
    }

    fn simple_format(rust_str: String) -> String {
        rust_str
            .replace("{", "{\n")
            .replace("}", "}\n")
            .replace(";", ";\n")
    }
}
