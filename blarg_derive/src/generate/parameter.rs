use crate::model::{Command, DeriveParameter, DeriveValue, ParameterType};
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};

impl DeriveParameter {
    pub(crate) fn generate(self, parent: &syn::Ident) -> TokenStream2 {
        let DeriveParameter {
            field_name,
            parameter_type,
            help,
        } = self;
        let field_name_str = format!("{field_name}");

        match parameter_type {
            ParameterType::CollectionArgument { nargs } => {
                let nargs = nargs.tokens;
                if let Some(help) = help {
                    let help = help.tokens;
                    quote! {
                        clp = clp.add(Parameter::argument(Collection::new(&mut #parent.#field_name, #nargs), #field_name_str)
                            .help(#help));
                    }
                } else {
                    quote! {
                        clp = clp.add(Parameter::argument(Collection::new(&mut #parent.#field_name, #nargs), #field_name_str));
                    }
                }
            }
            ParameterType::ScalarArgument => {
                if let Some(help) = help {
                    let help = help.tokens;
                    quote! {
                        clp = clp.add(Parameter::argument(Scalar::new(&mut #parent.#field_name), #field_name_str)
                            .help(#help));
                    }
                } else {
                    quote! {
                        clp = clp.add(Parameter::argument(Scalar::new(&mut #parent.#field_name), #field_name_str));
                    }
                }
            }

            ParameterType::CollectionOption { nargs, short } => {
                let nargs = nargs.tokens;
                let short = flatten(short);
                let field_default = format_ident!("{field_name}_default");

                if let Some(help) = help {
                    let help = help.tokens;
                    quote! {
                        let #field_default = format!("{:?}", #parent.#field_name);
                        clp = clp.add(Parameter::option(Collection::new(&mut #parent.#field_name, #nargs), #field_name_str, #short)
                            .help(format!("{} (default {})", #help, #field_default)));
                    }
                } else {
                    quote! {
                        let #field_default = format!("{:?}", #parent.#field_name);
                        clp = clp.add(Parameter::option(Collection::new(&mut #parent.#field_name, #nargs), #field_name_str, #short)
                            .help(format!("(default {})", #field_default)));
                    }
                }
            }
            ParameterType::ScalarOption { short } => {
                let short = flatten(short);
                let field_default = format_ident!("{field_name}_default");

                if let Some(help) = help {
                    let help = help.tokens;
                    quote! {
                        let #field_default = #parent.#field_name.to_string();
                        clp = clp.add(Parameter::option(Scalar::new(&mut #parent.#field_name), #field_name_str, #short)
                            .help(format!("{} (default {})", #help, #field_default)));
                    }
                } else {
                    quote! {
                        let #field_default = #parent.#field_name.to_string();
                        clp = clp.add(Parameter::option(Scalar::new(&mut #parent.#field_name), #field_name_str, #short)
                            .help(format!("(default {})", #field_default)));
                    }
                }
            }
            ParameterType::OptionalOption { short } => {
                let short = flatten(short);
                let field_default = format_ident!("{field_name}_default");

                if let Some(help) = help {
                    let help = help.tokens;
                    quote! {
                        if let Some(inner) = #parent.#field_name.as_ref() {
                            let #field_default = format!("{inner}");
                            clp = clp.add(Parameter::option(Optional::new(&mut #parent.#field_name), #field_name_str, #short)
                                .help(format!("{} (default {})", #help, #field_default)));
                        } else {
                            clp = clp.add(Parameter::option(Optional::new(&mut #parent.#field_name), #field_name_str, #short)
                                .help(#help));
                        }
                    }
                } else {
                    quote! {
                        if let Some(inner) = #parent.#field_name.as_ref() {
                            let #field_default = format!("{inner}");
                            clp = clp.add(Parameter::option(Optional::new(&mut #parent.#field_name), #field_name_str, #short)
                                .help(format!("(default {})", #field_default)));
                        } else {
                            clp = clp.add(Parameter::option(Optional::new(&mut #parent.#field_name), #field_name_str, #short));
                        }
                    }
                }
            }

            ParameterType::Switch { short } => {
                let short = flatten(short);
                let field_name_target = format_ident!("{field_name}_target");

                if let Some(help) = help {
                    let help = help.tokens;
                    quote! {
                        let #field_name_target = #parent.#field_name.clone();
                        clp = clp.add(Parameter::option(Switch::new(&mut #parent.#field_name, !#field_name_target), #field_name_str, #short)
                            .help(#help));
                    }
                } else {
                    quote! {
                        let #field_name_target = #parent.#field_name.clone();
                        clp = clp.add(Parameter::option(Switch::new(&mut #parent.#field_name, !#field_name_target), #field_name_str, #short));
                    }
                }
            }
            ParameterType::Condition { commands } => {
                let commands: Vec<_> = commands
                    .into_iter()
                    .map(|command| {
                        let Command {
                            variant,
                            command_struct,
                        } = command;
                        let variant = variant.tokens;
                        let command_struct = &command_struct.tokens;
                        let command_struct_target = format_ident!("{command_struct}_target");
                        quote! {
                            clp = clp.command(#variant, #command_struct::setup_command(&mut #command_struct_target));
                        }
                    })
                    .collect();
                if let Some(help) = help {
                    let help = help.tokens;
                    quote! {
                        let mut clp = clp.branch(Condition::new(Scalar::new(&mut #parent.#field_name), #field_name_str)
                            .help(#help));
                        #( #commands )*
                    }
                } else {
                    quote! {
                        let mut clp = clp.branch(Condition::new(Scalar::new(&mut #parent.#field_name), #field_name_str));
                        #( #commands )*
                    }
                }
            }
        }
    }
}

fn flatten(value: Option<DeriveValue>) -> TokenStream2 {
    value.map_or_else(
        || quote! { None },
        |s| {
            let tokens = s.tokens;
            quote! { Some(#tokens) }
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use proc_macro2::Literal;
    use proc_macro2::Span;
    use quote::ToTokens;

    #[test]
    fn render_collection_argument() {
        // Setup
        let parameter = DeriveParameter {
            field_name: ident("my_field"),
            parameter_type: ParameterType::CollectionArgument {
                nargs: DeriveValue {
                    tokens: quote! { Nargs::AtLeastOne },
                },
            },
            help: None,
        };

        // Execute
        let token_stream = parameter.generate(&ident("target"));

        // Verify
        assert_eq!(
            token_stream.to_string(),
            "clp = clp . add (Parameter :: argument (Collection :: new (& mut target . my_field , Nargs :: AtLeastOne) , \"my_field\")) ;"
        );
    }

    #[test]
    fn render_collection_argument_help() {
        // Setup
        let parameter = DeriveParameter {
            field_name: ident("my_field"),
            parameter_type: ParameterType::CollectionArgument {
                nargs: DeriveValue {
                    tokens: quote! { Nargs::AtLeastOne },
                },
            },
            help: Some(DeriveValue {
                tokens: Literal::string("abc 123").to_token_stream(),
            }),
        };

        // Execute
        let token_stream = parameter.generate(&ident("target"));

        // Verify
        assert_eq!(
            simple_format(token_stream.to_string()),
            r#"clp = clp . add (Parameter :: argument (Collection :: new (& mut target . my_field , Nargs :: AtLeastOne) , "my_field") . help ("abc 123")) ;
"#
        );
    }

    #[test]
    fn render_scalar_argument() {
        // Setup
        let parameter = DeriveParameter {
            field_name: ident("my_field"),
            parameter_type: ParameterType::ScalarArgument,
            help: None,
        };

        // Execute
        let token_stream = parameter.generate(&ident("target"));

        // Verify
        assert_eq!(
            token_stream.to_string(),
            "clp = clp . add (Parameter :: argument (Scalar :: new (& mut target . my_field) , \"my_field\")) ;"
        );
    }

    #[test]
    fn render_scalar_argument_help() {
        // Setup
        let parameter = DeriveParameter {
            field_name: ident("my_field"),
            parameter_type: ParameterType::ScalarArgument,
            help: Some(DeriveValue {
                tokens: Literal::string("abc 123").to_token_stream(),
            }),
        };

        // Execute
        let token_stream = parameter.generate(&ident("target"));

        // Verify
        assert_eq!(
            simple_format(token_stream.to_string()),
            r#"clp = clp . add (Parameter :: argument (Scalar :: new (& mut target . my_field) , "my_field") . help ("abc 123")) ;
"#
        );
    }

    #[test]
    fn render_collection_option() {
        // Setup
        let parameter = DeriveParameter {
            field_name: ident("my_field"),
            parameter_type: ParameterType::CollectionOption {
                nargs: DeriveValue {
                    tokens: quote! { Nargs::AtLeastOne },
                },
                short: None,
            },
            help: None,
        };

        // Execute
        let token_stream = parameter.generate(&ident("target"));

        // Verify
        assert_eq!(
            simple_format(token_stream.to_string()),
            r#"let my_field_default = format ! ("{
:?}
" , target . my_field) ;
 clp = clp . add (Parameter :: option (Collection :: new (& mut target . my_field , Nargs :: AtLeastOne) , "my_field" , None) . help (format ! ("(default {
}
)" , my_field_default))) ;
"#
        );
    }

    #[test]
    fn render_collection_option_help() {
        // Setup
        let parameter = DeriveParameter {
            field_name: ident("my_field"),
            parameter_type: ParameterType::CollectionOption {
                nargs: DeriveValue {
                    tokens: quote! { Nargs::AtLeastOne },
                },
                short: None,
            },
            help: Some(DeriveValue {
                tokens: Literal::string("abc 123").to_token_stream(),
            }),
        };

        // Execute
        let token_stream = parameter.generate(&ident("target"));

        // Verify
        assert_eq!(
            simple_format(token_stream.to_string()),
            r#"let my_field_default = format ! ("{
:?}
" , target . my_field) ;
 clp = clp . add (Parameter :: option (Collection :: new (& mut target . my_field , Nargs :: AtLeastOne) , "my_field" , None) . help (format ! ("{
}
 (default {
}
)" , "abc 123" , my_field_default))) ;
"#
        );
    }

    #[test]
    fn render_collection_option_short() {
        // Setup
        let parameter = DeriveParameter {
            field_name: ident("my_field"),
            parameter_type: ParameterType::CollectionOption {
                nargs: DeriveValue {
                    tokens: quote! { Nargs::AtLeastOne },
                },
                short: Some(DeriveValue {
                    tokens: Literal::character('m').into_token_stream(),
                }),
            },
            help: None,
        };

        // Execute
        let token_stream = parameter.generate(&ident("target"));

        // Verify
        assert_eq!(
            simple_format(token_stream.to_string()),
            r#"let my_field_default = format ! ("{
:?}
" , target . my_field) ;
 clp = clp . add (Parameter :: option (Collection :: new (& mut target . my_field , Nargs :: AtLeastOne) , "my_field" , Some ('m')) . help (format ! ("(default {
}
)" , my_field_default))) ;
"#
        );
    }

    #[test]
    fn render_optional_option() {
        // Setup
        let parameter = DeriveParameter {
            field_name: ident("my_field"),
            parameter_type: ParameterType::OptionalOption { short: None },
            help: None,
        };

        // Execute
        let token_stream = parameter.generate(&ident("target"));

        // Verify
        assert_eq!(
            simple_format(token_stream.to_string()),
            r#"if let Some (inner) = target . my_field . as_ref () {
 let my_field_default = format ! ("{
inner}
") ;
 clp = clp . add (Parameter :: option (Optional :: new (& mut target . my_field) , "my_field" , None) . help (format ! ("(default {
}
)" , my_field_default))) ;
 }
 else {
 clp = clp . add (Parameter :: option (Optional :: new (& mut target . my_field) , "my_field" , None)) ;
 }
"#
        );
    }

    #[test]
    fn render_optional_option_help() {
        // Setup
        let parameter = DeriveParameter {
            field_name: ident("my_field"),
            parameter_type: ParameterType::OptionalOption { short: None },
            help: Some(DeriveValue {
                tokens: Literal::string("abc 123").to_token_stream(),
            }),
        };

        // Execute
        let token_stream = parameter.generate(&ident("target"));

        // Verify
        assert_eq!(
            simple_format(token_stream.to_string()),
            r#"if let Some (inner) = target . my_field . as_ref () {
 let my_field_default = format ! ("{
inner}
") ;
 clp = clp . add (Parameter :: option (Optional :: new (& mut target . my_field) , "my_field" , None) . help (format ! ("{
}
 (default {
}
)" , "abc 123" , my_field_default))) ;
 }
 else {
 clp = clp . add (Parameter :: option (Optional :: new (& mut target . my_field) , "my_field" , None) . help ("abc 123")) ;
 }
"#
        );
    }

    #[test]
    fn render_optional_option_short() {
        // Setup
        let parameter = DeriveParameter {
            field_name: ident("my_field"),
            parameter_type: ParameterType::OptionalOption {
                short: Some(DeriveValue {
                    tokens: Literal::character('m').into_token_stream(),
                }),
            },
            help: None,
        };

        // Execute
        let token_stream = parameter.generate(&ident("target"));

        // Verify
        assert_eq!(
            simple_format(token_stream.to_string()),
            r#"if let Some (inner) = target . my_field . as_ref () {
 let my_field_default = format ! ("{
inner}
") ;
 clp = clp . add (Parameter :: option (Optional :: new (& mut target . my_field) , "my_field" , Some ('m')) . help (format ! ("(default {
}
)" , my_field_default))) ;
 }
 else {
 clp = clp . add (Parameter :: option (Optional :: new (& mut target . my_field) , "my_field" , Some ('m'))) ;
 }
"#
        );
    }

    #[test]
    fn render_scalar_option() {
        // Setup
        let parameter = DeriveParameter {
            field_name: ident("my_field"),
            parameter_type: ParameterType::ScalarOption { short: None },
            help: None,
        };

        // Execute
        let token_stream = parameter.generate(&ident("target"));

        // Verify
        assert_eq!(
            simple_format(token_stream.to_string()),
            r#"let my_field_default = target . my_field . to_string () ;
 clp = clp . add (Parameter :: option (Scalar :: new (& mut target . my_field) , "my_field" , None) . help (format ! ("(default {
}
)" , my_field_default))) ;
"#
        );
    }

    #[test]
    fn render_scalar_option_help() {
        // Setup
        let parameter = DeriveParameter {
            field_name: ident("my_field"),
            parameter_type: ParameterType::ScalarOption { short: None },
            help: Some(DeriveValue {
                tokens: Literal::string("abc 123").to_token_stream(),
            }),
        };

        // Execute
        let token_stream = parameter.generate(&ident("target"));

        // Verify
        assert_eq!(
            simple_format(token_stream.to_string()),
            r#"let my_field_default = target . my_field . to_string () ;
 clp = clp . add (Parameter :: option (Scalar :: new (& mut target . my_field) , "my_field" , None) . help (format ! ("{
}
 (default {
}
)" , "abc 123" , my_field_default))) ;
"#
        );
    }

    #[test]
    fn render_scalar_option_short() {
        // Setup
        let parameter = DeriveParameter {
            field_name: ident("my_field"),
            parameter_type: ParameterType::ScalarOption {
                short: Some(DeriveValue {
                    tokens: Literal::character('m').into_token_stream(),
                }),
            },
            help: None,
        };

        // Execute
        let token_stream = parameter.generate(&ident("target"));

        // Verify
        assert_eq!(
            simple_format(token_stream.to_string()),
            r#"let my_field_default = target . my_field . to_string () ;
 clp = clp . add (Parameter :: option (Scalar :: new (& mut target . my_field) , "my_field" , Some ('m')) . help (format ! ("(default {
}
)" , my_field_default))) ;
"#
        );
    }

    #[test]
    fn render_switch() {
        // Setup
        let parameter = DeriveParameter {
            field_name: ident("my_field"),
            parameter_type: ParameterType::Switch { short: None },
            help: None,
        };

        // Execute
        let token_stream = parameter.generate(&ident("target"));

        // Verify
        assert_eq!(
            token_stream.to_string(),
            "let my_field_target = target . my_field . clone () ; clp = clp . add (Parameter :: option (Switch :: new (& mut target . my_field , ! my_field_target) , \"my_field\" , None)) ;"
        );
    }

    #[test]
    fn render_switch_help() {
        // Setup
        let parameter = DeriveParameter {
            field_name: ident("my_field"),
            parameter_type: ParameterType::Switch { short: None },
            help: Some(DeriveValue {
                tokens: Literal::string("abc 123").to_token_stream(),
            }),
        };

        // Execute
        let token_stream = parameter.generate(&ident("target"));

        // Verify
        assert_eq!(
            simple_format(token_stream.to_string()),
            r#"let my_field_target = target . my_field . clone () ;
 clp = clp . add (Parameter :: option (Switch :: new (& mut target . my_field , ! my_field_target) , "my_field" , None) . help ("abc 123")) ;
"#
        );
    }

    #[test]
    fn render_switch_short() {
        // Setup
        let parameter = DeriveParameter {
            field_name: ident("my_field"),
            parameter_type: ParameterType::Switch {
                short: Some(DeriveValue {
                    tokens: Literal::character('m').into_token_stream(),
                }),
            },
            help: None,
        };

        // Execute
        let token_stream = parameter.generate(&ident("target"));

        // Verify
        assert_eq!(
            token_stream.to_string(),
            "let my_field_target = target . my_field . clone () ; clp = clp . add (Parameter :: option (Switch :: new (& mut target . my_field , ! my_field_target) , \"my_field\" , Some ('m'))) ;"
        );
    }

    #[test]
    fn render_condition() {
        // Setup
        let parameter = DeriveParameter {
            field_name: ident("my_field"),
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
            help: None,
        };

        // Execute
        let token_stream = parameter.generate(&ident("target"));

        // Verify
        assert_eq!(
            simple_format(token_stream.to_string()),
            r#"let mut clp = clp . branch (Condition :: new (Scalar :: new (& mut target . my_field) , "my_field")) ;
 clp = clp . command (0 , Abc :: setup_command (& mut Abc_target)) ;
 clp = clp . command (1 , Def :: setup_command (& mut Def_target)) ;
"#
        );
    }

    #[test]
    fn render_condition_help() {
        // Setup
        let parameter = DeriveParameter {
            field_name: ident("my_field"),
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
            help: Some(DeriveValue {
                tokens: Literal::string("abc 123").to_token_stream(),
            }),
        };

        // Execute
        let token_stream = parameter.generate(&ident("target"));

        // Verify
        assert_eq!(
            simple_format(token_stream.to_string()),
            r#"let mut clp = clp . branch (Condition :: new (Scalar :: new (& mut target . my_field) , "my_field") . help ("abc 123")) ;
 clp = clp . command (0 , Abc :: setup_command (& mut Abc_target)) ;
 clp = clp . command (1 , Def :: setup_command (& mut Def_target)) ;
"#
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
