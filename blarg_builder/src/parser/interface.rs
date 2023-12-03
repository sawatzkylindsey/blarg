use crate::parser::base::ParseError;
use crate::parser::ErrorContext;

#[cfg(feature = "debug")]
use tracing::debug;

#[derive(Debug)]
pub(crate) struct PaddingWidth(usize);

impl PaddingWidth {
    pub(crate) fn new(width: usize) -> Result<Self, ()> {
        // padding must be at least 1
        if width >= 1 {
            Ok(PaddingWidth(width))
        } else {
            Err(())
        }
    }
}

#[derive(Debug)]
pub(crate) struct LeftWidth(usize);

impl LeftWidth {
    pub(crate) fn new(width: usize) -> Result<Self, ()> {
        // left must be at least 1
        if width >= 1 {
            Ok(LeftWidth(width))
        } else {
            Err(())
        }
    }
}

#[derive(Debug)]
pub(crate) struct MiddleWidth(usize);

impl MiddleWidth {
    pub(crate) fn new(width: usize) -> Result<Self, ()> {
        // middle must be at least 2 (so we can hyphenate)
        if width >= 2 {
            Ok(MiddleWidth(width))
        } else {
            Err(())
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RightWidth(usize);

impl RightWidth {
    pub(crate) fn new(width: usize) -> Result<Self, ()> {
        // right must be at least 1
        if width >= 1 {
            Ok(RightWidth(width))
        } else {
            Err(())
        }
    }

    pub(crate) fn value(&self) -> usize {
        self.0
    }
}

#[derive(Debug)]
pub(crate) struct TotalWidth(pub usize);

#[derive(Debug)]
pub(crate) struct ColumnRenderer {
    padding: PaddingWidth,
    left: LeftWidth,
    middle: MiddleWidth,
    rights: Vec<RightWidth>,
}

// We'll target 95% of the total width, to ensure the renderer doesn't literally use the full space.
const TARGET_TOTAL_FACTOR: f64 = 0.95;

// Let's assume the average word length is 5.
// Then 17 is a good minimum, because it allows precisely 3 words with a space between them.
pub(crate) const MINIMUM_MIDDLE_WIDTH: usize = 17;

impl ColumnRenderer {
    /// Produce a renderer based off the provided widths.
    /// This renderer will use a heuristic to chose the middle width.
    pub(crate) fn guided(
        padding: PaddingWidth,
        left: LeftWidth,
        middle: MiddleWidth,
        rights: Vec<RightWidth>,
        total_width: TotalWidth,
    ) -> Self {
        // We always have a left and a middle (and a padding between them).
        let mut non_middle: usize = &left.0 + &padding.0;

        if !rights.is_empty() {
            non_middle += &padding.0
                + rights.iter().map(|r| r.0).sum::<usize>()
                + ((rights.len() - 1) * &padding.0);
        }

        let target_total_width = (total_width.0 as f64 * TARGET_TOTAL_FACTOR) as usize;
        let guided_middle = std::cmp::max(middle.0, MINIMUM_MIDDLE_WIDTH);

        if guided_middle + non_middle <= target_total_width {
            #[cfg(feature = "debug")]
            {
                debug!("Columns {non_middle} and middle fit within the target total {target_total_width}.  Selecting middle: {guided_middle}.");
            }

            Self::new(padding, left, MiddleWidth(guided_middle), rights)
        } else if non_middle < total_width.0 {
            let calculated_middle = std::cmp::max(total_width.0 - non_middle, MINIMUM_MIDDLE_WIDTH);
            #[cfg(feature = "debug")]
            {
                debug!("Columns {non_middle} fits within the total {total_width}.  Selecting middle: {calculated_middle}.");
            }

            Self::new(padding, left, MiddleWidth(calculated_middle), rights)
        } else {
            #[cfg(feature = "debug")]
            {
                debug!("Columns {non_middle} do not fit within the total {total_width}.  Selecting middle: {MINIMUM_MIDDLE_WIDTH}.");
            }

            Self::new(padding, left, MiddleWidth(MINIMUM_MIDDLE_WIDTH), rights)
        }
    }

    /// Produce a renderer based off the provided widths.
    pub(crate) fn new(
        padding: PaddingWidth,
        left: LeftWidth,
        middle: MiddleWidth,
        rights: Vec<RightWidth>,
    ) -> Self {
        Self {
            padding,
            left,
            middle,
            rights,
        }
    }

    pub(crate) fn render(
        &self,
        indent: usize,
        left: &str,
        middle: &str,
        rights: &Vec<String>,
    ) -> Vec<String> {
        assert!(rights.len() <= self.rights.len());
        let padding = &self.padding.0;
        let padding = format!("{:padding$}", "");
        let mut right = String::default();

        if !rights.is_empty() {
            right = padding.clone();

            for (i, item) in rights.iter().enumerate() {
                let width = &self.rights[i].0;
                assert!(item.len() <= *width);

                if &i + 1 < rights.len() {
                    right.push_str(format!("{:width$}{padding}", item).as_str());
                } else {
                    if &item.len() < width {
                        right.push_str(format!("{}", item).as_str());
                    } else {
                        right.push_str(format!("{:width$}", item).as_str());
                    }
                }
            }
        }

        let left_column_width = &self.left.0;
        assert!(&left.len() <= left_column_width);
        let middle_column_width = &self.middle.0 - indent;
        let middle_parts = chunk(middle, middle_column_width);
        let mut out = Vec::default();

        for (i, part) in middle_parts.iter().enumerate() {
            if i == 0 {
                if right.is_empty() {
                    out.push(format!(
                        "{:indent$}{:left_column_width$}{padding}{}",
                        "", left, part
                    ));
                } else {
                    assert!(&part.len() <= &middle_column_width);
                    out.push(format!(
                        "{:indent$}{:left_column_width$}{padding}{:middle_column_width$}{right}",
                        "", left, part
                    ));
                }
            } else {
                out.push(format!(
                    "{:indent$}{:left_column_width$}{padding}{}",
                    "", "", part
                ));
            }
        }

        if out.is_empty() {
            assert!(middle_parts.is_empty());
            if right.is_empty() {
                out.push(format!("{:indent$}{:left_column_width$}", "", left));
            } else {
                out.push(format!(
                    "{:indent$}{:left_column_width$}{padding}{:middle_column_width$}{right}",
                    "", left, ""
                ));
            }
        }

        out
    }
}

fn chunk(paragraph: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::default();
    let mut current = String::default();

    for word in paragraph.split(' ') {
        if !word.is_empty() {
            if current.is_empty() {
                hyphenate(width, &mut lines, &mut current, word);
            } else {
                if current.len() + word.len() + 1 <= width {
                    current.push(' ');
                    current.push_str(word);
                } else {
                    lines.push(current);
                    current = String::default();
                    hyphenate(width, &mut lines, &mut current, word);
                }
            }
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }

    lines
}

fn hyphenate(width: usize, lines: &mut Vec<String>, current: &mut String, word: &str) {
    let increment = width - 1;
    let mut left = 0;
    let mut right = increment.clone();

    while &right + 1 < word.len() {
        lines.push(format!("{}-", &word[left..right]));
        left += &increment;
        right += &increment;
    }

    current.push_str(&word[left..]);
}

pub(crate) trait UserInterface {
    fn print(&self, message: String);
    fn print_error(&self, error: ParseError);
    fn print_error_context(&self, error_context: ErrorContext);
}

#[derive(Default)]
pub(crate) struct ConsoleInterface {}

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

        pub(crate) fn consume_message(self) -> String {
            let (message, error, error_context) = self.consume();
            assert_eq!(error, None);
            assert_eq!(error_context, None);
            message.unwrap()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn column_renderer_simple() {
        let cr = ColumnRenderer::new(
            PaddingWidth::new(4).unwrap(),
            LeftWidth::new(5).unwrap(),
            MiddleWidth::new(23).unwrap(),
            vec![],
        );

        assert_eq!(
            cr.render(0, "abc", "something", &vec![]),
            vec!["abc      something".to_string()]
        );
        assert_eq!(
            cr.render(0, "abc", "  something  ", &vec![]),
            vec!["abc      something".to_string()]
        );

        assert_eq!(
            cr.render(0, "abc12", "something pieces full", &vec![]),
            vec!["abc12    something pieces full".to_string()]
        );
        assert_eq!(
            cr.render(0, "abc", "something pieces full more stuff", &vec![]),
            vec![
                "abc      something pieces full".to_string(),
                "         more stuff".to_string(),
            ]
        );

        assert_eq!(
            cr.render(0, "abc", "something pieces fully more stuff", &vec![]),
            vec![
                "abc      something pieces fully".to_string(),
                "         more stuff".to_string(),
            ]
        );
        assert_eq!(
            cr.render(0, "abc", "something pieces fuller more stuff", &vec![]),
            vec![
                "abc      something pieces fuller".to_string(),
                "         more stuff".to_string(),
            ]
        );
        assert_eq!(
            cr.render(
                0,
                "abc",
                "something pieces fullest more stuff extra     ",
                &vec![]
            ),
            vec![
                "abc      something pieces".to_string(),
                "         fullest more stuff".to_string(),
                "         extra".to_string(),
            ]
        );
    }

    #[test]
    fn column_renderer() {
        let cr = ColumnRenderer::new(
            PaddingWidth::new(4).unwrap(),
            LeftWidth::new(5).unwrap(),
            MiddleWidth::new(8).unwrap(),
            vec![
                RightWidth::new(5).unwrap(),
                RightWidth::new(2).unwrap(),
                RightWidth::new(5).unwrap(),
            ],
        );

        assert_eq!(
            cr.render(
                0,
                "abc",
                "my stuff",
                &vec!["a".to_string(), "b".to_string(), "c".to_string()]
            ),
            vec!["abc      my stuff    a        b     c".to_string()]
        );
        assert_eq!(
            cr.render(
                0,
                "abc",
                "my stuff",
                &vec!["a".to_string(), "".to_string(), "c".to_string()]
            ),
            vec!["abc      my stuff    a              c".to_string()]
        );
        assert_eq!(
            cr.render(
                0,
                "abc12",
                "my stuff",
                &vec!["abcde".to_string(), "bc".to_string(), "cdefg".to_string()]
            ),
            vec!["abc12    my stuff    abcde    bc    cdefg".to_string()]
        );

        assert_eq!(
            cr.render(
                0,
                "abc",
                "my stuff and some",
                &vec!["a".to_string(), "b".to_string(), "c".to_string()]
            ),
            vec![
                "abc      my stuff    a        b     c".to_string(),
                "         and some".to_string(),
            ]
        );
        assert_eq!(
            cr.render(
                0,
                "abc",
                "my stuff and some",
                &vec!["a".to_string(), "".to_string(), "c".to_string()]
            ),
            vec![
                "abc      my stuff    a              c".to_string(),
                "         and some".to_string(),
            ]
        );
        assert_eq!(
            cr.render(
                0,
                "abc12",
                "my stuff and some",
                &vec!["abcde".to_string(), "bc".to_string(), "cdefg".to_string()]
            ),
            vec![
                "abc12    my stuff    abcde    bc    cdefg".to_string(),
                "         and some".to_string(),
            ]
        );
    }

    #[test]
    fn column_renderer_middle_overflow() {
        let cr = ColumnRenderer::new(
            PaddingWidth::new(4).unwrap(),
            LeftWidth::new(5).unwrap(),
            MiddleWidth::new(23).unwrap(),
            vec![],
        );

        assert_eq!(
            cr.render(0, "abc", "somethingxpiecesxfuller", &vec![]),
            vec!["abc      somethingxpiecesxfuller".to_string()]
        );
        assert_eq!(
            cr.render(
                0,
                "abc",
                "somethingxpiecesxfullerandthenwecontinueforalongtime",
                &vec![]
            ),
            vec![
                "abc      somethingxpiecesxfulle-".to_string(),
                "         randthenwecontinuefora-".to_string(),
                "         longtime".to_string(),
            ]
        );
        assert_eq!(
            cr.render(
                0,
                "abc",
                "somethingxpiecesxfullerandthenwecontinueforalongtimeuntildonexxxxxx",
                &vec![]
            ),
            vec![
                "abc      somethingxpiecesxfulle-".to_string(),
                "         randthenwecontinuefora-".to_string(),
                "         longtimeuntildonexxxxxx".to_string(),
            ]
        );

        assert_eq!(
            cr.render(0, "abc", "something pieces fuller", &vec![]),
            vec!["abc      something pieces fuller".to_string()]
        );
        assert_eq!(
            cr.render(
                0,
                "abc",
                "something pieces fullerandthenwecontinueforalongtime",
                &vec![]
            ),
            vec![
                "abc      something pieces".to_string(),
                "         fullerandthenwecontinu-".to_string(),
                "         eforalongtime".to_string(),
            ]
        );
    }

    #[test]
    fn column_renderer_middle_empty() {
        let cr = ColumnRenderer::new(
            PaddingWidth::new(4).unwrap(),
            LeftWidth::new(5).unwrap(),
            MiddleWidth::new(8).unwrap(),
            vec![
                RightWidth::new(5).unwrap(),
                RightWidth::new(2).unwrap(),
                RightWidth::new(5).unwrap(),
            ],
        );

        // TODO: Fix the trailing whitespace on this one.
        assert_eq!(cr.render(0, "abc", "", &vec![]), vec!["abc  ".to_string()]);
        assert_eq!(
            cr.render(
                0,
                "abc",
                "",
                &vec!["a".to_string(), "b".to_string(), "c".to_string()]
            ),
            vec!["abc                  a        b     c".to_string()]
        );
        assert_eq!(
            cr.render(
                0,
                "abc",
                "",
                &vec!["a".to_string(), "".to_string(), "c".to_string()]
            ),
            vec!["abc                  a              c".to_string()]
        );
        assert_eq!(
            cr.render(
                0,
                "abc12",
                "",
                &vec!["abcde".to_string(), "bc".to_string(), "cdefg".to_string()]
            ),
            vec!["abc12                abcde    bc    cdefg".to_string()]
        );
    }

    #[test]
    fn column_renderer_indent() {
        let cr = ColumnRenderer::new(
            PaddingWidth::new(4).unwrap(),
            LeftWidth::new(5).unwrap(),
            MiddleWidth::new(10).unwrap(),
            vec![
                RightWidth::new(5).unwrap(),
                RightWidth::new(2).unwrap(),
                RightWidth::new(5).unwrap(),
            ],
        );

        assert_eq!(
            cr.render(1, "abc", "something", &vec![]),
            vec![" abc      something".to_string()]
        );
        assert_eq!(
            cr.render(1, "abc", "somethingx", &vec![]),
            vec![
                " abc      somethin-".to_string(),
                "          gx".to_string(),
            ]
        );
        assert_eq!(
            cr.render(2, "abc", "somethin", &vec![]),
            vec!["  abc      somethin".to_string()]
        );

        assert_eq!(
            cr.render(
                1,
                "abc",
                "something",
                &vec!["a".to_string(), "b".to_string(), "c".to_string()]
            ),
            vec![" abc      something    a        b     c".to_string()]
        );
        assert_eq!(
            cr.render(
                1,
                "abc",
                "somethingx",
                &vec!["a".to_string(), "b".to_string(), "c".to_string()]
            ),
            vec![
                " abc      somethin-    a        b     c".to_string(),
                "          gx".to_string(),
            ]
        );
        assert_eq!(
            cr.render(
                2,
                "abc",
                "somethi",
                &vec!["a".to_string(), "b".to_string(), "c".to_string()]
            ),
            vec!["  abc      somethi     a        b     c".to_string(),]
        );

        assert_eq!(
            cr.render(
                1,
                "abc12",
                "something",
                &vec!["abcde".to_string(), "bc".to_string(), "cdefg".to_string()]
            ),
            vec![" abc12    something    abcde    bc    cdefg".to_string()]
        );
        assert_eq!(
            cr.render(
                1,
                "abc12",
                "somethingx",
                &vec!["abcde".to_string(), "bc".to_string(), "cdefg".to_string()]
            ),
            vec![
                " abc12    somethin-    abcde    bc    cdefg".to_string(),
                "          gx".to_string(),
            ]
        );
        assert_eq!(
            cr.render(
                2,
                "abc12",
                "somethin",
                &vec!["abcde".to_string(), "bc".to_string(), "cdefg".to_string()]
            ),
            vec!["  abc12    somethin    abcde    bc    cdefg".to_string()]
        );
    }

    #[test]
    #[should_panic]
    fn column_renderer_left_overflow() {
        let cr = ColumnRenderer::new(
            PaddingWidth::new(4).unwrap(),
            LeftWidth::new(5).unwrap(),
            MiddleWidth::new(10).unwrap(),
            vec![],
        );
        cr.render(0, "abcdef", "something", &vec![]);
    }

    #[test]
    #[should_panic]
    fn column_renderer_right_overflow() {
        let cr = ColumnRenderer::new(
            PaddingWidth::new(4).unwrap(),
            LeftWidth::new(5).unwrap(),
            MiddleWidth::new(10).unwrap(),
            vec![RightWidth::new(1).unwrap()],
        );
        cr.render(0, "abcdef", "something", &vec!["ab".to_string()]);
    }

    #[test]
    #[should_panic]
    fn column_renderer_right_too_many() {
        let cr = ColumnRenderer::new(
            PaddingWidth::new(4).unwrap(),
            LeftWidth::new(5).unwrap(),
            MiddleWidth::new(10).unwrap(),
            vec![RightWidth::new(5).unwrap()],
        );
        cr.render(
            0,
            "abcdef",
            "something",
            &vec!["ab".to_string(), "cd".to_string()],
        );
    }

    #[test]
    fn column_renderer_guided() {
        //
        // When the total width is too short (for even the non middle).
        //
        let cr = ColumnRenderer::guided(
            PaddingWidth::new(2).unwrap(),
            LeftWidth::new(5).unwrap(),
            MiddleWidth::new(2).unwrap(),
            vec![],
            TotalWidth(7),
        );
        assert_eq!(cr.middle.0, MINIMUM_MIDDLE_WIDTH);

        //
        // When the total width is too short (for it all).
        //
        let cr = ColumnRenderer::guided(
            PaddingWidth::new(2).unwrap(),
            LeftWidth::new(5).unwrap(),
            MiddleWidth::new(2).unwrap(),
            vec![],
            TotalWidth(15),
        );
        assert_eq!(cr.middle.0, MINIMUM_MIDDLE_WIDTH);

        let cr = ColumnRenderer::guided(
            PaddingWidth::new(2).unwrap(),
            LeftWidth::new(5).unwrap(),
            MiddleWidth::new(MINIMUM_MIDDLE_WIDTH + 1).unwrap(),
            vec![],
            TotalWidth(15),
        );
        assert_eq!(cr.middle.0, MINIMUM_MIDDLE_WIDTH);

        //
        // When the total width is just right.
        //
        let cr = ColumnRenderer::guided(
            PaddingWidth::new(2).unwrap(),
            LeftWidth::new(5).unwrap(),
            MiddleWidth::new(MINIMUM_MIDDLE_WIDTH).unwrap(),
            vec![],
            TotalWidth(26),
        );
        assert_eq!(cr.middle.0, MINIMUM_MIDDLE_WIDTH);

        let cr = ColumnRenderer::guided(
            PaddingWidth::new(2).unwrap(),
            LeftWidth::new(5).unwrap(),
            MiddleWidth::new(MINIMUM_MIDDLE_WIDTH + 1).unwrap(),
            vec![],
            TotalWidth(27),
        );
        assert_eq!(cr.middle.0, MINIMUM_MIDDLE_WIDTH + 1);

        let cr = ColumnRenderer::guided(
            PaddingWidth::new(2).unwrap(),
            LeftWidth::new(5).unwrap(),
            MiddleWidth::new(MINIMUM_MIDDLE_WIDTH + 2).unwrap(),
            vec![],
            TotalWidth(27),
        );
        assert_eq!(cr.middle.0, MINIMUM_MIDDLE_WIDTH + 3);

        //
        // When the total width is too long.
        //
        let cr = ColumnRenderer::guided(
            PaddingWidth::new(2).unwrap(),
            LeftWidth::new(5).unwrap(),
            MiddleWidth::new(MINIMUM_MIDDLE_WIDTH).unwrap(),
            vec![],
            TotalWidth(50),
        );
        assert_eq!(cr.middle.0, MINIMUM_MIDDLE_WIDTH);

        let cr = ColumnRenderer::guided(
            PaddingWidth::new(2).unwrap(),
            LeftWidth::new(5).unwrap(),
            MiddleWidth::new(MINIMUM_MIDDLE_WIDTH + 10).unwrap(),
            vec![],
            TotalWidth(50),
        );
        assert_eq!(cr.middle.0, MINIMUM_MIDDLE_WIDTH + 10);
    }

    #[test]
    fn column_renderer_guided_right() {
        //
        // When the total width is too short (for even the non middle).
        //
        let cr = ColumnRenderer::guided(
            PaddingWidth::new(2).unwrap(),
            LeftWidth::new(5).unwrap(),
            MiddleWidth::new(2).unwrap(),
            vec![RightWidth::new(1).unwrap()],
            TotalWidth(10),
        );
        assert_eq!(cr.middle.0, MINIMUM_MIDDLE_WIDTH);

        //
        // When the total width is too short (for it all).
        //
        let cr = ColumnRenderer::guided(
            PaddingWidth::new(2).unwrap(),
            LeftWidth::new(5).unwrap(),
            MiddleWidth::new(2).unwrap(),
            vec![RightWidth::new(1).unwrap()],
            TotalWidth(15),
        );
        assert_eq!(cr.middle.0, MINIMUM_MIDDLE_WIDTH);

        let cr = ColumnRenderer::guided(
            PaddingWidth::new(2).unwrap(),
            LeftWidth::new(5).unwrap(),
            MiddleWidth::new(MINIMUM_MIDDLE_WIDTH + 1).unwrap(),
            vec![RightWidth::new(1).unwrap()],
            TotalWidth(15),
        );
        assert_eq!(cr.middle.0, MINIMUM_MIDDLE_WIDTH);

        //
        // When the total width is just right.
        //
        let cr = ColumnRenderer::guided(
            PaddingWidth::new(2).unwrap(),
            LeftWidth::new(5).unwrap(),
            MiddleWidth::new(MINIMUM_MIDDLE_WIDTH).unwrap(),
            vec![RightWidth::new(1).unwrap()],
            TotalWidth(29),
        );
        assert_eq!(cr.middle.0, MINIMUM_MIDDLE_WIDTH);

        let cr = ColumnRenderer::guided(
            PaddingWidth::new(2).unwrap(),
            LeftWidth::new(5).unwrap(),
            MiddleWidth::new(MINIMUM_MIDDLE_WIDTH + 1).unwrap(),
            vec![RightWidth::new(1).unwrap()],
            TotalWidth(30),
        );
        assert_eq!(cr.middle.0, MINIMUM_MIDDLE_WIDTH + 1);

        let cr = ColumnRenderer::guided(
            PaddingWidth::new(2).unwrap(),
            LeftWidth::new(5).unwrap(),
            MiddleWidth::new(MINIMUM_MIDDLE_WIDTH + 2).unwrap(),
            vec![RightWidth::new(1).unwrap()],
            TotalWidth(30),
        );
        assert_eq!(cr.middle.0, MINIMUM_MIDDLE_WIDTH + 3);

        //
        // When the total width is too long.
        //
        let cr = ColumnRenderer::guided(
            PaddingWidth::new(2).unwrap(),
            LeftWidth::new(5).unwrap(),
            MiddleWidth::new(MINIMUM_MIDDLE_WIDTH).unwrap(),
            vec![RightWidth::new(1).unwrap()],
            TotalWidth(50),
        );
        assert_eq!(cr.middle.0, MINIMUM_MIDDLE_WIDTH);

        let cr = ColumnRenderer::guided(
            PaddingWidth::new(2).unwrap(),
            LeftWidth::new(5).unwrap(),
            MiddleWidth::new(MINIMUM_MIDDLE_WIDTH + 10).unwrap(),
            vec![RightWidth::new(1).unwrap()],
            TotalWidth(50),
        );
        assert_eq!(cr.middle.0, MINIMUM_MIDDLE_WIDTH + 10);
    }
}
