use crate::parser::base::ParseError;
use std::cell::RefCell;

pub(crate) trait UserInterface {
    fn print(&self, message: String);
    fn print_error(&self, error: ParseError);
    fn print_error_context(&self, offset: usize, tokens: &[&str]);
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

    fn print_error_context(&self, offset: usize, tokens: &[&str]) {
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

pub(crate) struct InMemory {
    pub(crate) message: RefCell<Option<String>>,
    pub(crate) error: RefCell<Option<String>>,
    pub(crate) error_context: RefCell<Option<(usize, Vec<String>)>>,
}

impl Default for InMemory {
    fn default() -> Self {
        InMemory {
            message: RefCell::new(None),
            error: RefCell::new(None),
            error_context: RefCell::new(None),
        }
    }
}

impl UserInterface for InMemory {
    fn print(&self, message: String) {
        let mut output = self.message.borrow_mut();

        if output.is_some() {
            (*output).as_mut().unwrap().push_str(&message);
        } else {
            (*output).replace(message);
        }
    }

    fn print_error(&self, error: ParseError) {
        self.error.borrow_mut().replace(error.to_string());
    }

    fn print_error_context(&self, offset: usize, tokens: &[&str]) {
        self.error_context
            .borrow_mut()
            .replace((offset, tokens.iter().map(|s| s.to_string()).collect()));
    }
}
