use crate::parser::base::ParseError;
use crate::parser::ErrorContext;

pub(crate) trait UserInterface {
    fn print(&self, message: String);
    fn print_error(&self, error: ParseError);
    fn print_error_context(&self, error_context: ErrorContext);
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

    fn print_error_context(&self, error_context: ErrorContext) {
        eprintln!("{error_context}");
    }
}

#[cfg(test)]
pub(crate) mod util {
    use crate::parser::{ErrorContext, ParseError, UserInterface};
    use std::cell::RefCell;
    use std::sync::mpsc;

    pub(crate) struct InMemoryInterface {
        message: RefCell<Option<Vec<String>>>,
        error: RefCell<Option<String>>,
        error_context: RefCell<Option<ErrorContext>>,
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
                (*output).as_mut().unwrap().push(message);
            } else {
                (*output).replace(vec![message]);
            }
        }

        fn print_error(&self, error: ParseError) {
            // Assumes print_error() is only ever called once.
            self.error.borrow_mut().replace(error.to_string());
        }

        fn print_error_context(&self, error_context: ErrorContext) {
            // Assumes print_error_context() is only ever called once.
            self.error_context.borrow_mut().replace(error_context);
        }
    }

    impl InMemoryInterface {
        pub(crate) fn consume(self) -> (Option<String>, Option<String>, Option<ErrorContext>) {
            let InMemoryInterface {
                message,
                error,
                error_context,
            } = self;

            (
                message.take().map(|messages| messages.join("\n")),
                error.take(),
                error_context.take(),
            )
        }

        pub(crate) fn consume_message(self) -> String {
            let (message, error, error_context) = self.consume();
            assert_eq!(error, None);
            assert_eq!(error_context, None);
            message.unwrap()
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
        error_context_tx: mpsc::Sender<Option<ErrorContext>>,
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

        fn print_error_context(&self, error_context: ErrorContext) {
            // Assumes print_error_context() is only ever called once, with the receiver only taking the first.
            self.error_context_tx.send(Some(error_context)).unwrap();
        }
    }

    pub(crate) struct ReceiverInterface {
        message_rx: mpsc::Receiver<Option<String>>,
        error_rx: mpsc::Receiver<Option<String>>,
        error_context_rx: mpsc::Receiver<Option<ErrorContext>>,
    }

    impl ReceiverInterface {
        pub(crate) fn consume(self) -> (Option<String>, Option<String>, Option<ErrorContext>) {
            let ReceiverInterface {
                message_rx,
                error_rx,
                error_context_rx,
            } = self;

            (
                drain(message_rx),
                drain(error_rx),
                // Assumes print_error_context() is only ever called once
                // (we take the first if multiple were sent on the channel).
                error_context_rx.recv().unwrap(),
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
