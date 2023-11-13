use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};

use crate::model::{DeriveParser, DeriveSubParser, ParameterType};

impl From<DeriveParser> for TokenStream2 {
    fn from(value: DeriveParser) -> Self {
        let DeriveParser {
            struct_name,
            program,
            initializer,
            parameters,
            hints,
        } = value;
        let program = program.tokens;
        let initializer = initializer.tokens;

        let struct_target = format_ident!("{struct_name}_target");
        let mut sub_struct_initializers = quote! {};
        let mut structs_signature = quote! { #struct_name };
        let mut structs_return = quote! { #struct_target };

        for parameter in &parameters {
            if let ParameterType::Condition { commands } = &parameter.parameter_type {
                let subs: Vec<TokenStream2> = commands
                    .iter()
                    .map(|c| {
                        let command_struct = &c.command_struct.tokens;
                        let field_name_target = format_ident!("{command_struct}_target");

                        quote! {
                            let mut #field_name_target = <#command_struct>::#initializer();
                        }
                    })
                    .collect();
                sub_struct_initializers = quote! {
                    #( #subs )*
                };

                let subs: Vec<TokenStream2> = commands
                    .iter()
                    .map(|c| {
                        let command_struct = &c.command_struct.tokens;

                        quote! {
                            #command_struct
                        }
                    })
                    .collect();
                structs_signature = quote! { (#struct_name, #( #subs ),*) };

                let subs: Vec<TokenStream2> = commands
                    .iter()
                    .map(|c| {
                        let command_struct = &c.command_struct.tokens;
                        let field_name_target = format_ident!("{command_struct}_target");

                        quote! {
                            #field_name_target
                        }
                    })
                    .collect();
                structs_return = quote! { (#struct_target, #( #subs ),*) };

                // There is at most 1 Condition per CommandLineParser.
                break;
            }
        }

        let clp = if parameters.is_empty() {
            quote! {
                let clp = CommandLineParser::new(#program);
            }
        } else {
            let fields: Vec<_> = parameters
                .into_iter()
                .map(|p| p.generate(&struct_target, &hints))
                .collect();

            quote! {
                let mut clp = CommandLineParser::new(#program);
                #( #fields )*
            }
        };

        quote! {
            impl #struct_name {
                /// Generated by BlargParser
                pub fn blarg_parse() -> #structs_signature {
                    let mut #struct_target = <#struct_name>::#initializer();
                    #sub_struct_initializers
                    #clp
                    let parser = clp.build();
                    parser.parse();
                    #structs_return
                }
            }
        }
        .into()
    }
}

impl From<DeriveSubParser> for TokenStream2 {
    fn from(value: DeriveSubParser) -> Self {
        let DeriveSubParser {
            struct_name,
            parameters,
            hints,
        } = value;

        let struct_target = format_ident!("{struct_name}_target");
        let fields: Vec<_> = parameters
            .into_iter()
            .map(|p| p.generate(&struct_target, &hints))
            .collect();

        let clp = if fields.is_empty() {
            quote! { |clp| clp }
        } else {
            quote! {
                |mut clp| {
                    #( #fields )*
                    clp
                }
            }
        };

        quote! {
                impl #struct_name {
                    /// Generated by BlargSubParser
                    pub fn setup_command<'a>(#struct_target: &'a mut #struct_name) -> impl FnOnce(SubCommand<'a>) -> SubCommand<'a> {
                        #clp
                    }
                }
            }
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Command, DeriveParameter, DeriveValue, Hints, ParameterType};
    use proc_macro2::Literal;
    use proc_macro2::Span;
    use quote::ToTokens;

    #[test]
    fn render_derive_parser_empty() {
        // Setup
        let parser = DeriveParser {
            struct_name: ident("my_struct"),
            initializer: DeriveValue {
                tokens: quote! { default }.into_token_stream(),
            },
            program: DeriveValue {
                tokens: quote! { env!("CARGO_CRATE_NAME") },
            },
            parameters: vec![],
            hints: Hints::Off,
        };

        // Execute
        let token_stream = TokenStream2::try_from(parser).unwrap();

        // Verify
        assert_eq!(
            simple_format(token_stream.to_string()),
            r#"impl my_struct {
 # [doc = r" Generated by BlargParser"] pub fn blarg_parse () -> my_struct {
 let mut my_struct_target = < my_struct > :: default () ;
 let clp = CommandLineParser :: new (env ! ("CARGO_CRATE_NAME")) ;
 let parser = clp . build () ;
 parser . parse () ;
 my_struct_target }
 }
"#,
        );
    }

    #[test]
    fn render_derive_parser() {
        // Setup
        let parser = DeriveParser {
            struct_name: ident("my_struct"),
            program: DeriveValue {
                tokens: Literal::string("abc").into_token_stream(),
            },
            initializer: DeriveValue {
                tokens: quote! { default }.into_token_stream(),
            },
            parameters: vec![DeriveParameter {
                field_name: ident("my_field"),
                from_str_type: "usize".to_string(),
                parameter_type: ParameterType::ScalarArgument,
                choices: None,
                help: None,
            }],
            hints: Hints::Off,
        };

        // Execute
        let token_stream = TokenStream2::try_from(parser).unwrap();

        // Verify
        assert_eq!(
            simple_format(token_stream.to_string()),
            r#"impl my_struct {
 # [doc = r" Generated by BlargParser"] pub fn blarg_parse () -> my_struct {
 let mut my_struct_target = < my_struct > :: default () ;
 let mut clp = CommandLineParser :: new ("abc") ;
 clp = clp . add (Parameter :: argument (Scalar :: new (& mut my_struct_target . my_field) , "my_field")) ;
 let parser = clp . build () ;
 parser . parse () ;
 my_struct_target }
 }
"#,
        );
    }

    #[test]
    fn render_derive_parser_condition() {
        // Setup
        let parser = DeriveParser {
            struct_name: ident("my_struct"),
            program: DeriveValue {
                tokens: Literal::string("abc").into_token_stream(),
            },
            initializer: DeriveValue {
                tokens: quote! { default }.into_token_stream(),
            },
            parameters: vec![DeriveParameter {
                field_name: ident("my_field"),
                from_str_type: "usize".to_string(),
                parameter_type: ParameterType::Condition {
                    commands: vec![
                        Command {
                            variant: DeriveValue {
                                tokens: Literal::usize_unsuffixed(0).into_token_stream(),
                            },
                            command_struct: DeriveValue {
                                tokens: ident("Abc").to_token_stream(),
                            },
                        },
                        Command {
                            variant: DeriveValue {
                                tokens: Literal::usize_unsuffixed(1).into_token_stream(),
                            },
                            command_struct: DeriveValue {
                                tokens: ident("Def").to_token_stream(),
                            },
                        },
                    ],
                },
                choices: None,
                help: None,
            }],
            hints: Hints::Off,
        };

        // Execute
        let token_stream = TokenStream2::try_from(parser).unwrap();

        // Verify
        assert_eq!(
            simple_format(token_stream.to_string()),
            r#"impl my_struct {
 # [doc = r" Generated by BlargParser"] pub fn blarg_parse () -> (my_struct , Abc , Def) {
 let mut my_struct_target = < my_struct > :: default () ;
 let mut Abc_target = < Abc > :: default () ;
 let mut Def_target = < Def > :: default () ;
 let mut clp = CommandLineParser :: new ("abc") ;
 let mut clp = clp . branch (Condition :: new (Scalar :: new (& mut my_struct_target . my_field) , "my_field")) ;
 clp = clp . command (0 , Abc :: setup_command (& mut Abc_target)) ;
 clp = clp . command (1 , Def :: setup_command (& mut Def_target)) ;
 let parser = clp . build () ;
 parser . parse () ;
 (my_struct_target , Abc_target , Def_target) }
 }
"#,
        );
    }

    #[test]
    fn render_derive_sub_parser_empty() {
        // Setup
        let sub_parser = DeriveSubParser {
            struct_name: ident("my_struct"),
            parameters: vec![],
            hints: Hints::Off,
        };

        // Execute
        let token_stream = TokenStream2::try_from(sub_parser).unwrap();

        // Verify
        assert_eq!(
            simple_format(token_stream.to_string()),
            r#"impl my_struct {
 # [doc = r" Generated by BlargSubParser"] pub fn setup_command < 'a > (my_struct_target : & 'a mut my_struct) -> impl FnOnce (SubCommand < 'a >) -> SubCommand < 'a > {
 | clp | clp }
 }
"#,
        );
    }

    #[test]
    fn render_derive_sub_parser() {
        // Setup
        let sub_parser = DeriveSubParser {
            struct_name: ident("my_struct"),
            parameters: vec![DeriveParameter {
                field_name: ident("my_field"),
                from_str_type: "usize".to_string(),
                parameter_type: ParameterType::ScalarArgument,
                choices: None,
                help: None,
            }],
            hints: Hints::Off,
        };

        // Execute
        let token_stream = TokenStream2::try_from(sub_parser).unwrap();

        // Verify
        assert_eq!(
            simple_format(token_stream.to_string()),
            r#"impl my_struct {
 # [doc = r" Generated by BlargSubParser"] pub fn setup_command < 'a > (my_struct_target : & 'a mut my_struct) -> impl FnOnce (SubCommand < 'a >) -> SubCommand < 'a > {
 | mut clp | {
 clp = clp . add (Parameter :: argument (Scalar :: new (& mut my_struct_target . my_field) , "my_field")) ;
 clp }
 }
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
