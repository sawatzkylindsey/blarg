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
                    if column_width < name.len() + (grammar.len() * 2) + 6 {
                        column_width = name.len() + (grammar.len() * 2) + 6;
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
                Some(s) => format!("-{s}{grammar}, --{name}{grammar}"),
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
            vec![(
                "flag".to_string(),
                Some('f'),
                Nargs::Precisely(1),
                Some("message"),
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
    fn print_help_option_precisely0() {
        // Setup
        let printer = Printer::new(
            vec![("flag".to_string(), None, Nargs::Precisely(0), None)],
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
            vec![("flag".to_string(), None, Nargs::Precisely(2), None)],
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
            vec![("flag".to_string(), None, Nargs::AtLeastOne, None)],
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
            vec![("flag".to_string(), None, Nargs::Any, None)],
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
            vec![("name".to_string(), Nargs::Precisely(1), Some("message"))],
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
    fn print_help_argument_precisely2() {
        // Setup
        let printer = Printer::new(
            Vec::default(),
            vec![("name".to_string(), Nargs::Precisely(2), None)],
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
            vec![("name".to_string(), Nargs::AtLeastOne, None)],
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
        let printer = Printer::new(Vec::default(), vec![("name".to_string(), Nargs::Any, None)]);
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
                (
                    "car".to_string(),
                    Some('x'),
                    Nargs::Any,
                    Some("car message"),
                ),
                (
                    "blue".to_string(),
                    Some('y'),
                    Nargs::Precisely(0),
                    Some("blue message"),
                ),
                (
                    "apple".to_string(),
                    Some('z'),
                    Nargs::Precisely(1),
                    Some("apple message"),
                ),
            ],
            vec![
                (
                    "name".to_string(),
                    Nargs::Precisely(1),
                    Some("name message"),
                ),
                ("items".to_string(), Nargs::Any, Some("items message")),
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
