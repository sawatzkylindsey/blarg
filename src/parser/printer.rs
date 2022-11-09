use std::collections::HashMap;

use crate::constant::*;
use crate::model::Nargs;
use crate::parser::interface::UserInterface;

pub(crate) type OptionParameter = (String, Option<char>, Nargs, Option<&'static str>);
pub(crate) type ArgumentParameter = (String, Nargs, Option<&'static str>);

pub(crate) struct Printer {
    options: Vec<OptionParameter>,
    arguments: Vec<ArgumentParameter>,
}

impl Printer {
    #[cfg(test)]
    pub(crate) fn empty() -> Self {
        Self::new(Vec::default(), Vec::default())
    }

    pub(crate) fn new(
        mut options: Vec<OptionParameter>,
        arguments: Vec<ArgumentParameter>,
    ) -> Self {
        options.sort_by(|a, b| a.0.cmp(&b.0));
        Self { options, arguments }
    }

    pub(crate) fn print_help(
        &self,
        program: impl Into<String>,
        user_interface: &(impl UserInterface + ?Sized),
    ) {
        let help_flags = format!("-{HELP_SHORT}, --{HELP_NAME}");
        let mut summary = vec![format!("[-{HELP_SHORT}]")];
        let mut column_width = help_flags.len();
        let mut grammars: HashMap<String, String> = HashMap::default();

        for (name, short, nargs, _) in &self.options {
            let grammar = match nargs {
                Nargs::Precisely(0) => "".to_string(),
                Nargs::Precisely(n) => format!(
                    " {}",
                    (0..*n)
                        .map(|_| name.to_ascii_uppercase())
                        .collect::<Vec<String>>()
                        .join(" ")
                ),
                Nargs::Any => format!(" [{} ...]", name.to_ascii_uppercase()),
                Nargs::AtLeastOne => format!(" {} [...]", name.to_ascii_uppercase()),
            };
            grammars.insert(name.clone(), grammar.clone());
            match short {
                Some(s) => {
                    if column_width < name.len() + grammar.len() + 6 {
                        column_width = name.len() + grammar.len() + 6;
                    }

                    summary.push(format!("[-{s}{grammar}]"));
                }
                None => {
                    if column_width < name.len() + grammar.len() + 2 {
                        column_width = name.len() + grammar.len() + 2;
                    }

                    summary.push(format!("[--{name}{grammar}]"));
                }
            };
        }

        for (name, nargs, _) in &self.arguments {
            let grammar = match nargs {
                Nargs::Precisely(n) => format!(
                    "{}",
                    (0..*n)
                        .map(|_| name.to_ascii_uppercase())
                        .collect::<Vec<String>>()
                        .join(" ")
                ),
                Nargs::Any => format!("[{} ...]", name.to_ascii_uppercase()),
                Nargs::AtLeastOne => format!("{} [...]", name.to_ascii_uppercase()),
            };
            grammars.insert(name.clone(), grammar.clone());

            if column_width < grammar.len() {
                column_width = grammar.len();
            }

            summary.push(format!("{grammar}"));
        }

        user_interface.print(format!(
            "usage: {p} {s}",
            p = program.into(),
            s = summary.join(" ")
        ));

        if !self.arguments.is_empty() {
            user_interface.print("".to_string());
            user_interface.print("positional arguments:".to_string());

            for (name, _, description) in &self.arguments {
                let grammar = grammars
                    .remove(name)
                    .expect("internal error - must have been set");
                let argument_description = match description {
                    Some(message) => format!("  {message}"),
                    None => "".to_string(),
                };
                user_interface.print(format!(" {:column_width$}{argument_description}", grammar));
            }
        }

        user_interface.print("".to_string());
        user_interface.print("options:".to_string());
        user_interface.print(format!(
            " {:column_width$}  Show this help message and exit.",
            help_flags
        ));

        for (name, short, _, description) in &self.options {
            let grammar = grammars
                .remove(name)
                .expect("internal error - must have been set");
            let option_flags = match short {
                Some(s) => format!("-{s}, --{name}{grammar}"),
                None => format!("--{name}{grammar}"),
            };
            let option_description = match description {
                Some(message) => format!("  {message}"),
                None => "".to_string(),
            };
            user_interface.print(format!(
                " {:column_width$}{option_description}",
                option_flags
            ));
        }
    }
}
