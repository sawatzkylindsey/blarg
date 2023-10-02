use crate::{
    model::{DeriveChoices, DeriveValue, DeriveVariant, IntermediateAttributes},
    MACRO_BLARG_CHOICES,
};

impl TryFrom<syn::DeriveInput> for DeriveChoices {
    type Error = syn::Error;

    fn try_from(value: syn::DeriveInput) -> Result<Self, Self::Error> {
        let parser_name = &value.ident;

        match &value.data {
            syn::Data::Enum(de) => {
                let variants = de
                    .variants
                    .iter()
                    .map(DeriveVariant::try_from)
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(DeriveChoices {
                    struct_name: parser_name.clone(),
                    variants,
                })
            }
            _ => Err(syn::Error::new(
                parser_name.span(),
                format!("Invalid - {MACRO_BLARG_CHOICES} only applies to 'enum' data structures."),
            )),
        }
    }
}

impl TryFrom<&syn::Variant> for DeriveVariant {
    type Error = syn::Error;

    fn try_from(value: &syn::Variant) -> Result<Self, Self::Error> {
        let mut attributes = IntermediateAttributes::default();

        for attribute in &value.attrs {
            if attribute.path().is_ident("blarg") {
                attributes = IntermediateAttributes::from(attribute);
            }
        }

        let field_name = value.ident.clone();
        let explicit_hidden = attributes.singletons.contains("hidden");
        let help = match attributes.pairs.get("help") {
            Some(values) => {
                let tokens = values
                    .first()
                    .expect("attribute pair 'help' must contain non-empty values")
                    .tokens
                    .clone();
                Some(DeriveValue { tokens })
            }
            None => None,
        };

        Ok(DeriveVariant {
            field_name,
            hidden: explicit_hidden,
            help,
        })
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
    fn construct_derive_choices_empty() {
        // Setup
        let input: syn::DeriveInput = syn::parse_str(
            r#"
                #[derive(BlargChoices)]
                enum Values { }
            "#,
        )
        .unwrap();

        // Execute
        let derive_choices = DeriveChoices::try_from(input).unwrap();

        // Verify
        assert_eq!(
            derive_choices,
            DeriveChoices {
                struct_name: ident("Values"),
                variants: Vec::default(),
            }
        );
    }

    #[test]
    fn construct_derive_choices() {
        // Setup
        let input: syn::DeriveInput = syn::parse_str(
            r#"
                #[derive(BlargChoices)]
                enum Values {
                    Abc,
                    #[blarg(help = "def")]
                    Def,
                    #[blarg(help = "ghi", hidden)]
                    Ghi,
                    #[blarg(hidden)]
                    Jkl,
                }
            "#,
        )
        .unwrap();

        // Execute
        let derive_choices = DeriveChoices::try_from(input).unwrap();

        // Verify
        assert_eq!(
            derive_choices,
            DeriveChoices {
                struct_name: ident("Values"),
                variants: vec![
                    DeriveVariant {
                        field_name: ident("Abc"),
                        hidden: false,
                        help: None,
                    },
                    DeriveVariant {
                        field_name: ident("Def"),
                        hidden: false,
                        help: Some(DeriveValue {
                            tokens: Literal::string("def").into_token_stream(),
                        }),
                    },
                    DeriveVariant {
                        field_name: ident("Ghi"),
                        hidden: true,
                        help: Some(DeriveValue {
                            tokens: Literal::string("ghi").into_token_stream(),
                        }),
                    },
                    DeriveVariant {
                        field_name: ident("Jkl"),
                        hidden: true,
                        help: None,
                    },
                ],
            }
        );
    }

    #[test]
    fn construct_derive_choices_invalid() {
        // Setup
        let input: syn::DeriveInput = syn::parse_str(
            r#"
                #[derive(BlargChoices)]
                struct Values { }
            "#,
        )
        .unwrap();

        // Execute
        let error = DeriveChoices::try_from(input).unwrap_err();

        // Verify
        assert_eq!(
            error.to_string(),
            "Invalid - BlargChoices only applies to 'enum' data structures."
        );
    }

    fn ident(name: &str) -> syn::Ident {
        syn::Ident::new(name, Span::call_site())
    }
}
