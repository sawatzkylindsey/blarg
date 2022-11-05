use std::collections::{HashMap, HashSet, VecDeque};
use thiserror::Error;

use crate::matcher::api::*;
use crate::matcher::model::*;

#[derive(Debug, Error, PartialEq, Eq)]
pub(crate) enum TokenMatcherError {
    #[error("Cannot duplicate the option '{0}'.")]
    DuplicateOption(String),

    #[error("Cannot duplicate the short option '{0}'.")]
    DuplicateShortOption(char),
}

#[derive(Debug, Error, PartialEq, Eq)]
pub(crate) enum MatchError {
    #[error("Not enough tokens provided to parameter '{0}'.")]
    Undercomplete(String),

    #[error("Too many tokens provided to parameter '{0}'.")]
    Overcomplete(String),

    #[error("No more arguments to match against.")]
    ArgumentsExhausted,

    #[error("Option '{0}' does not exist.")]
    InvalidOption(String),

    #[error("Short option '{0}' does not exist.")]
    InvalidShortOption(char),
}

impl From<CloseError> for MatchError {
    fn from(error: CloseError) -> Self {
        match error {
            CloseError::TooFewValues { name, .. } => MatchError::Undercomplete(name),
            CloseError::TooManyValues { name, .. } => MatchError::Overcomplete(name),
        }
    }
}

#[derive(Debug)]
pub(crate) struct TokenMatcher {
    option_bounds: HashMap<String, Bound>,
    short_options: HashMap<char, String>,
    arguments: VecDeque<ArgumentConfig>,
    fed: usize,
    matches: Vec<MatchTokens>,
    buffer: Option<MatchBuffer>,
}

impl TokenMatcher {
    pub(crate) fn new(
        options: HashSet<OptionConfig>,
        arguments: VecDeque<ArgumentConfig>,
    ) -> Result<Self, TokenMatcherError> {
        let mut option_bounds = HashMap::default();
        let mut short_options = HashMap::default();

        for option_config in options.into_iter() {
            if option_bounds
                .insert(option_config.name(), option_config.bound())
                .is_some()
            {
                return Err(TokenMatcherError::DuplicateOption(option_config.name()));
            }

            if let Some(short) = option_config.short() {
                if short_options
                    .insert(short.clone(), option_config.name())
                    .is_some()
                {
                    return Err(TokenMatcherError::DuplicateShortOption(short.clone()));
                }
            }
        }

        Ok(Self {
            option_bounds,
            short_options,
            arguments,
            fed: 0,
            matches: Vec::default(),
            buffer: None,
        })
    }

    pub(crate) fn feed(&mut self, token: &str) -> Result<(), MatchError> {
        let token_length = token.len();
        // 1. Find a 'long' flag, such as:
        //  --initial
        //  --initial ..
        //  --initial=..
        // 2. Find 'short' flag(s), such as (both -i and -v are example short flags):
        //  -i
        //  -i..
        //  -i ..
        //  -i=..
        //  -iv..
        //  -iv ..
        //  -iv=..
        // 3. Match against an argument.
        let result = if let Some(token) = token.strip_prefix("--") {
            self.match_option(split_equals_delimiter(token))
        } else if let Some(token) = token.strip_prefix("-") {
            self.match_option_short(split_equals_delimiter(token))
        } else {
            self.match_argument(token)
        };

        self.fed += token_length;
        result
    }

    fn match_argument(&mut self, token: &str) -> Result<(), MatchError> {
        let mut match_buffer = match self.buffer.take() {
            Some(match_buffer) => {
                if match_buffer.is_open() {
                    match_buffer
                } else {
                    // Flip to the next argument
                    let match_tokens = match_buffer.close().expect(
                        "internal error - by definition, a non-open buffer must be able to close",
                    );
                    self.matches.push(match_tokens);
                    self.next_argument()?
                }
            }
            None => {
                // Flip to the next argument.
                self.next_argument()?
            }
        };

        match_buffer.push(self.fed, token.to_string());

        if let Some(_) = self.buffer.replace(match_buffer) {
            unreachable!("internal error - the buffer is expected to be None");
        }

        Ok(())
    }

    fn next_argument(&mut self) -> Result<MatchBuffer, MatchError> {
        match self.arguments.pop_front() {
            Some(argument_config) => Ok(MatchBuffer::new(
                argument_config.name(),
                argument_config.bound(),
            )),
            None => Err(MatchError::ArgumentsExhausted),
        }
    }

    fn match_option(
        &mut self,
        (option_name, single_argument): (&str, Option<&str>),
    ) -> Result<(), MatchError> {
        if let Some(bound) = self.option_bounds.remove(option_name) {
            let mut match_buffer = MatchBuffer::new(option_name.to_string(), bound);

            let next_buffer = match single_argument {
                Some(value) => {
                    // The 3 comes from the option specifier '--' and argument specifier '='.
                    match_buffer.push(self.fed + option_name.len() + 3, value.to_string());

                    // Options using k=v syntax cannot follow up with more values afterwards.
                    let match_tokens = match_buffer.close()?;
                    self.matches.push(match_tokens);
                    None
                }
                None => Some(match_buffer),
            };
            self.update_buffer(next_buffer)
        } else {
            Err(MatchError::InvalidOption(option_name.to_string()))
        }
    }

    fn match_option_short(
        &mut self,
        (short_option_name, single_argument): (&str, Option<&str>),
    ) -> Result<(), MatchError> {
        for (index, single) in short_option_name.chars().enumerate() {
            if let Some(name) = self.short_options.get(&single) {
                if let Some(bound) = self.option_bounds.remove(name) {
                    // If this is the final character from the short option token (the variable 'short_option_name').
                    if index + 1 == short_option_name.len() {
                        // Only the final option may accept values.
                        let mut match_buffer = MatchBuffer::new(name.to_string(), bound);

                        match single_argument {
                            // If an equals delimited value was specified, use it.
                            Some(value) => {
                                // The 2 comes from the short option specifier '-' and argument specifier '='.
                                match_buffer.push(
                                    self.fed + short_option_name.len() + 2,
                                    value.to_string(),
                                );

                                // Options using k=v syntax cannot follow up with more values afterwards.
                                let match_tokens = match_buffer.close()?;
                                self.matches.push(match_tokens);
                            }
                            // If no equals delimited value was specified, allow the values to be fed as subsequent tokens.
                            None => {
                                self.update_buffer(Some(match_buffer))?;
                            }
                        };
                    } else {
                        // All characters in the head of the short option token (the variable 'short_option_name') must allow no values.
                        let match_tokens = MatchBuffer::new(name.to_string(), bound).close()?;
                        self.matches.push(match_tokens);
                    }
                } else {
                    unreachable!("internal error - mis-aligned short option.");
                }

                self.short_options
                    .remove(&single)
                    .expect("internal error - must be able to remove the selected short option");
            } else {
                return Err(MatchError::InvalidShortOption(single));
            }
        }

        Ok(())
    }

    fn update_buffer(&mut self, next_buffer: Option<MatchBuffer>) -> Result<(), MatchError> {
        let previous_buffer = std::mem::replace(&mut self.buffer, next_buffer);

        if let Some(match_buffer) = previous_buffer {
            let match_tokens = match_buffer.close()?;
            self.matches.push(match_tokens);
        }

        Ok(())
    }

    pub(crate) fn can_close(&self) -> bool {
        if let Some(match_buffer) = &self.buffer {
            if !match_buffer.can_close() {
                return false;
            }
        }

        for argument_config in &self.arguments {
            let match_buffer = MatchBuffer::new(argument_config.name(), argument_config.bound());
            if !match_buffer.can_close() {
                return false;
            }
        }

        true

        /*if self.arguments.is_empty() {
            println!("args empty");
                println!("buffer check..");
            } else {
                println!("no buffer check");
                true
            }
        } else {
            false
        }*/
    }

    pub(crate) fn close(mut self) -> Result<Matches, (usize, MatchError, Matches)> {
        let mut close_error: Option<CloseError> = None;

        if let Some(match_buffer) = self.buffer {
            match match_buffer.close() {
                Ok(match_tokens) => {
                    self.matches.push(match_tokens);
                }
                Err(error) => {
                    close_error.replace(error);
                }
            };
        }

        for argument_config in self.arguments {
            let match_buffer = MatchBuffer::new(argument_config.name(), argument_config.bound());
            match match_buffer.close() {
                Ok(match_tokens) => {
                    self.matches.push(match_tokens);
                }
                Err(error) => {
                    // Only track the first error.
                    if close_error.is_none() {
                        close_error.replace(error);
                    }
                }
            };
        }

        let matches = Matches {
            values: self.matches,
        };

        if let Some(error) = close_error {
            Err((self.fed, MatchError::from(error), matches))
        } else {
            Ok(matches)
        }
    }
}

fn split_equals_delimiter(token: &str) -> (&str, Option<&str>) {
    match token.split_once("=") {
        Some((n, v)) => (n, Some(v)),
        None => (token, None),
    }
}

impl Matches {
    pub(crate) fn contains(&self, name: &str) -> bool {
        self.values.iter().any(|mt| &mt.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[test]
    fn option_duplicate() {
        let options = HashSet::from([
            OptionConfig::new("abc".to_string(), None, Bound::Range(1, 1)),
            OptionConfig::new("abc".to_string(), Some('a'), Bound::Range(1, 1)),
        ]);
        let error = TokenMatcher::new(options, VecDeque::default()).unwrap_err();
        assert_eq!(error, TokenMatcherError::DuplicateOption("abc".to_string()));
    }

    #[rstest]
    #[case(Bound::Range(0, 0), 0, true)]
    #[case(Bound::Range(0, 0), 1, false)]
    #[case(Bound::Range(0, 1), 0, true)]
    #[case(Bound::Range(0, 1), 1, true)]
    #[case(Bound::Range(0, 1), 2, false)]
    fn option_range_upper(#[case] bound: Bound, #[case] feed: u8, #[case] expected_ok: bool) {
        // Setup
        let options = HashSet::from([OptionConfig::new("initial".to_string(), None, bound)]);
        let mut tp = TokenMatcher::new(options, VecDeque::default()).unwrap();
        let tokens: Vec<String> = (0..feed).map(|i| i.to_string()).collect();
        let mut feed_error = false;

        // Execute
        tp.feed("--initial").unwrap();
        for (i, token) in tokens.iter().enumerate() {
            let result = tp.feed(token);

            if !expected_ok && i + 1 == feed.into() {
                assert_eq!(result.unwrap_err(), MatchError::ArgumentsExhausted);
                feed_error = true;
            } else {
                result.unwrap();
            }
        }

        // Verify
        if expected_ok {
            let mut offset = 9;
            assert_eq!(
                tp.close().unwrap().values,
                vec![MatchTokens {
                    name: "initial".to_string(),
                    values: tokens
                        .into_iter()
                        .map(|t| {
                            let length = t.len();
                            let out = (offset, t);
                            offset += length;
                            out
                        })
                        .collect(),
                }]
            );
        } else if !feed_error {
            let (offset, error, matches) = tp.close().unwrap_err();
            assert_eq!(offset, feed as usize);
            assert_eq!(error, MatchError::Undercomplete("initial".to_string()));
            assert_eq!(matches.values, vec![]);
        }
    }

    #[rstest]
    #[case(Bound::Lower(0), 0, true)]
    #[case(Bound::Lower(0), 1, true)]
    #[case(Bound::Lower(1), 0, false)]
    #[case(Bound::Lower(1), 1, true)]
    #[case(Bound::Lower(1), 2, true)]
    #[case(Bound::Range(0, 3), 0, true)]
    #[case(Bound::Range(0, 3), 1, true)]
    #[case(Bound::Range(1, 3), 0, false)]
    #[case(Bound::Range(1, 3), 1, true)]
    #[case(Bound::Range(1, 3), 2, true)]
    fn option_lower(#[case] bound: Bound, #[case] feed: u8, #[case] expected_ok: bool) {
        // Setup
        let options = HashSet::from([OptionConfig::new("initial".to_string(), None, bound)]);
        let mut tp = TokenMatcher::new(options, VecDeque::default()).unwrap();
        let tokens: Vec<String> = (0..feed).map(|i| i.to_string()).collect();

        // Execute
        tp.feed("--initial").unwrap();
        for token in &tokens {
            tp.feed(token).unwrap();
        }

        // Verify
        if expected_ok {
            let mut offset = 9;
            assert_eq!(
                tp.close().unwrap().values,
                vec![MatchTokens {
                    name: "initial".to_string(),
                    values: tokens
                        .into_iter()
                        .map(|t| {
                            let length = t.len();
                            let out = (offset, t);
                            offset += length;
                            out
                        })
                        .collect(),
                }]
            );
        } else {
            let (offset, error, matches) = tp.close().unwrap_err();
            assert_eq!(offset, (feed as usize) + 9);
            assert_eq!(error, MatchError::Undercomplete("initial".to_string()));
            assert_eq!(matches.values, vec![]);
        }
    }

    #[rstest]
    #[case(0)]
    #[case(1)]
    #[case(2)]
    #[case(10)]
    #[case(100)]
    fn option_unlimited(#[case] feed: u8) {
        // Setup
        let options = HashSet::from([OptionConfig::new(
            "initial".to_string(),
            None,
            Bound::Lower(0),
        )]);
        let mut tp = TokenMatcher::new(options, VecDeque::default()).unwrap();
        let tokens: Vec<String> = (0..feed).map(|i| i.to_string()).collect();

        // Execute
        tp.feed("--initial").unwrap();
        for token in &tokens {
            tp.feed(token).unwrap();
        }

        // Verify
        let mut offset = 9;
        assert_eq!(
            tp.close().unwrap().values,
            vec![MatchTokens {
                name: "initial".to_string(),
                values: tokens
                    .into_iter()
                    .map(|t| {
                        let length = t.len();
                        let out = (offset, t);
                        offset += length;
                        out
                    })
                    .collect(),
            }]
        );
    }

    #[test]
    fn option_unmatched() {
        let options = HashSet::from([OptionConfig::new(
            "verbose".to_string(),
            None,
            Bound::Lower(0),
        )]);
        let mut tp = TokenMatcher::new(options, VecDeque::default()).unwrap();

        assert_eq!(
            tp.feed("--moot").unwrap_err(),
            MatchError::InvalidOption("moot".to_string())
        );
    }

    #[test]
    fn option_repeat() {
        let options = HashSet::from([OptionConfig::new(
            "verbose".to_string(),
            None,
            Bound::Lower(0),
        )]);
        let mut tp = TokenMatcher::new(options, VecDeque::default()).unwrap();

        tp.feed("--verbose").unwrap();
        assert_eq!(
            tp.feed("--verbose").unwrap_err(),
            MatchError::InvalidOption("verbose".to_string())
        );
    }

    #[rstest]
    #[case(vec!["-v"], true, None)]
    #[case(vec!["-f"], false, Some(vec![]))]
    #[case(vec!["-f", "a"], false, Some(vec![(2, "a")]))]
    #[case(vec!["-f", "a", "bc"], false, Some(vec![(2, "a"), (3, "bc")]))]
    #[case(vec!["-vf"], true, Some(vec![]))]
    #[case(vec!["-vf", "a"], true, Some(vec![(3, "a")]))]
    #[case(vec!["-vf", "a", "bc"], true, Some(vec![(3, "a"), (4, "bc")]))]
    fn option_short(
        #[case] tokens: Vec<&str>,
        #[case] expected_verbose: bool,
        #[case] expected_flags: Option<Vec<(usize, &str)>>,
    ) {
        // Setup
        let options = HashSet::from([
            OptionConfig::new("verbose".to_string(), Some('v'), Bound::Range(0, 0)),
            OptionConfig::new("flag".to_string(), Some('f'), Bound::Lower(0)),
        ]);
        let mut tp = TokenMatcher::new(options, VecDeque::default()).unwrap();

        // Execute
        for token in tokens.iter() {
            tp.feed(token).unwrap();
        }
        let matches = tp.close().unwrap();

        // Verify
        if expected_verbose {
            assert!(matches.contains("verbose"));
            assert!(matches.values.contains(&MatchTokens {
                name: "verbose".to_string(),
                values: Vec::default(),
            }));
        }

        match expected_flags {
            None => {
                assert_eq!(matches.values.len(), if expected_verbose { 1 } else { 0 });
            }
            Some(expected) => {
                assert_eq!(matches.values.len(), if expected_verbose { 2 } else { 1 });
                assert!(matches.contains("flag"));
                assert!(matches.values.contains(&MatchTokens {
                    name: "flag".to_string(),
                    values: expected.iter().map(|(i, e)| (*i, e.to_string())).collect(),
                }));
            }
        };
    }

    #[rstest]
    #[case(vec!["--initial="], Some((10, "")))]
    #[case(vec!["--initial=a"], Some((10, "a")))]
    #[case(vec!["--initial=a b "], Some((10, "a b ")))]
    #[case(vec!["--initial=a b c"], Some((10, "a b c")))]
    #[case(vec!["--initial=", "x"], None)]
    #[case(vec!["--initial=a", "x"], None)]
    #[case(vec!["-i="], Some((3, "")))]
    #[case(vec!["-i=a"], Some((3, "a")))]
    #[case(vec!["-i=a b "], Some((3, "a b ")))]
    #[case(vec!["-i=a b c"], Some((3, "a b c")))]
    #[case(vec!["-i=", "x"], None)]
    #[case(vec!["-i=a", "x"], None)]
    fn option_equals_delimiter(#[case] tokens: Vec<&str>, #[case] expected: Option<(usize, &str)>) {
        // Setup
        let options = HashSet::from([OptionConfig::new(
            "initial".to_string(),
            Some('i'),
            Bound::Lower(0),
        )]);
        let mut tp = TokenMatcher::new(options, VecDeque::default()).unwrap();
        let mut result = Ok(());

        // Execute
        for token in &tokens {
            result.unwrap();
            result = tp.feed(token);
        }

        // Verify
        match expected {
            Some((offset, value)) => {
                result.unwrap();
                assert_eq!(
                    tp.close().unwrap().values,
                    vec![MatchTokens {
                        name: "initial".to_string(),
                        values: vec![(offset, value.to_string())],
                    }]
                );
            }
            None => {
                assert_eq!(result.unwrap_err(), MatchError::ArgumentsExhausted);
            }
        }
    }

    #[rstest]
    #[case(vec!["--super-verbose"], 0, vec![])]
    #[case(vec!["--super-verbose="], 1, vec![(16, "")])]
    #[case(vec!["--super-verbose=a"], 1, vec![(16, "a")])]
    #[case(vec!["--super-verbose", "a"], 1, vec![(15, "a")])]
    #[case(vec!["--super-verbose", "a", "b"], 2, vec![(15, "a"), (16, "b")])]
    #[case(vec!["-s"], 0, vec![])]
    #[case(vec!["-s="], 1, vec![(3, "")])]
    #[case(vec!["-s=a"], 1, vec![(3, "a")])]
    #[case(vec!["-s", "a"], 1, vec![(2, "a")])]
    #[case(vec!["-s", "a", "b"], 2, vec![(2, "a"), (3, "b")])]
    fn option_dash_name(
        #[case] tokens: Vec<&str>,
        #[case] limit: u8,
        #[case] expected: Vec<(usize, &str)>,
    ) {
        // Setup
        let options = HashSet::from([OptionConfig::new(
            "super-verbose".to_string(),
            Some('s'),
            Bound::Range(0, limit),
        )]);
        let mut tp = TokenMatcher::new(options, VecDeque::default()).unwrap();

        // Execute
        for token in &tokens {
            tp.feed(token).unwrap();
        }

        // Verify
        assert_eq!(
            tp.close().unwrap().values,
            vec![MatchTokens {
                name: "super-verbose".to_string(),
                values: expected.iter().map(|(i, e)| (*i, e.to_string())).collect(),
            }]
        );
    }

    #[test]
    fn option_short_too_few() {
        let options = HashSet::from([
            OptionConfig::new("verbose".to_string(), Some('v'), Bound::Lower(1)),
            OptionConfig::new("flag".to_string(), Some('f'), Bound::Lower(0)),
        ]);
        let mut tp = TokenMatcher::new(options, VecDeque::default()).unwrap();

        // Execute & verify
        assert_eq!(
            tp.feed("-vf").unwrap_err(),
            MatchError::Undercomplete("verbose".to_string())
        );
    }

    #[test]
    fn option_short_duplicate() {
        let options = HashSet::from([
            OptionConfig::new("verbose".to_string(), Some('v'), Bound::Lower(0)),
            OptionConfig::new("item".to_string(), Some('v'), Bound::Lower(0)),
        ]);
        let error = TokenMatcher::new(options, VecDeque::default()).unwrap_err();
        assert_eq!(error, TokenMatcherError::DuplicateShortOption('v'));
    }

    #[test]
    fn option_short_unmatched() {
        let options = HashSet::from([OptionConfig::new(
            "verbose".to_string(),
            Some('v'),
            Bound::Lower(0),
        )]);
        let mut tp = TokenMatcher::new(options, VecDeque::default()).unwrap();

        assert_eq!(
            tp.feed("-f").unwrap_err(),
            MatchError::InvalidShortOption('f')
        );
    }

    #[test]
    fn option_short_repeat() {
        let options = HashSet::from([OptionConfig::new(
            "verbose".to_string(),
            Some('v'),
            Bound::Lower(0),
        )]);
        let mut tp = TokenMatcher::new(options, VecDeque::default()).unwrap();

        tp.feed("-v").unwrap();
        assert_eq!(
            tp.feed("-v").unwrap_err(),
            MatchError::InvalidShortOption('v')
        );
    }

    #[rstest]
    #[case(Bound::Lower(0), 0, true)]
    #[case(Bound::Lower(0), 1, true)]
    #[case(Bound::Lower(1), 0, false)]
    #[case(Bound::Lower(1), 1, true)]
    #[case(Bound::Range(0, 1), 0, true)]
    #[case(Bound::Range(0, 1), 1, true)]
    #[case(Bound::Range(1, 1), 0, false)]
    #[case(Bound::Range(1, 1), 1, true)]
    fn option_can_close(#[case] bound: Bound, #[case] feed: u8, #[case] can_close: bool) {
        // Setup
        let options = HashSet::from([OptionConfig::new("initial".to_string(), None, bound)]);
        let mut tp = TokenMatcher::new(options, VecDeque::default()).unwrap();
        // At first, it can always close - the token matcher only takes in options.
        assert!(tp.can_close());
        let tokens: Vec<String> = (0..feed).map(|i| i.to_string()).collect();

        // Execute
        tp.feed("--initial").unwrap();
        for token in &tokens {
            tp.feed(token).unwrap();
        }

        // Verify
        assert_eq!(tp.can_close(), can_close);
    }

    #[rstest]
    #[case(Bound::Range(0, 0), 0, true)]
    #[case(Bound::Range(0, 0), 1, false)]
    #[case(Bound::Range(0, 1), 0, true)]
    #[case(Bound::Range(0, 1), 1, true)]
    #[case(Bound::Range(0, 1), 2, false)]
    #[case(Bound::Range(1, 1), 0, false)]
    #[case(Bound::Range(1, 1), 1, true)]
    #[case(Bound::Range(1, 1), 2, false)]
    fn argument_range_upper(#[case] bound: Bound, #[case] feed: u8, #[case] expected_ok: bool) {
        // Setup
        let arguments = VecDeque::from([ArgumentConfig::new("item".to_string(), bound)]);
        let mut tp = TokenMatcher::new(HashSet::default(), arguments).unwrap();
        let tokens: Vec<String> = (0..feed).map(|i| i.to_string()).collect();
        let mut feed_error = false;

        // Execute
        for (i, token) in tokens.iter().enumerate() {
            let result = tp.feed(token);

            if !expected_ok && i + 1 == feed.into() {
                if let Err(MatchError::ArgumentsExhausted) = result {
                    feed_error = true;
                }
            } else {
                result.unwrap();
            }
        }

        // Verify
        if expected_ok {
            assert_eq!(
                tp.close().unwrap().values,
                vec![MatchTokens {
                    name: "item".to_string(),
                    values: tokens.into_iter().enumerate().collect(),
                }]
            );
        } else if !feed_error {
            let (offset, error, matches) = tp.close().unwrap_err();
            assert_eq!(offset, feed as usize);

            match bound {
                Bound::Range(n, _) if n > feed => {
                    assert_eq!(error, MatchError::Undercomplete("item".to_string()));
                }
                Bound::Range(_, n) if n < feed => {
                    assert_eq!(error, MatchError::Overcomplete("item".to_string()));
                }
                _ => unreachable!("invalid test scenario"),
            };

            assert_eq!(matches.values, vec![]);
        }
    }

    #[rstest]
    #[case(0, false)]
    #[case(1, true)]
    #[case(2, true)]
    fn argument_range_feed_lower(#[case] feed: u8, #[case] expected_ok: bool) {
        // Setup
        let arguments =
            VecDeque::from([ArgumentConfig::new("item".to_string(), Bound::Range(1, 3))]);
        let mut tp = TokenMatcher::new(HashSet::default(), arguments).unwrap();
        let tokens: Vec<String> = (0..feed).map(|i| i.to_string()).collect();

        // Execute
        for token in tokens.iter() {
            tp.feed(token).unwrap();
        }

        // Verify
        if expected_ok {
            assert_eq!(
                tp.close().unwrap().values,
                vec![MatchTokens {
                    name: "item".to_string(),
                    values: tokens.into_iter().enumerate().collect(),
                }]
            );
        } else {
            let (offset, error, matches) = tp.close().unwrap_err();
            assert_eq!(offset, 0);
            assert_eq!(error, MatchError::Undercomplete("item".to_string()));
            assert_eq!(matches.values, vec![]);
        }
    }

    #[rstest]
    #[case(0, false)]
    #[case(1, true)]
    #[case(2, true)]
    #[case(10, true)]
    #[case(100, true)]
    fn argument_unlimited(#[case] feed: u8, #[case] expected_ok: bool) {
        // Setup
        let arguments = VecDeque::from([ArgumentConfig::new("item".to_string(), Bound::Lower(1))]);
        let mut tp = TokenMatcher::new(HashSet::default(), arguments).unwrap();
        let tokens: Vec<String> = (0..feed).map(|i| i.to_string()).collect();

        // Execute
        for token in &tokens {
            tp.feed(token).unwrap();
        }

        // Verify
        if expected_ok {
            let mut offset = 0;
            assert_eq!(
                tp.close().unwrap().values,
                vec![MatchTokens {
                    name: "item".to_string(),
                    values: tokens
                        .into_iter()
                        .map(|t| {
                            let length = t.len();
                            let out = (offset, t);
                            offset += length;
                            out
                        })
                        .collect(),
                }]
            );
        } else {
            let (offset, error, matches) = tp.close().unwrap_err();
            assert_eq!(offset, 0);
            assert_eq!(error, MatchError::Undercomplete("item".to_string()));
            assert_eq!(matches.values, vec![]);
        }
    }

    #[test]
    fn arguments_multiple() {
        // Setup
        let arguments = VecDeque::from([
            ArgumentConfig::new("arg1".to_string(), Bound::Range(1, 2)),
            ArgumentConfig::new("arg2".to_string(), Bound::Lower(1)),
        ]);
        let mut tp = TokenMatcher::new(HashSet::default(), arguments).unwrap();

        // Execute
        tp.feed("a").unwrap();
        tp.feed("b").unwrap();
        tp.feed("c").unwrap();

        // Verify
        assert_eq!(
            tp.close().unwrap().values,
            vec![
                MatchTokens {
                    name: "arg1".to_string(),
                    values: vec![(0, "a".to_string()), (1, "b".to_string())],
                },
                MatchTokens {
                    name: "arg2".to_string(),
                    values: vec![(2, "c".to_string())],
                },
            ]
        );
    }

    #[test]
    fn arguments_with_preceeding_unlimited() {
        let arguments = VecDeque::from([
            ArgumentConfig::new("arg1".to_string(), Bound::Lower(1)),
            ArgumentConfig::new("arg2".to_string(), Bound::Range(1, 1)),
        ]);
        let mut tp = TokenMatcher::new(HashSet::default(), arguments).unwrap();

        tp.feed("value1").unwrap();
        tp.feed("value2").unwrap();

        let (offset, error, matches) = tp.close().unwrap_err();
        assert_eq!(offset, 12);
        assert_eq!(error, MatchError::Undercomplete("arg2".to_string()));
        assert_eq!(
            matches.values,
            vec![MatchTokens {
                name: "arg1".to_string(),
                values: vec![(0, "value1".to_string()), (6, "value2".to_string())],
            }]
        );
    }

    #[rstest]
    #[case(Bound::Lower(0), 0, true)]
    #[case(Bound::Lower(0), 1, true)]
    #[case(Bound::Lower(1), 0, false)]
    #[case(Bound::Lower(1), 1, true)]
    #[case(Bound::Range(0, 1), 0, true)]
    #[case(Bound::Range(0, 1), 1, true)]
    #[case(Bound::Range(1, 1), 0, false)]
    #[case(Bound::Range(1, 1), 1, true)]
    fn argument_can_close(#[case] bound: Bound, #[case] feed: u8, #[case] can_close: bool) {
        // Setup
        let arguments = VecDeque::from([ArgumentConfig::new("item".to_string(), bound)]);
        let mut tp = TokenMatcher::new(HashSet::default(), arguments).unwrap();
        let tokens: Vec<String> = (0..feed).map(|i| i.to_string()).collect();

        // Execute
        for token in &tokens {
            tp.feed(token).unwrap();
        }

        // Verify
        assert_eq!(tp.can_close(), can_close);
    }

    #[rstest]
    #[case(vec!["x", "y", "z"], 0, 1, 2, None)]
    #[case(vec!["--verbose", "x", "y", "z"], 9, 10, 11, Some(0))]
    #[case(vec!["x", "y", "--verbose", "z"], 0, 1, 11, Some(1))]
    #[case(vec!["x", "y", "z", "--verbose"], 0, 1, 2, Some(2))]
    fn arguments_option_zero_mix(
        #[case] tokens: Vec<&str>,
        #[case] x_offset: usize,
        #[case] y_offset: usize,
        #[case] z_offset: usize,
        #[case] a_index: Option<usize>,
    ) {
        let options = HashSet::from([OptionConfig::new(
            "verbose".to_string(),
            None,
            Bound::Range(0, 0),
        )]);
        let arguments = VecDeque::from([
            ArgumentConfig::new("arg1".to_string(), Bound::Range(1, 2)),
            ArgumentConfig::new("arg2".to_string(), Bound::Lower(1)),
        ]);
        let mut tp = TokenMatcher::new(options, arguments).unwrap();

        for token in &tokens {
            tp.feed(token).unwrap();
        }

        let mut expected = vec![
            MatchTokens {
                name: "arg1".to_string(),
                values: vec![(x_offset, "x".to_string()), (y_offset, "y".to_string())],
            },
            MatchTokens {
                name: "arg2".to_string(),
                values: vec![(z_offset, "z".to_string())],
            },
        ];
        if let Some(index) = a_index {
            expected.insert(
                index,
                MatchTokens {
                    name: "verbose".to_string(),
                    values: Vec::default(),
                },
            );
        }

        assert_eq!(tp.close().unwrap().values, expected);
    }

    #[rstest]
    #[case(vec!["x", "y", "z"], 0, 1, 2, None)]
    #[case(vec!["--initial", "a", "x", "y", "z"], 10, 11, 12, Some((0, 9)))]
    #[case(vec!["x", "y", "--initial", "a", "z"], 0, 1, 12, Some((1, 11)))]
    #[case(vec!["x", "y", "z", "--initial", "a"], 0, 1, 2, Some((2, 12)))]
    fn arguments_option_one_mix(
        #[case] tokens: Vec<&str>,
        #[case] x_offset: usize,
        #[case] y_offset: usize,
        #[case] z_offset: usize,
        #[case] a_index_offset: Option<(usize, usize)>,
    ) {
        let options = HashSet::from([OptionConfig::new(
            "initial".to_string(),
            None,
            Bound::Range(1, 1),
        )]);
        let arguments = VecDeque::from([
            ArgumentConfig::new("arg1".to_string(), Bound::Range(1, 2)),
            ArgumentConfig::new("arg2".to_string(), Bound::Lower(1)),
        ]);
        let mut tp = TokenMatcher::new(options, arguments).unwrap();

        for token in &tokens {
            tp.feed(token).unwrap();
        }

        let mut expected = vec![
            MatchTokens {
                name: "arg1".to_string(),
                values: vec![(x_offset, "x".to_string()), (y_offset, "y".to_string())],
            },
            MatchTokens {
                name: "arg2".to_string(),
                values: vec![(z_offset, "z".to_string())],
            },
        ];
        if let Some((index, offset)) = a_index_offset {
            expected.insert(
                index,
                MatchTokens {
                    name: "initial".to_string(),
                    values: vec![(offset, "a".to_string())],
                },
            );
        }

        assert_eq!(tp.close().unwrap().values, expected);
    }

    #[test]
    fn arguments_option_breaker() {
        let options = HashSet::from([OptionConfig::new(
            "verbose".to_string(),
            None,
            Bound::Range(0, 0),
        )]);
        let arguments = VecDeque::from([
            ArgumentConfig::new("arg1".to_string(), Bound::Range(1, 2)),
            ArgumentConfig::new("arg2".to_string(), Bound::Lower(1)),
        ]);
        let mut tp = TokenMatcher::new(options, arguments).unwrap();

        for token in vec!["x", "--verbose", "z"] {
            tp.feed(token).unwrap();
        }

        assert_eq!(
            tp.close().unwrap().values,
            vec![
                MatchTokens {
                    name: "arg1".to_string(),
                    values: vec![(0, "x".to_string())],
                },
                MatchTokens {
                    name: "verbose".to_string(),
                    values: Vec::default(),
                },
                MatchTokens {
                    name: "arg2".to_string(),
                    values: vec![(10, "z".to_string())],
                },
            ]
        );
    }
}
