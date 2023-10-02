use crate::model::{Command, DeriveParameter, DeriveValue, ParameterType};
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};

impl DeriveParameter {
    pub(crate) fn generate(self, parent: &syn::Ident) -> TokenStream2 {
        let DeriveParameter {
            field_name,
            parameter_type,
            choices,
            help,
        } = self;
        let field_name_str = format!("{field_name}");

        let (before_lines, parameter, after_lines) = match &parameter_type {
            ParameterType::CollectionArgument { nargs } => {
                let nargs = &nargs.tokens;
                (
                    None,
                    quote! {
                        Parameter::argument(Collection::new(&mut #parent.#field_name, #nargs), #field_name_str)
                    },
                    None,
                )
            }
            ParameterType::ScalarArgument => (
                None,
                quote! {
                    Parameter::argument(Scalar::new(&mut #parent.#field_name), #field_name_str)
                },
                None,
            ),

            ParameterType::CollectionOption { nargs, short } => {
                let nargs = &nargs.tokens;
                let short = flatten(short.as_ref());
                (
                    None,
                    quote! {
                        Parameter::option(Collection::new(&mut #parent.#field_name, #nargs), #field_name_str, #short)
                    },
                    None,
                )
            }
            ParameterType::ScalarOption { short } => {
                let short = flatten(short.as_ref());
                (
                    None,
                    quote! {

                        Parameter::option(Scalar::new(&mut #parent.#field_name), #field_name_str, #short)
                    },
                    None,
                )
            }
            ParameterType::OptionalOption { short } => {
                let short = flatten(short.as_ref());
                (
                    None,
                    quote! {
                        Parameter::option(Optional::new(&mut #parent.#field_name), #field_name_str, #short)
                    },
                    None,
                )
            }

            ParameterType::Switch { short } => {
                let short = flatten(short.as_ref());
                let field_name_target = format_ident!("{field_name}_target");

                (
                    Some(quote! {
                        let #field_name_target = #parent.#field_name.clone();
                    }),
                    quote! {
                        Parameter::option(Switch::new(&mut #parent.#field_name, !#field_name_target), #field_name_str, #short)
                    },
                    None,
                )
            }
            ParameterType::Condition { commands } => {
                let commands: Vec<_> = commands
                    .into_iter()
                    .map(|command| {
                        let Command {
                            variant,
                            command_struct,
                        } = command;
                        let variant = &variant.tokens;
                        let command_struct = &command_struct.tokens;
                        let command_struct_target = format_ident!("{command_struct}_target");
                        quote! {
                            clp = clp.command(#variant, #command_struct::setup_command(&mut #command_struct_target));
                        }
                    })
                    .collect();
                (
                    None,
                    quote! {
                        Condition::new(Scalar::new(&mut #parent.#field_name), #field_name_str)
                    },
                    Some(quote! {
                        #( #commands )*
                    }),
                )
            }
        };

        let default = match &parameter_type {
            ParameterType::CollectionOption { .. } => {
                let field_default = format_ident!("{field_name}_default");
                Some(quote! { let #field_default = format!("{:?}", #parent.#field_name); })
            }
            ParameterType::ScalarOption { .. } => {
                let field_default = format_ident!("{field_name}_default");
                Some(quote! { let #field_default = #parent.#field_name.to_string(); })
            }
            _ => None,
        };

        match parameter_type {
            ParameterType::CollectionOption { .. } | ParameterType::ScalarOption { .. } => {
                let field_default = format_ident!("{field_name}_default");
                match (choices, help) {
                    (Some(choices), Some(help)) => {
                        let choices = choices.tokens;
                        let help = help.tokens;
                        quote! {
                            #before_lines
                            #default
                            clp = clp.add(#choices(#parameter.help(format!("{} (default {})", #help, #field_default))));
                            #after_lines
                        }
                    }
                    (Some(choices), None) => {
                        let choices = choices.tokens;
                        quote! {
                            #before_lines
                            #default
                            clp = clp.add(#choices(#parameter.help(format!("(default {})", #field_default))));
                            #after_lines
                        }
                    }
                    (None, Some(help)) => {
                        let help = help.tokens;
                        quote! {
                            #before_lines
                            #default
                            clp = clp.add(#parameter.help(format!("{} (default {})", #help, #field_default)));
                            #after_lines
                        }
                    }
                    (None, None) => {
                        quote! {
                            #before_lines
                            #default
                            clp = clp.add(#parameter.help(format!("(default {})", #field_default)));
                            #after_lines
                        }
                    }
                }
            }
            ParameterType::OptionalOption { .. } => {
                let field_default = format_ident!("{field_name}_default");

                match (choices, help) {
                    (Some(choices), Some(help)) => {
                        let choices = choices.tokens;
                        let help = help.tokens;
                        quote! {
                            #before_lines
                            if let Some(inner) = #parent.#field_name.as_ref() {
                                let #field_default = format!("{inner}");
                                clp = clp.add(#choices(#parameter
                                    .help(format!("{} (default {})", #help, #field_default))));
                            } else {
                                clp = clp.add(#choices(#parameter.help(#help)));
                            }
                            #after_lines
                        }
                    }
                    (Some(choices), None) => {
                        let choices = choices.tokens;
                        quote! {
                            #before_lines
                            if let Some(inner) = #parent.#field_name.as_ref() {
                                let #field_default = format!("{inner}");
                                clp = clp.add(#choices(#parameter
                                    .help(format!("(default {})", #field_default))));
                            } else {
                                clp = clp.add(#choices(#parameter));
                            }
                            #after_lines
                        }
                    }
                    (None, Some(help)) => {
                        let help = help.tokens;
                        quote! {
                            #before_lines
                            if let Some(inner) = #parent.#field_name.as_ref() {
                                let #field_default = format!("{inner}");
                                clp = clp.add(#parameter
                                    .help(format!("{} (default {})", #help, #field_default)));
                            } else {
                                clp = clp.add(#parameter.help(#help));
                            }
                            #after_lines
                        }
                    }
                    (None, None) => {
                        quote! {
                            #before_lines
                            if let Some(inner) = #parent.#field_name.as_ref() {
                                let #field_default = format!("{inner}");
                                clp = clp.add(#parameter
                                    .help(format!("(default {})", #field_default)));
                            } else {
                                clp = clp.add(#parameter);
                            }
                            #after_lines
                        }
                    }
                }
            }
            ParameterType::Condition { .. } => match (choices, help) {
                (Some(choices), Some(help)) => {
                    let choices = choices.tokens;
                    let help = help.tokens;
                    quote! {
                        #before_lines
                        let mut clp = clp.branch(#choices(#parameter.help(#help)));
                        #after_lines
                    }
                }
                (Some(choices), None) => {
                    let choices = choices.tokens;
                    quote! {
                        #before_lines
                        let mut clp = clp.branch(#choices(#parameter));
                        #after_lines
                    }
                }
                (None, Some(help)) => {
                    let help = help.tokens;
                    quote! {
                        #before_lines
                        let mut clp = clp.branch(#parameter.help(#help));
                        #after_lines
                    }
                }
                (None, None) => {
                    quote! {
                        #before_lines
                        let mut clp = clp.branch(#parameter);
                        #after_lines
                    }
                }
            },
            _ => match (choices, help) {
                (Some(choices), Some(help)) => {
                    let choices = choices.tokens;
                    let help = help.tokens;
                    quote! {
                        #before_lines
                        clp = clp.add(#choices(#parameter.help(#help)));
                        #after_lines
                    }
                }
                (Some(choices), None) => {
                    let choices = choices.tokens;
                    quote! {
                        #before_lines
                        clp = clp.add(#choices(#parameter));
                        #after_lines
                    }
                }
                (None, Some(help)) => {
                    let help = help.tokens;
                    quote! {
                        #before_lines
                        clp = clp.add(#parameter.help(#help));
                        #after_lines
                    }
                }
                (None, None) => {
                    quote! {
                        #before_lines
                        clp = clp.add(#parameter);
                        #after_lines
                    }
                }
            },
        }
    }
}

fn flatten(value: Option<&DeriveValue>) -> TokenStream2 {
    value.map_or_else(
        || quote! { None },
        |s| {
            let tokens = &s.tokens;
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
            choices: None,
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
    fn render_collection_argument_choices() {
        // Setup
        let parameter = DeriveParameter {
            field_name: ident("my_field"),
            parameter_type: ParameterType::CollectionArgument {
                nargs: DeriveValue {
                    tokens: quote! { Nargs::AtLeastOne },
                },
            },
            choices: Some(DeriveValue {
                tokens: quote! { my_func },
            }),
            help: None,
        };

        // Execute
        let token_stream = parameter.generate(&ident("target"));

        // Verify
        assert_eq!(
            simple_format(token_stream.to_string()),
            r#"clp = clp . add (my_func (Parameter :: argument (Collection :: new (& mut target . my_field , Nargs :: AtLeastOne) , "my_field"))) ;
"#
        );
    }

    #[test]
    fn render_collection_argument_choices_help() {
        // Setup
        let parameter = DeriveParameter {
            field_name: ident("my_field"),
            parameter_type: ParameterType::CollectionArgument {
                nargs: DeriveValue {
                    tokens: quote! { Nargs::AtLeastOne },
                },
            },
            choices: Some(DeriveValue {
                tokens: quote! { my_func },
            }),
            help: Some(DeriveValue {
                tokens: Literal::string("abc 123").to_token_stream(),
            }),
        };

        // Execute
        let token_stream = parameter.generate(&ident("target"));

        // Verify
        assert_eq!(
            simple_format(token_stream.to_string()),
            r#"clp = clp . add (my_func (Parameter :: argument (Collection :: new (& mut target . my_field , Nargs :: AtLeastOne) , "my_field") . help ("abc 123"))) ;
"#
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
            choices: None,
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
            choices: None,
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
    fn render_scalar_argument_choices() {
        // Setup
        let parameter = DeriveParameter {
            field_name: ident("my_field"),
            parameter_type: ParameterType::ScalarArgument,
            choices: Some(DeriveValue {
                tokens: quote! { my_func },
            }),
            help: None,
        };

        // Execute
        let token_stream = parameter.generate(&ident("target"));

        // Verify
        assert_eq!(
            simple_format(token_stream.to_string()),
            r#"clp = clp . add (my_func (Parameter :: argument (Scalar :: new (& mut target . my_field) , "my_field"))) ;
"#
        );
    }

    #[test]
    fn render_scalar_argument_choices_help() {
        // Setup
        let parameter = DeriveParameter {
            field_name: ident("my_field"),
            parameter_type: ParameterType::ScalarArgument,
            choices: Some(DeriveValue {
                tokens: quote! { my_func },
            }),
            help: Some(DeriveValue {
                tokens: Literal::string("abc 123").to_token_stream(),
            }),
        };

        // Execute
        let token_stream = parameter.generate(&ident("target"));

        // Verify
        assert_eq!(
            simple_format(token_stream.to_string()),
            r#"clp = clp . add (my_func (Parameter :: argument (Scalar :: new (& mut target . my_field) , "my_field") . help ("abc 123"))) ;
"#
        );
    }

    #[test]
    fn render_scalar_argument_help() {
        // Setup
        let parameter = DeriveParameter {
            field_name: ident("my_field"),
            parameter_type: ParameterType::ScalarArgument,
            choices: None,
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
            choices: None,
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
    fn render_collection_option_choices() {
        // Setup
        let parameter = DeriveParameter {
            field_name: ident("my_field"),
            parameter_type: ParameterType::CollectionOption {
                nargs: DeriveValue {
                    tokens: quote! { Nargs::AtLeastOne },
                },
                short: None,
            },
            choices: Some(DeriveValue {
                tokens: quote! { my_func },
            }),
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
 clp = clp . add (my_func (Parameter :: option (Collection :: new (& mut target . my_field , Nargs :: AtLeastOne) , "my_field" , None) . help (format ! ("(default {
}
)" , my_field_default)))) ;
"#
        );
    }

    #[test]
    fn render_collection_option_choices_help() {
        // Setup
        let parameter = DeriveParameter {
            field_name: ident("my_field"),
            parameter_type: ParameterType::CollectionOption {
                nargs: DeriveValue {
                    tokens: quote! { Nargs::AtLeastOne },
                },
                short: None,
            },
            choices: Some(DeriveValue {
                tokens: quote! { my_func },
            }),
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
 clp = clp . add (my_func (Parameter :: option (Collection :: new (& mut target . my_field , Nargs :: AtLeastOne) , "my_field" , None) . help (format ! ("{
}
 (default {
}
)" , "abc 123" , my_field_default)))) ;
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
            choices: None,
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
            choices: None,
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
            choices: None,
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
    fn render_optional_option_choices() {
        // Setup
        let parameter = DeriveParameter {
            field_name: ident("my_field"),
            parameter_type: ParameterType::OptionalOption { short: None },
            choices: Some(DeriveValue {
                tokens: quote! { my_func },
            }),
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
 clp = clp . add (my_func (Parameter :: option (Optional :: new (& mut target . my_field) , "my_field" , None) . help (format ! ("(default {
}
)" , my_field_default)))) ;
 }
 else {
 clp = clp . add (my_func (Parameter :: option (Optional :: new (& mut target . my_field) , "my_field" , None))) ;
 }
"#
        );
    }
    #[test]
    fn render_optional_option_choices_help() {
        // Setup
        let parameter = DeriveParameter {
            field_name: ident("my_field"),
            parameter_type: ParameterType::OptionalOption { short: None },
            choices: Some(DeriveValue {
                tokens: quote! { my_func },
            }),
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
 clp = clp . add (my_func (Parameter :: option (Optional :: new (& mut target . my_field) , "my_field" , None) . help (format ! ("{
}
 (default {
}
)" , "abc 123" , my_field_default)))) ;
 }
 else {
 clp = clp . add (my_func (Parameter :: option (Optional :: new (& mut target . my_field) , "my_field" , None) . help ("abc 123"))) ;
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
            choices: None,
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
            choices: None,
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
            choices: None,
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
    fn render_scalar_option_choices() {
        // Setup
        let parameter = DeriveParameter {
            field_name: ident("my_field"),
            parameter_type: ParameterType::ScalarOption { short: None },
            choices: Some(DeriveValue {
                tokens: quote! { my_func },
            }),
            help: None,
        };

        // Execute
        let token_stream = parameter.generate(&ident("target"));

        // Verify
        assert_eq!(
            simple_format(token_stream.to_string()),
            r#"let my_field_default = target . my_field . to_string () ;
 clp = clp . add (my_func (Parameter :: option (Scalar :: new (& mut target . my_field) , "my_field" , None) . help (format ! ("(default {
}
)" , my_field_default)))) ;
"#
        );
    }

    #[test]
    fn render_scalar_option_choices_help() {
        // Setup
        let parameter = DeriveParameter {
            field_name: ident("my_field"),
            parameter_type: ParameterType::ScalarOption { short: None },
            choices: Some(DeriveValue {
                tokens: quote! { my_func },
            }),
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
 clp = clp . add (my_func (Parameter :: option (Scalar :: new (& mut target . my_field) , "my_field" , None) . help (format ! ("{
}
 (default {
}
)" , "abc 123" , my_field_default)))) ;
"#
        );
    }

    #[test]
    fn render_scalar_option_help() {
        // Setup
        let parameter = DeriveParameter {
            field_name: ident("my_field"),
            parameter_type: ParameterType::ScalarOption { short: None },
            choices: None,
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
            choices: None,
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
            choices: None,
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
    fn render_switch_choices() {
        // Setup
        let parameter = DeriveParameter {
            field_name: ident("my_field"),
            parameter_type: ParameterType::Switch { short: None },
            choices: Some(DeriveValue {
                tokens: quote! { my_func },
            }),
            help: None,
        };

        // Execute
        let token_stream = parameter.generate(&ident("target"));

        // Verify
        assert_eq!(
            simple_format(token_stream.to_string()),
            r#"let my_field_target = target . my_field . clone () ;
 clp = clp . add (my_func (Parameter :: option (Switch :: new (& mut target . my_field , ! my_field_target) , "my_field" , None))) ;
"#
        );
    }

    #[test]
    fn render_switch_choices_help() {
        // Setup
        let parameter = DeriveParameter {
            field_name: ident("my_field"),
            parameter_type: ParameterType::Switch { short: None },
            choices: Some(DeriveValue {
                tokens: quote! { my_func },
            }),
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
 clp = clp . add (my_func (Parameter :: option (Switch :: new (& mut target . my_field , ! my_field_target) , "my_field" , None) . help ("abc 123"))) ;
"#
        );
    }

    #[test]
    fn render_switch_help() {
        // Setup
        let parameter = DeriveParameter {
            field_name: ident("my_field"),
            parameter_type: ParameterType::Switch { short: None },
            choices: None,
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
            choices: None,
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
            choices: None,
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
    fn render_condition_choices() {
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
            choices: Some(DeriveValue {
                tokens: quote! { my_func },
            }),
            help: None,
        };

        // Execute
        let token_stream = parameter.generate(&ident("target"));

        // Verify
        assert_eq!(
            simple_format(token_stream.to_string()),
            r#"let mut clp = clp . branch (my_func (Condition :: new (Scalar :: new (& mut target . my_field) , "my_field"))) ;
 clp = clp . command (0 , Abc :: setup_command (& mut Abc_target)) ;
 clp = clp . command (1 , Def :: setup_command (& mut Def_target)) ;
"#
        );
    }

    #[test]
    fn render_condition_choices_help() {
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
            choices: Some(DeriveValue {
                tokens: quote! { my_func },
            }),
            help: Some(DeriveValue {
                tokens: Literal::string("abc 123").to_token_stream(),
            }),
        };

        // Execute
        let token_stream = parameter.generate(&ident("target"));

        // Verify
        assert_eq!(
            simple_format(token_stream.to_string()),
            r#"let mut clp = clp . branch (my_func (Condition :: new (Scalar :: new (& mut target . my_field) , "my_field") . help ("abc 123"))) ;
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
            choices: None,
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
