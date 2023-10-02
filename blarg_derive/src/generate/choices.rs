use crate::model::{DeriveChoices, DeriveVariant};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

impl From<DeriveChoices> for TokenStream2 {
    fn from(value: DeriveChoices) -> Self {
        let DeriveChoices {
            struct_name,
            variants,
        } = value;

        let choices: Vec<_> = variants
            .into_iter()
            .filter_map(|v| v.generate(&struct_name))
            .collect();
        quote! {
            impl #struct_name {
                fn blarg_choices<T: Choices<#struct_name>>(mut value: T) -> T {
                    #( #choices )*
                    value
                }
            }
        }
        .into()
    }
}

impl DeriveVariant {
    fn generate(self, parent: &syn::Ident) -> Option<TokenStream2> {
        let DeriveVariant {
            field_name,
            hidden,
            help,
        } = self;

        if !hidden {
            if let Some(help) = help {
                let help = help.tokens;
                Some(quote! {
                    value = value.choice(#parent::#field_name, #help);
                })
            } else {
                Some(quote! {
                    value = value.choice(#parent::#field_name, "");
                })
            }
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::DeriveValue;
    use proc_macro2::Literal;
    use proc_macro2::Span;
    use quote::ToTokens;

    #[test]
    fn render_derive_choices_empty() {
        // Setup
        let choices = DeriveChoices {
            struct_name: ident("my_struct"),
            variants: vec![],
        };

        // Execute
        let token_stream = TokenStream2::try_from(choices).unwrap();

        // Verify
        assert_eq!(
            simple_format(token_stream.to_string()),
            r#"impl my_struct {
 fn blarg_choices < T : Choices < my_struct >> (mut value : T) -> T {
 value }
 }
"#,
        );
    }

    #[test]
    fn render_derive_choices() {
        // Setup
        let choices = DeriveChoices {
            struct_name: ident("my_struct"),
            variants: vec![DeriveVariant {
                field_name: ident("Abc"),
                hidden: false,
                help: None,
            }],
        };

        // Execute
        let token_stream = TokenStream2::try_from(choices).unwrap();

        // Verify
        assert_eq!(
            simple_format(token_stream.to_string()),
            r#"impl my_struct {
 fn blarg_choices < T : Choices < my_struct >> (mut value : T) -> T {
 value = value . choice (my_struct :: Abc , "") ;
 value }
 }
"#,
        );
    }

    #[test]
    fn render_derive_variant_empty() {
        // Setup
        let variant = DeriveVariant {
            field_name: ident("Abc"),
            hidden: false,
            help: None,
        };

        // Execute
        let token_stream = variant.generate(&ident("target")).unwrap();

        // Verify
        assert_eq!(
            token_stream.to_string(),
            "value = value . choice (target :: Abc , \"\") ;"
        );
    }

    #[test]
    fn render_derive_variant() {
        // Setup
        let variant = DeriveVariant {
            field_name: ident("Abc"),
            hidden: false,
            help: Some(DeriveValue {
                tokens: Literal::string("abc 123").to_token_stream(),
            }),
        };

        // Execute
        let token_stream = variant.generate(&ident("target")).unwrap();

        // Verify
        assert_eq!(
            token_stream.to_string(),
            "value = value . choice (target :: Abc , \"abc 123\") ;"
        );
    }

    #[test]
    fn render_derive_variant_hidden() {
        // Setup
        let variant = DeriveVariant {
            field_name: ident("Abc"),
            hidden: true,
            help: Some(DeriveValue {
                tokens: Literal::string("abc 123").to_token_stream(),
            }),
        };

        // Execute
        let result = variant.generate(&ident("target"));

        // Verify
        assert!(result.is_none());
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