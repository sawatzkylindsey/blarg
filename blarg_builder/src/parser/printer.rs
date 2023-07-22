use std::collections::HashMap;

use crate::constant::*;
use crate::model::Nargs;
use crate::parser::interface::UserInterface;

pub(crate) struct OptionParameter {
    name: String,
    short: Option<char>,
    nargs: Nargs,
    description: Option<String>,
    choices: HashMap<String, String>,
}

impl OptionParameter {
    #[cfg(test)]
    fn basic(name: String, short: Option<char>, nargs: Nargs, description: Option<String>) -> Self {
        Self {
            name,
            short,
            nargs,
            description,
            choices: HashMap::default(),
        }
    }

    pub(crate) fn new(
        name: String,
        short: Option<char>,
        nargs: Nargs,
        description: Option<String>,
        choices: HashMap<String, String>,
    ) -> Self {
        Self {
            name,
            short,
            nargs,
            description,
            choices,
        }
    }
}

pub(crate) struct ArgumentParameter {
    name: String,
    nargs: Nargs,
    description: Option<String>,
    choices: HashMap<String, String>,
}

impl ArgumentParameter {
    #[cfg(test)]
    fn basic(name: String, nargs: Nargs, description: Option<String>) -> Self {
        Self {
            name,
            nargs,
            description,
            choices: HashMap::default(),
        }
    }

    pub(crate) fn new(
        name: String,
        nargs: Nargs,
        description: Option<String>,
        choices: HashMap<String, String>,
    ) -> Self {
        Self {
            name,
            nargs,
            description,
            choices,
        }
    }
}

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
        options.sort_by(|a, b| a.name.cmp(&b.name));
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

        for OptionParameter {
            name,
            short,
            nargs,
            choices,
            ..
        } in &self.options
        {
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
                    // The 6 accounts for "-S , --".
                    // Ex: "-f FLAG, --flag FLAG"
                    //      ^^     ^^^^
                    if column_width < name.len() + (grammar.len() * 2) + 6 {
                        column_width = name.len() + (grammar.len() * 2) + 6;
                    }

                    summary.push(format!("[-{s}{grammar}]"));
                }
                None => {
                    // The 2 accounts for "--".
                    // Ex: "--flag FLAG"
                    //      ^^
                    if column_width < name.len() + grammar.len() + 2 {
                        column_width = name.len() + grammar.len() + 2;
                    }

                    summary.push(format!("[--{name}{grammar}]"));
                }
            };

            for (choice, _) in choices.iter() {
                // The 2 accounts for the choice indent.
                if column_width < choice.len() + 2 {
                    column_width = choice.len() + 2;
                }
            }
        }

        for ArgumentParameter {
            name,
            nargs,
            choices,
            ..
        } in &self.arguments
        {
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

            for (choice, _) in choices.iter() {
                // The 2 accounts for the choice indent.
                if column_width < choice.len() + 2 {
                    column_width = choice.len() + 2;
                }
            }
        }

        user_interface.print(format!(
            "usage: {p} {s}",
            p = program.into(),
            s = summary.join(" ")
        ));

        if !self.arguments.is_empty() {
            user_interface.print("".to_string());
            user_interface.print("positional arguments:".to_string());

            for ArgumentParameter {
                name,
                description,
                choices,
                ..
            } in &self.arguments
            {
                let grammar = grammars
                    .remove(name)
                    .expect("internal error - must have been set");
                let argument_description = match description {
                    Some(message) => format!("  {message}"),
                    None => "".to_string(),
                };
                let (argument_choices, choices_ordered) = if choices.is_empty() {
                    ("".to_string(), None)
                } else {
                    let mut choices_ordered: Vec<String> = choices.keys().cloned().collect();
                    choices_ordered.sort();
                    (
                        format!("  {{{}}}", choices_ordered.join(", ")),
                        Some(choices_ordered),
                    )
                };
                user_interface.print(format!(
                    " {:column_width$}{argument_choices}{argument_description}",
                    grammar
                ));

                if let Some(choice_keys) = choices_ordered {
                    for choice in choice_keys {
                        let description = choices
                            .get(&choice)
                            .expect("internal error - choice must exist");
                        user_interface.print(format!("   {:column_width$}  {description}", choice));
                    }
                }
            }
        }

        user_interface.print("".to_string());
        user_interface.print("options:".to_string());
        user_interface.print(format!(
            " {:column_width$}  Show this help message and exit.",
            help_flags
        ));

        for OptionParameter {
            name,
            short,
            description,
            choices,
            ..
        } in &self.options
        {
            let grammar = grammars
                .remove(name)
                .expect("internal error - must have been set");
            let option_flags = match short {
                Some(s) => format!("-{s}{grammar}, --{name}{grammar}"),
                None => format!("--{name}{grammar}"),
            };
            let option_description = match description {
                Some(message) => format!("  {message}"),
                None => "".to_string(),
            };
            let (option_choices, choices_ordered) = if choices.is_empty() {
                ("".to_string(), None)
            } else {
                let mut choices_ordered: Vec<String> = choices.keys().cloned().collect();
                choices_ordered.sort();
                (
                    format!("  {{{}}}", choices_ordered.join(", ")),
                    Some(choices_ordered),
                )
            };
            user_interface.print(format!(
                " {:column_width$}{option_choices}{option_description}",
                option_flags
            ));

            if let Some(choice_keys) = choices_ordered {
                for choice in choice_keys {
                    let description = choices
                        .get(&choice)
                        .expect("internal error - choice must exist");
                    user_interface.print(format!("   {:column_width$}  {description}", choice));
                }
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ErrorContext {
    offset: usize,
    tokens: Vec<String>,
}

impl ErrorContext {
    pub(crate) fn new(offset: usize, tokens: &[&str]) -> Self {
        Self {
            offset,
            tokens: tokens.iter().map(|s| s.to_string()).collect(),
        }
    }
}

impl std::fmt::Display for ErrorContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut tokens_length = 0;
        let mut projection = String::default();
        let mut projection_offset = 0;

        for (i, token) in self.tokens.iter().enumerate() {
            tokens_length += token.len();
            projection.push_str(token);

            if i + 1 < self.tokens.len() {
                projection.push_str(" ");

                if tokens_length <= self.offset {
                    projection_offset += 1;
                }
            }
        }

        write!(
            f,
            "{projection}\n{:width$}^",
            "",
            width = std::cmp::min(self.offset, tokens_length.saturating_sub(1)) + projection_offset
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::util::InMemoryInterface;

    #[test]
    fn print_help_empty() {
        // Setup
        let printer = Printer::empty();
        let interface = InMemoryInterface::default();

        // Execute
        printer.print_help("program", &interface);

        // Verify
        let message = interface.consume_message();
        assert_eq!(
            message,
            r#"usage: program [-h]

options:
 -h, --help  Show this help message and exit."#
        );
    }

    #[test]
    fn print_help_option() {
        // Setup
        let printer = Printer::new(
            vec![OptionParameter::basic(
                "flag".to_string(),
                Some('f'),
                Nargs::Precisely(1),
                Some("message".to_string()),
            )],
            Vec::default(),
        );
        let interface = InMemoryInterface::default();

        // Execute
        printer.print_help("program", &interface);

        // Verify
        let message = interface.consume_message();
        assert_eq!(
            message,
            r#"usage: program [-h] [-f FLAG]

options:
 -h, --help            Show this help message and exit.
 -f FLAG, --flag FLAG  message"#
        );
    }

    #[test]
    fn print_help_option_choices() {
        // Setup
        let printer = Printer::new(
            vec![OptionParameter::new(
                "flag".to_string(),
                Some('f'),
                Nargs::Precisely(1),
                None,
                HashMap::from([
                    ("xyz".to_string(), "do the xyz".to_string()),
                    ("abc".to_string(), "do the abc".to_string()),
                    ("123".to_string(), "do the 123".to_string()),
                ]),
            )],
            Vec::default(),
        );
        let interface = InMemoryInterface::default();

        // Execute
        printer.print_help("program", &interface);

        // Verify
        let message = interface.consume_message();
        assert_eq!(
            message,
            r#"usage: program [-h] [-f FLAG]

options:
 -h, --help            Show this help message and exit.
 -f FLAG, --flag FLAG  {123, abc, xyz}
   123                   do the 123
   abc                   do the abc
   xyz                   do the xyz"#
        );
    }

    #[test]
    fn print_help_option_precisely0() {
        // Setup
        let printer = Printer::new(
            vec![OptionParameter::basic(
                "flag".to_string(),
                None,
                Nargs::Precisely(0),
                None,
            )],
            Vec::default(),
        );
        let interface = InMemoryInterface::default();

        // Execute
        printer.print_help("program", &interface);

        // Verify
        let message = interface.consume_message();
        assert_eq!(
            message,
            r#"usage: program [-h] [--flag]

options:
 -h, --help  Show this help message and exit.
 --flag    "#
        );
    }

    #[test]
    fn print_help_option_precisely2() {
        // Setup
        let printer = Printer::new(
            vec![OptionParameter::basic(
                "flag".to_string(),
                None,
                Nargs::Precisely(2),
                None,
            )],
            Vec::default(),
        );
        let interface = InMemoryInterface::default();

        // Execute
        printer.print_help("program", &interface);

        // Verify
        let message = interface.consume_message();
        assert_eq!(
            message,
            r#"usage: program [-h] [--flag FLAG FLAG]

options:
 -h, --help        Show this help message and exit.
 --flag FLAG FLAG"#
        );
    }

    #[test]
    fn print_help_option_atleastone() {
        // Setup
        let printer = Printer::new(
            vec![OptionParameter::basic(
                "flag".to_string(),
                None,
                Nargs::AtLeastOne,
                None,
            )],
            Vec::default(),
        );
        let interface = InMemoryInterface::default();

        // Execute
        printer.print_help("program", &interface);

        // Verify
        let message = interface.consume_message();
        assert_eq!(
            message,
            r#"usage: program [-h] [--flag FLAG [...]]

options:
 -h, --help         Show this help message and exit.
 --flag FLAG [...]"#
        );
    }

    #[test]
    fn print_help_option_any() {
        // Setup
        let printer = Printer::new(
            vec![OptionParameter::basic(
                "flag".to_string(),
                None,
                Nargs::Any,
                None,
            )],
            Vec::default(),
        );
        let interface = InMemoryInterface::default();

        // Execute
        printer.print_help("program", &interface);

        // Verify
        let message = interface.consume_message();
        assert_eq!(
            message,
            r#"usage: program [-h] [--flag [FLAG ...]]

options:
 -h, --help         Show this help message and exit.
 --flag [FLAG ...]"#
        );
    }

    #[test]
    fn print_help_argument() {
        // Setup
        let printer = Printer::new(
            Vec::default(),
            vec![ArgumentParameter::basic(
                "name".to_string(),
                Nargs::Precisely(1),
                Some("message".to_string()),
            )],
        );
        let interface = InMemoryInterface::default();

        // Execute
        printer.print_help("program", &interface);

        // Verify
        let message = interface.consume_message();
        assert_eq!(
            message,
            r#"usage: program [-h] NAME

positional arguments:
 NAME        message

options:
 -h, --help  Show this help message and exit."#
        );
    }

    #[test]
    fn print_help_argument_choices() {
        // Setup
        let printer = Printer::new(
            Vec::default(),
            vec![ArgumentParameter::new(
                "name".to_string(),
                Nargs::Precisely(1),
                None,
                HashMap::from([
                    ("xyz".to_string(), "do the xyz".to_string()),
                    ("abc".to_string(), "do the abc".to_string()),
                    ("123".to_string(), "do the 123".to_string()),
                ]),
            )],
        );
        let interface = InMemoryInterface::default();

        // Execute
        printer.print_help("program", &interface);

        // Verify
        let message = interface.consume_message();
        assert_eq!(
            message,
            r#"usage: program [-h] NAME

positional arguments:
 NAME        {123, abc, xyz}
   123         do the 123
   abc         do the abc
   xyz         do the xyz

options:
 -h, --help  Show this help message and exit."#
        );
    }

    #[test]
    fn print_help_argument_precisely2() {
        // Setup
        let printer = Printer::new(
            Vec::default(),
            vec![ArgumentParameter::basic(
                "name".to_string(),
                Nargs::Precisely(2),
                None,
            )],
        );
        let interface = InMemoryInterface::default();

        // Execute
        printer.print_help("program", &interface);

        // Verify
        let message = interface.consume_message();
        assert_eq!(
            message,
            r#"usage: program [-h] NAME NAME

positional arguments:
 NAME NAME 

options:
 -h, --help  Show this help message and exit."#
        );
    }

    #[test]
    fn print_help_argument_atleastone() {
        // Setup
        let printer = Printer::new(
            Vec::default(),
            vec![ArgumentParameter::basic(
                "name".to_string(),
                Nargs::AtLeastOne,
                None,
            )],
        );
        let interface = InMemoryInterface::default();

        // Execute
        printer.print_help("program", &interface);

        // Verify
        let message = interface.consume_message();
        assert_eq!(
            message,
            r#"usage: program [-h] NAME [...]

positional arguments:
 NAME [...]

options:
 -h, --help  Show this help message and exit."#
        );
    }

    #[test]
    fn print_help_argument_any() {
        // Setup
        let printer = Printer::new(
            Vec::default(),
            vec![ArgumentParameter::basic(
                "name".to_string(),
                Nargs::Any,
                None,
            )],
        );
        let interface = InMemoryInterface::default();

        // Execute
        printer.print_help("program", &interface);

        // Verify
        let message = interface.consume_message();
        assert_eq!(
            message,
            r#"usage: program [-h] [NAME ...]

positional arguments:
 [NAME ...]

options:
 -h, --help  Show this help message and exit."#
        );
    }

    #[test]
    fn print_help() {
        // Setup
        let printer = Printer::new(
            vec![
                OptionParameter::basic(
                    "car".to_string(),
                    Some('x'),
                    Nargs::Any,
                    Some("car message".to_string()),
                ),
                OptionParameter::basic(
                    "blue".to_string(),
                    Some('y'),
                    Nargs::Precisely(0),
                    Some("blue message".to_string()),
                ),
                OptionParameter::basic(
                    "apple".to_string(),
                    Some('z'),
                    Nargs::Precisely(1),
                    Some("apple message".to_string()),
                ),
            ],
            vec![
                ArgumentParameter::basic(
                    "name".to_string(),
                    Nargs::Precisely(1),
                    Some("name message".to_string()),
                ),
                ArgumentParameter::basic(
                    "items".to_string(),
                    Nargs::Any,
                    Some("items message".to_string()),
                ),
            ],
        );
        let interface = InMemoryInterface::default();

        // Execute
        printer.print_help("program", &interface);

        // Verify
        let message = interface.consume_message();
        assert_eq!(
            message,
            r#"usage: program [-h] [-z APPLE] [-y] [-x [CAR ...]] NAME [ITEMS ...]

positional arguments:
 NAME                           name message
 [ITEMS ...]                    items message

options:
 -h, --help                     Show this help message and exit.
 -z APPLE, --apple APPLE        apple message
 -y, --blue                     blue message
 -x [CAR ...], --car [CAR ...]  car message"#
        );
    }

    #[test]
    fn print_help_choices_from_option() {
        // Setup
        let printer = Printer::new(
            vec![
                OptionParameter::basic(
                    "blue".to_string(),
                    Some('y'),
                    Nargs::Precisely(0),
                    Some("blue message".to_string()),
                ),
                OptionParameter::new(
                    "apple".to_string(),
                    Some('z'),
                    Nargs::Precisely(1),
                    Some("extra".to_string()),
                    HashMap::from([(
                        "abcdefghijklmnopqrstuvwxyz".to_string(),
                        "abcdefghijklmnopqrstuvwxyz".to_string(),
                    )]),
                ),
            ],
            vec![
                ArgumentParameter::basic(
                    "name".to_string(),
                    Nargs::Precisely(1),
                    Some("name message".to_string()),
                ),
                ArgumentParameter::basic(
                    "items".to_string(),
                    Nargs::Any,
                    Some("items message".to_string()),
                ),
            ],
        );
        let interface = InMemoryInterface::default();

        // Execute
        printer.print_help("program", &interface);

        // Verify
        let message = interface.consume_message();
        assert_eq!(
            message,
            r#"usage: program [-h] [-z APPLE] [-y] NAME [ITEMS ...]

positional arguments:
 NAME                          name message
 [ITEMS ...]                   items message

options:
 -h, --help                    Show this help message and exit.
 -z APPLE, --apple APPLE       {abcdefghijklmnopqrstuvwxyz}  extra
   abcdefghijklmnopqrstuvwxyz    abcdefghijklmnopqrstuvwxyz
 -y, --blue                    blue message"#
        );
    }

    #[test]
    fn print_help_choices_from_argument() {
        // Setup
        let printer = Printer::new(
            vec![OptionParameter::basic(
                "blue".to_string(),
                Some('y'),
                Nargs::Precisely(0),
                Some("blue message".to_string()),
            )],
            vec![
                ArgumentParameter::new(
                    "name".to_string(),
                    Nargs::Precisely(1),
                    Some("extra".to_string()),
                    HashMap::from([(
                        "abcdefghijklmnopqrstuvwxyz".to_string(),
                        "abcdefghijklmnopqrstuvwxyz".to_string(),
                    )]),
                ),
                ArgumentParameter::basic(
                    "items".to_string(),
                    Nargs::Any,
                    Some("items message".to_string()),
                ),
            ],
        );
        let interface = InMemoryInterface::default();

        // Execute
        printer.print_help("program", &interface);

        // Verify
        let message = interface.consume_message();
        assert_eq!(
            message,
            r#"usage: program [-h] [-y] NAME [ITEMS ...]

positional arguments:
 NAME                          {abcdefghijklmnopqrstuvwxyz}  extra
   abcdefghijklmnopqrstuvwxyz    abcdefghijklmnopqrstuvwxyz
 [ITEMS ...]                   items message

options:
 -h, --help                    Show this help message and exit.
 -y, --blue                    blue message"#
        );
    }

    #[test]
    fn error_context_tokens0() {
        assert_eq!(
            ErrorContext::new(0, &[]).to_string(),
            r#"
^"#
        );
        assert_eq!(
            ErrorContext::new(1, &[]).to_string(),
            r#"
^"#
        );
        assert_eq!(
            ErrorContext::new(2, &[]).to_string(),
            r#"
^"#
        );
        assert_eq!(
            ErrorContext::new(3, &[]).to_string(),
            r#"
^"#
        );
    }

    #[test]
    fn error_context_tokens1() {
        assert_eq!(
            ErrorContext::new(0, &["abc"]).to_string(),
            r#"abc
^"#
        );
        assert_eq!(
            ErrorContext::new(1, &["abc"]).to_string(),
            r#"abc
 ^"#
        );
        assert_eq!(
            ErrorContext::new(2, &["abc"]).to_string(),
            r#"abc
  ^"#
        );
        assert_eq!(
            ErrorContext::new(3, &["abc"]).to_string(),
            r#"abc
  ^"#
        );
    }

    #[test]
    fn error_context_tokens2() {
        assert_eq!(
            ErrorContext::new(0, &["abc", "123"]).to_string(),
            r#"abc 123
^"#
        );
        assert_eq!(
            ErrorContext::new(1, &["abc", "123"]).to_string(),
            r#"abc 123
 ^"#
        );
        assert_eq!(
            ErrorContext::new(2, &["abc", "123"]).to_string(),
            r#"abc 123
  ^"#
        );
        assert_eq!(
            ErrorContext::new(3, &["abc", "123"]).to_string(),
            r#"abc 123
    ^"#
        );
        assert_eq!(
            ErrorContext::new(4, &["abc", "123"]).to_string(),
            r#"abc 123
     ^"#
        );
        assert_eq!(
            ErrorContext::new(5, &["abc", "123"]).to_string(),
            r#"abc 123
      ^"#
        );
        assert_eq!(
            ErrorContext::new(6, &["abc", "123"]).to_string(),
            r#"abc 123
      ^"#
        );
    }
}
