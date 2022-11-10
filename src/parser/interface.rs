use crate::parser::base::ParseError;

pub(crate) trait UserInterface {
    fn print(&self, message: String);
    fn print_error(&self, error: ParseError);
    fn print_error_context(&self, offset: usize, tokens: &[&str]);
}

pub(crate) struct ConsoleInterface {}

impl Default for ConsoleInterface {
    fn default() -> Self {
        Self {}
    }
}

impl UserInterface for ConsoleInterface {
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

#[cfg(test)]
pub(crate) mod util {
    use crate::parser::{ParseError, UserInterface};
    use std::cell::RefCell;
    use std::sync::mpsc;

    pub(crate) struct InMemoryInterface {
        message: RefCell<Option<String>>,
        error: RefCell<Option<String>>,
        error_context: RefCell<Option<(usize, Vec<String>)>>,
    }

    impl Default for InMemoryInterface {
        fn default() -> Self {
            Self {
                message: RefCell::new(None),
                error: RefCell::new(None),
                error_context: RefCell::new(None),
            }
        }
    }

    impl UserInterface for InMemoryInterface {
        fn print(&self, message: String) {
            // Allows for print() to be called many times, concatenating the messages.
            let mut output = self.message.borrow_mut();

            if output.is_some() {
                (*output).as_mut().unwrap().push_str(&message);
            } else {
                (*output).replace(message);
            }
        }

        fn print_error(&self, error: ParseError) {
            // Assumes print_error() is only ever called once.
            self.error.borrow_mut().replace(error.to_string());
        }

        fn print_error_context(&self, offset: usize, tokens: &[&str]) {
            // Assumes print_error_context() is only ever called once.
            self.error_context
                .borrow_mut()
                .replace((offset, tokens.iter().map(|s| s.to_string()).collect()));
        }
    }

    impl InMemoryInterface {
        pub(crate) fn consume(self) -> (Option<String>, Option<String>, Option<(usize, String)>) {
            let InMemoryInterface {
                message,
                error,
                error_context,
            } = self;

            (
                message.take(),
                error.take(),
                error_context
                    .take()
                    .map(|(offset, tokens)| (offset, tokens.join(" "))),
            )
        }
    }

    pub(crate) fn channel_interface() -> (SenderInterface, ReceiverInterface) {
        let (message_tx, message_rx) = mpsc::channel();
        let (error_tx, error_rx) = mpsc::channel();
        let (error_context_tx, error_context_rx) = mpsc::channel();
        let sender = SenderInterface {
            message_tx,
            error_tx,
            error_context_tx,
        };
        let receiver = ReceiverInterface {
            message_rx,
            error_rx,
            error_context_rx,
        };
        (sender, receiver)
    }

    pub(crate) struct SenderInterface {
        message_tx: mpsc::Sender<Option<String>>,
        error_tx: mpsc::Sender<Option<String>>,
        error_context_tx: mpsc::Sender<Option<(usize, Vec<String>)>>,
    }

    impl Drop for SenderInterface {
        fn drop(&mut self) {
            self.message_tx.send(None).unwrap();
            self.error_tx.send(None).unwrap();
            self.error_context_tx.send(None).unwrap();
        }
    }

    impl UserInterface for SenderInterface {
        fn print(&self, message: String) {
            // Allows for print() to be called many times, with the receiver concatenating the messages.
            self.message_tx.send(Some(message)).unwrap();
        }

        fn print_error(&self, error: ParseError) {
            // Allows for print() to be called many times, with the receiver concatenating the messages.
            self.error_tx.send(Some(error.to_string())).unwrap();
        }

        fn print_error_context(&self, offset: usize, tokens: &[&str]) {
            // Assumes print_error_context() is only ever called once, with the receiver only taking the last.
            self.error_context_tx
                .send(Some((
                    offset,
                    tokens.iter().map(|s| s.to_string()).collect(),
                )))
                .unwrap();
        }
    }

    pub(crate) struct ReceiverInterface {
        message_rx: mpsc::Receiver<Option<String>>,
        error_rx: mpsc::Receiver<Option<String>>,
        error_context_rx: mpsc::Receiver<Option<(usize, Vec<String>)>>,
    }

    impl ReceiverInterface {
        pub(crate) fn consume(self) -> (Option<String>, Option<String>, Option<(usize, String)>) {
            let ReceiverInterface {
                message_rx,
                error_rx,
                error_context_rx,
            } = self;

            (
                drain(message_rx),
                drain(error_rx),
                // Assumes print_error_context() is only ever called once (aka: we take the last).
                error_context_rx
                    .recv()
                    .unwrap()
                    .map(|(offset, tokens)| (offset, tokens.join(" "))),
            )
        }
    }

    fn drain(receiver: mpsc::Receiver<Option<String>>) -> Option<String> {
        let mut values = Vec::default();

        loop {
            match receiver.recv().unwrap() {
                Some(message) => values.push(message),
                None => break,
            }
        }

        if values.is_empty() {
            None
        } else {
            Some(values.join("\n"))
        }
    }
}
