use crate::parser::ParseError;

pub(crate) trait UserInterface {
    fn print(&self, message: String);
    fn print_error(&self, error: ParseError);
    fn print_context(&self, tokens: &[&str], offset: usize);
}

pub(crate) struct Console {}

impl Default for Console {
    fn default() -> Self {
        Console {}
    }
}

impl UserInterface for Console {
    fn print(&self, message: String) {
        println!("{message}");
    }

    fn print_error(&self, error: ParseError) {
        eprintln!("{error}");
    }

    fn print_context(&self, tokens: &[&str], offset: usize) {
        let mut token_length = 0;
        let mut representation = String::default();
        let mut offset_spaces: Option<usize> = None;

        for (i, token) in tokens.iter().enumerate() {
            if offset_spaces.is_none() {
                if token_length >= offset {
                    offset_spaces.replace(i + 1);
                }
            }

            token_length += token.len();
            representation.push_str(token);

            if i < tokens.len() {
                representation.push_str(" ");
            }
        }

        if offset_spaces.is_none() {
            offset_spaces.replace(tokens.len());
        }

        eprintln!("{representation}");
        eprintln!(
            "{:>width$}",
            "^",
            width = offset + offset_spaces.expect("internal error - must have set offset_spaces")
        );
    }
}
