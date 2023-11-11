use std::collections::HashMap;
use terminal_size::{terminal_size, Width};

use crate::constant::*;
use crate::model::Nargs;
use crate::parser::interface::UserInterface;
use crate::parser::{ColumnRenderer, LeftWidth, MiddleWidth, PaddingWidth, RightWidth, TotalWidth};

pub(crate) struct OptionParameter {
    name: String,
    short: Option<char>,
    nargs: Nargs,
    help: Option<String>,
    meta: Option<Vec<String>>,
    choices: HashMap<String, String>,
}

impl OptionParameter {
    #[cfg(test)]
    fn basic(
        name: String,
        short: Option<char>,
        nargs: Nargs,
        help: Option<String>,
        meta: Option<Vec<String>>,
    ) -> Self {
        Self {
            name,
            short,
            nargs,
            help,
            meta,
            choices: HashMap::default(),
        }
    }

    pub(crate) fn new(
        name: String,
        short: Option<char>,
        nargs: Nargs,
        help: Option<String>,
        meta: Option<Vec<String>>,
        choices: HashMap<String, String>,
    ) -> Self {
        Self {
            name,
            short,
            nargs,
            help,
            meta,
            choices,
        }
    }
}

pub(crate) struct ArgumentParameter {
    name: String,
    nargs: Nargs,
    help: Option<String>,
    meta: Option<Vec<String>>,
    choices: HashMap<String, String>,
}

impl ArgumentParameter {
    #[cfg(test)]
    fn basic(name: String, nargs: Nargs, help: Option<String>, meta: Option<Vec<String>>) -> Self {
        Self {
            name,
            nargs,
            help,
            meta,
            choices: HashMap::default(),
        }
    }

    pub(crate) fn new(
        name: String,
        nargs: Nargs,
        help: Option<String>,
        meta: Option<Vec<String>>,
        choices: HashMap<String, String>,
    ) -> Self {
        Self {
            name,
            nargs,
            help,
            meta,
            choices,
        }
    }
}

pub(crate) struct Printer {
    options: Vec<OptionParameter>,
    arguments: Vec<ArgumentParameter>,
    terminal_width: Option<usize>,
}

// Let's assume the average word length is 5.
// Then 17 is a good minimum, because it allows precisely 3 words with a space between them.
const DEFAULT_MIDDLE_WIDTH: usize = 17;
const PADDING_WIDTH: usize = 3;
const MAIN_INDENT: usize = 1;
const CHOICE_INDENT: usize = 2;

impl Printer {
    #[cfg(test)]
    pub(crate) fn empty() -> Self {
        Self::new(Vec::default(), Vec::default(), None)
    }

    pub(crate) fn terminal(
        options: Vec<OptionParameter>,
        arguments: Vec<ArgumentParameter>,
    ) -> Self {
        let terminal_width = if let Some((Width(terminal_width), _)) = terminal_size() {
            Some(terminal_width as usize)
        } else {
            None
        };

        Self::new(options, arguments, terminal_width)
    }

    pub(crate) fn new(
        mut options: Vec<OptionParameter>,
        arguments: Vec<ArgumentParameter>,
        terminal_width: Option<usize>,
    ) -> Self {
        options.sort_by(|a, b| a.name.cmp(&b.name));
        Self {
            options,
            arguments,
            terminal_width,
        }
    }

    pub(crate) fn print_help(
        &self,
        program: impl Into<String>,
        user_interface: &(impl UserInterface + ?Sized),
    ) {
        let help_flags = format!("-{HELP_SHORT}, --{HELP_NAME}");
        let mut summary = vec![format!("[-{HELP_SHORT}]")];
        let mut left_column_width = help_flags.len();
        let mut middle_column_width = HELP_MESSAGE.len() + MAIN_INDENT;
        let mut right_columns_widths = Vec::default();
        let mut grammars: HashMap<String, String> = HashMap::default();

        for OptionParameter {
            name,
            short,
            nargs,
            choices,
            help,
            meta,
        } in &self.options
        {
            let name_example = name.to_ascii_uppercase().replace("-", "_");
            let grammar = match nargs {
                Nargs::Precisely(0) => "".to_string(),
                Nargs::Precisely(n) => format!(
                    " {}",
                    (0..*n)
                        .map(|_| name_example.clone())
                        .collect::<Vec<String>>()
                        .join(" ")
                ),
                Nargs::Any => format!(" [{} ...]", name_example),
                Nargs::AtLeastOne => {
                    format!(" {} [...]", name_example)
                }
            };
            grammars.insert(name.clone(), grammar.clone());

            match short {
                Some(s) => {
                    // The 6 accounts for "-S , --".
                    // Ex: "-f FLAG, --flag FLAG"
                    //      ^^     ^^^^
                    if left_column_width < name.len() + (grammar.len() * 2) + 6 {
                        left_column_width = name.len() + (grammar.len() * 2) + 6;
                    }

                    summary.push(format!("[-{s}{grammar}]"));
                }
                None => {
                    // The 2 accounts for "--".
                    // Ex: "--flag FLAG"
                    //      ^^
                    if left_column_width < name.len() + grammar.len() + 2 {
                        left_column_width = name.len() + grammar.len() + 2;
                    }

                    summary.push(format!("[--{name}{grammar}]"));
                }
            };

            for (choice, description) in choices.iter() {
                if left_column_width < choice.len() + CHOICE_INDENT {
                    left_column_width = choice.len() + CHOICE_INDENT;
                }

                if middle_column_width < description.len() + MAIN_INDENT {
                    middle_column_width = description.len() + MAIN_INDENT;
                }
            }

            if let Some(help) = help {
                let choices_length = choices.keys().map(|c| c.len()).sum::<usize>();
                // `* 2` for the comma + space.
                // `+ 3` for the brackets + space
                let help_width =
                    help.len() + &choices_length + ((std::cmp::max(1, choices.len()) - 1) * 2) + 3;

                if middle_column_width < help_width + MAIN_INDENT {
                    middle_column_width = help_width + MAIN_INDENT;
                }
            }

            if let Some(meta) = meta {
                for (i, m) in meta.iter().enumerate() {
                    if i >= right_columns_widths.len() {
                        right_columns_widths
                            .push(RightWidth::new(std::cmp::max(1, m.len())).unwrap());
                    } else {
                        if right_columns_widths[*&i].value() < m.len() {
                            right_columns_widths[i] = RightWidth::new(m.len()).unwrap();
                        }
                    }
                }
            }
        }

        for ArgumentParameter {
            name,
            nargs,
            choices,
            help,
            meta,
        } in &self.arguments
        {
            let name_example = name.to_ascii_uppercase().replace("-", "_");
            let grammar = match nargs {
                Nargs::Precisely(n) => format!(
                    "{}",
                    (0..*n)
                        .map(|_| name_example.clone())
                        .collect::<Vec<String>>()
                        .join(" ")
                ),
                Nargs::Any => format!("[{} ...]", name_example),
                Nargs::AtLeastOne => {
                    format!("{} [...]", name_example)
                }
            };
            grammars.insert(name.clone(), grammar.clone());

            if left_column_width < grammar.len() {
                left_column_width = grammar.len();
            }

            summary.push(format!("{grammar}"));

            for (choice, description) in choices.iter() {
                if left_column_width < choice.len() + CHOICE_INDENT {
                    left_column_width = choice.len() + CHOICE_INDENT;
                }

                if middle_column_width < description.len() + MAIN_INDENT {
                    middle_column_width = description.len() + MAIN_INDENT;
                }
            }

            if let Some(help) = help {
                let choices_length = choices.keys().map(|c| c.len()).sum::<usize>();
                // `* 2` for the comma + space.
                // `+ 3` for the brackets + space
                let help_width =
                    help.len() + &choices_length + ((std::cmp::max(1, choices.len()) - 1) * 2) + 3;

                if middle_column_width < help_width + MAIN_INDENT {
                    middle_column_width = help_width + MAIN_INDENT;
                }
            }

            if let Some(meta) = meta {
                for (i, m) in meta.iter().enumerate() {
                    if i >= right_columns_widths.len() {
                        right_columns_widths
                            .push(RightWidth::new(std::cmp::max(1, m.len())).unwrap());
                    } else {
                        if right_columns_widths[*&i].value() < m.len() {
                            right_columns_widths[i] = RightWidth::new(m.len()).unwrap();
                        }
                    }
                }
            }
        }

        let column_renderer = match &self.terminal_width {
            Some(tw) => ColumnRenderer::guided(
                PaddingWidth::new(PADDING_WIDTH).unwrap(),
                LeftWidth::new(left_column_width.clone()).unwrap(),
                MiddleWidth::new(middle_column_width.clone()).unwrap(),
                right_columns_widths.clone(),
                TotalWidth(tw.clone()),
            ),
            None => None,
        };

        // column_renderer will be None if either:
        // * There isn't a self.terminal_width, or
        // * The terminal width isn't big enough for all the components.
        let column_renderer = column_renderer.unwrap_or(ColumnRenderer::new(
            PaddingWidth::new(PADDING_WIDTH).unwrap(),
            LeftWidth::new(left_column_width).unwrap(),
            MiddleWidth::new(std::cmp::min(middle_column_width, DEFAULT_MIDDLE_WIDTH)).unwrap(),
            right_columns_widths,
        ));

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
                help,
                choices,
                meta,
                ..
            } in &self.arguments
            {
                let grammar = grammars
                    .remove(name)
                    .expect("internal error - must have been set");
                let argument_help = match help {
                    Some(message) => format!("{message}"),
                    None => "".to_string(),
                };
                let (argument_choices, choices_ordered) = if choices.is_empty() {
                    ("".to_string(), None)
                } else {
                    let mut choices_ordered: Vec<String> = choices.keys().cloned().collect();
                    choices_ordered.sort();
                    (
                        format!("{{{}}} ", choices_ordered.join(", ")),
                        Some(choices_ordered),
                    )
                };
                for line in column_renderer.render(
                    MAIN_INDENT,
                    &grammar,
                    format!("{argument_choices}{argument_help}").as_str(),
                    meta.as_ref().unwrap_or(&Vec::default()),
                ) {
                    user_interface.print(line);
                }

                if let Some(choice_keys) = choices_ordered {
                    for choice in choice_keys {
                        let description = choices
                            .get(&choice)
                            .expect("internal error - choice must exist");
                        for line in column_renderer.render(
                            MAIN_INDENT + CHOICE_INDENT,
                            &choice,
                            description,
                            &vec![],
                        ) {
                            user_interface.print(line);
                        }
                    }
                }
            }
        }

        user_interface.print("".to_string());
        user_interface.print("options:".to_string());
        for line in column_renderer.render(MAIN_INDENT, &help_flags, HELP_MESSAGE, &vec![]) {
            user_interface.print(line);
        }

        for OptionParameter {
            name,
            short,
            help,
            choices,
            meta,
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
            let option_help = match help {
                Some(message) => format!("{message}"),
                None => "".to_string(),
            };
            let (option_choices, choices_ordered) = if choices.is_empty() {
                ("".to_string(), None)
            } else {
                let mut choices_ordered: Vec<String> = choices.keys().cloned().collect();
                choices_ordered.sort();
                (
                    format!("{{{}}} ", choices_ordered.join(", ")),
                    Some(choices_ordered),
                )
            };
            for line in column_renderer.render(
                MAIN_INDENT,
                &option_flags,
                format!("{option_choices}{option_help}").as_str(),
                meta.as_ref().unwrap_or(&Vec::default()),
            ) {
                user_interface.print(line);
            }

            if let Some(choice_keys) = choices_ordered {
                for choice in choice_keys {
                    let description = choices
                        .get(&choice)
                        .expect("internal error - choice must exist");
                    for line in column_renderer.render(
                        MAIN_INDENT + CHOICE_INDENT,
                        &choice,
                        description,
                        &vec![],
                    ) {
                        user_interface.print(line);
                    }
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
 -h, --help   Show this help
              message and
              exit."#
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
                None,
            )],
            Vec::default(),
            Some(120),
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
 -h, --help             Show this help message and exit.
 -f FLAG, --flag FLAG   message"#
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
                None,
                HashMap::from([
                    ("xyz".to_string(), "do the xyz".to_string()),
                    ("abc".to_string(), "do the abc".to_string()),
                    ("123".to_string(), "do the 123".to_string()),
                ]),
            )],
            Vec::default(),
            Some(120),
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
 -h, --help             Show this help message and exit.
 -f FLAG, --flag FLAG   {123, abc, xyz}
   123                    do the 123
   abc                    do the abc
   xyz                    do the xyz"#
        );
    }

    #[test]
    fn print_help_option_meta() {
        // Setup
        let printer = Printer::new(
            vec![OptionParameter::basic(
                "flag".to_string(),
                Some('f'),
                Nargs::Precisely(1),
                Some("message in a bottle, by the police.".to_string()),
                Some(vec!["the swift".to_string(), "brown fox".to_string()]),
            )],
            Vec::default(),
            Some(72),
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
 -h, --help             Show this help
                        message and
                        exit.
 -f FLAG, --flag FLAG   message in a       the swift   brown fox
                        bottle, by the
                        police."#
        );
    }

    #[test]
    fn print_help_option_meta_with_empty() {
        // Setup
        let printer = Printer::new(
            vec![
                OptionParameter::basic(
                    "flag".to_string(),
                    Some('f'),
                    Nargs::Precisely(1),
                    Some("message in a bottle, by the police.".to_string()),
                    Some(vec!["".to_string(), "brown fox".to_string()]),
                ),
                OptionParameter::basic(
                    "other".to_string(),
                    None,
                    Nargs::Precisely(1),
                    Some("".to_string()),
                    Some(vec!["x".to_string(), "brown fox".to_string()]),
                ),
            ],
            Vec::default(),
            Some(72),
        );
        let interface = InMemoryInterface::default();

        // Execute
        printer.print_help("program", &interface);

        // Verify
        let message = interface.consume_message();
        assert_eq!(
            message,
            r#"usage: program [-h] [-f FLAG] [--other OTHER]

options:
 -h, --help             Show this help
                        message and
                        exit.
 -f FLAG, --flag FLAG   message in a           brown fox
                        bottle, by the
                        police.
 --other OTHER                             x   brown fox"#
        );
    }

    #[test]
    fn print_help_option_meta_without_help() {
        // Setup
        let printer = Printer::new(
            vec![OptionParameter::basic(
                "flag".to_string(),
                Some('f'),
                Nargs::Precisely(1),
                None,
                Some(vec!["the swift".to_string(), "brown fox".to_string()]),
            )],
            Vec::default(),
            Some(72),
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
 -h, --help             Show this help
                        message and
                        exit.
 -f FLAG, --flag FLAG                      the swift   brown fox"#
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
                None,
            )],
            Vec::default(),
            Some(120),
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
 -h, --help   Show this help message and
              exit.
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
                None,
            )],
            Vec::default(),
            Some(120),
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
 -h, --help         Show this help message and exit.
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
                None,
            )],
            Vec::default(),
            Some(120),
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
 -h, --help          Show this help message and exit.
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
                None,
            )],
            Vec::default(),
            Some(120),
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
 -h, --help          Show this help message and exit.
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
                None,
            )],
            Some(120),
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
 NAME         message

options:
 -h, --help   Show this help message and
              exit."#
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
                None,
                HashMap::from([
                    ("xyz".to_string(), "do the xyz".to_string()),
                    ("abc".to_string(), "do the abc".to_string()),
                    ("123".to_string(), "do the 123".to_string()),
                ]),
            )],
            Some(120),
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
 NAME         {123, abc, xyz}
   123          do the 123
   abc          do the abc
   xyz          do the xyz

options:
 -h, --help   Show this help message and
              exit."#
        );
    }

    #[test]
    fn print_help_argument_meta() {
        // Setup
        let printer = Printer::new(
            Vec::default(),
            vec![ArgumentParameter::basic(
                "name".to_string(),
                Nargs::Precisely(1),
                Some("message in a bottle, by the police.".to_string()),
                Some(vec!["the swift".to_string(), "brown fox".to_string()]),
            )],
            Some(60),
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
 NAME         message in a       the swift   brown fox
              bottle, by the
              police.

options:
 -h, --help   Show this help
              message and
              exit."#
        );
    }

    #[test]
    fn print_help_argument_meta_with_empty() {
        // Setup
        let printer = Printer::new(
            Vec::default(),
            vec![
                ArgumentParameter::basic(
                    "name".to_string(),
                    Nargs::Precisely(1),
                    Some("message in a bottle, by the police.".to_string()),
                    Some(vec!["".to_string(), "brown fox".to_string()]),
                ),
                ArgumentParameter::basic(
                    "other".to_string(),
                    Nargs::Precisely(1),
                    Some("".to_string()),
                    Some(vec!["x".to_string(), "brown fox".to_string()]),
                ),
            ],
            Some(60),
        );
        let interface = InMemoryInterface::default();

        // Execute
        printer.print_help("program", &interface);

        // Verify
        let message = interface.consume_message();
        assert_eq!(
            message,
            r#"usage: program [-h] NAME OTHER

positional arguments:
 NAME         message in a           brown fox
              bottle, by the
              police.
 OTHER                           x   brown fox

options:
 -h, --help   Show this help
              message and
              exit."#
        );
    }

    #[test]
    fn print_help_argument_meta_without_help() {
        // Setup
        let printer = Printer::new(
            Vec::default(),
            vec![ArgumentParameter::basic(
                "name".to_string(),
                Nargs::Precisely(1),
                None,
                Some(vec!["the swift".to_string(), "brown fox".to_string()]),
            )],
            Some(120),
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
 NAME                                            the swift   brown fox

options:
 -h, --help   Show this help message and exit."#
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
                None,
            )],
            Some(120),
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
 -h, --help   Show this help message and
              exit."#
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
                None,
            )],
            Some(120),
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
 -h, --help   Show this help message and
              exit."#
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
                None,
            )],
            Some(120),
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
 -h, --help   Show this help message and
              exit."#
        );
    }

    #[test]
    fn print_help() {
        // Setup
        let printer = Printer::new(
            vec![
                OptionParameter::basic(
                    "car-park".to_string(),
                    Some('x'),
                    Nargs::Any,
                    Some("car message".to_string()),
                    Some(vec!["meta2".to_string()]),
                ),
                OptionParameter::basic(
                    "blue-spring".to_string(),
                    Some('y'),
                    Nargs::Precisely(0),
                    Some("blue message".to_string()),
                    None,
                ),
                OptionParameter::basic(
                    "apple".to_string(),
                    Some('z'),
                    Nargs::Precisely(1),
                    Some("apple message".to_string()),
                    None,
                ),
            ],
            vec![
                ArgumentParameter::basic(
                    "name-bob".to_string(),
                    Nargs::Precisely(1),
                    Some("name message".to_string()),
                    None,
                ),
                ArgumentParameter::basic(
                    "items-x".to_string(),
                    Nargs::Any,
                    Some("items message".to_string()),
                    Some(vec!["meta1".to_string()]),
                ),
            ],
            Some(120),
        );
        let interface = InMemoryInterface::default();

        // Execute
        printer.print_help("program", &interface);

        // Verify
        let message = interface.consume_message();
        assert_eq!(
            message,
            r#"usage: program [-h] [-z APPLE] [-y] [-x [CAR_PARK ...]] NAME_BOB [ITEMS_X ...]

positional arguments:
 NAME_BOB                                       name message
 [ITEMS_X ...]                                  items message                      meta1

options:
 -h, --help                                     Show this help message and exit.
 -z APPLE, --apple APPLE                        apple message
 -y, --blue-spring                              blue message
 -x [CAR_PARK ...], --car-park [CAR_PARK ...]   car message                        meta2"#
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
                    None,
                ),
                OptionParameter::new(
                    "apple".to_string(),
                    Some('z'),
                    Nargs::Precisely(1),
                    Some("extra".to_string()),
                    None,
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
                    None,
                ),
                ArgumentParameter::basic(
                    "items".to_string(),
                    Nargs::Any,
                    Some("items message".to_string()),
                    None,
                ),
            ],
            Some(120),
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
 NAME                           name message
 [ITEMS ...]                    items message

options:
 -h, --help                     Show this help message and exit.
 -z APPLE, --apple APPLE        {abcdefghijklmnopqrstuvwxyz} extra
   abcdefghijklmnopqrstuvwxyz     abcdefghijklmnopqrstuvwxyz
 -y, --blue                     blue message"#
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
                None,
            )],
            vec![
                ArgumentParameter::new(
                    "name".to_string(),
                    Nargs::Precisely(1),
                    Some("extra".to_string()),
                    None,
                    HashMap::from([(
                        "abcdefghijklmnopqrstuvwxyz".to_string(),
                        "abcdefghijklmnopqrstuvwxyz".to_string(),
                    )]),
                ),
                ArgumentParameter::basic(
                    "items".to_string(),
                    Nargs::Any,
                    Some("items message".to_string()),
                    None,
                ),
            ],
            Some(120),
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
 NAME                           {abcdefghijklmnopqrstuvwxyz} extra
   abcdefghijklmnopqrstuvwxyz     abcdefghijklmnopqrstuvwxyz
 [ITEMS ...]                    items message

options:
 -h, --help                     Show this help message and exit.
 -y, --blue                     blue message"#
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
