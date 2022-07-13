use std::collections::{HashMap, HashSet, VecDeque};

use crate::tokens::*;

pub(crate) struct TokenMatcher {
    option_bounds: HashMap<String, Bound>,
    short_options: HashMap<char, String>,
    arguments: VecDeque<ArgumentConfig>,
    matches: HashSet<MatchTokens>,
    buffer: Option<MatchBuffer>,
}

impl TokenMatcher {
    pub(crate) fn new(options: HashSet<OptionConfig>, arguments: VecDeque<ArgumentConfig>) -> Self {
        let mut option_bounds = HashMap::default();
        let mut short_options = HashMap::default();

        for option_config in options.into_iter() {
            option_bounds.insert(option_config.name(), option_config.bound());

            if let Some(short) = option_config.short() {
                // TODO: return as error, or something.
                assert!(short_options
                    .insert(short.clone(), option_config.name())
                    .is_none());
            }
        }

        Self {
            option_bounds,
            short_options,
            arguments,
            matches: HashSet::default(),
            buffer: None,
        }
    }

    pub(crate) fn feed(&mut self, token: &str) -> Result<(), ()> {
        // Find a 'long' flag, such as:
        //  --initial
        //  --initial ..
        //  --initial=..
        if let Some(token) = token.strip_prefix("--") {
            return self.match_option(split_equals_delimiter(token));
        }

        // Find 'short' flag(s), such as (both -i and -v are example short flags):
        //  -i
        //  -i..
        //  -i ..
        //  -i=..
        //  -iv..
        //  -iv ..
        //  -iv=..
        if let Some(token) = token.strip_prefix("-") {
            return self.match_option_short(split_equals_delimiter(token));
        }

        // Match against an argument.
        return self.match_argument(token);
    }

    fn match_argument(&mut self, token: &str) -> Result<(), ()> {
        let mut match_buffer = match self.buffer.take() {
            Some(match_buffer) => {
                if match_buffer.is_open() {
                    match_buffer
                } else {
                    // Flip to the next argument
                    let match_tokens = match_buffer.close()?;
                    self.matches.insert(match_tokens);
                    self.next_argument()?
                }
            }
            None => {
                // Flip to the next argument.
                self.next_argument()?
            }
        };

        match_buffer.push(token.to_string()).unwrap();

        if let Some(_) = self.buffer.replace(match_buffer) {
            panic!("internal error - the buffer is expected to be None");
        }

        Ok(())
    }

    fn next_argument(&mut self) -> Result<MatchBuffer, ()> {
        match self.arguments.pop_front() {
            Some(argument_config) => Ok(MatchBuffer::new(
                argument_config.name(),
                argument_config.bound(),
            )),
            None => Err(()),
        }
    }

    fn match_option(&mut self, (left, right): (&str, Option<&str>)) -> Result<(), ()> {
        if let Some(bound) = self.option_bounds.remove(left) {
            let mut match_buffer = MatchBuffer::new(left.to_string(), bound);

            let next_buffer = match right {
                Some(v) => {
                    match_buffer.push(v.to_string())?;
                    // Options using k=v syntax cannot follow up with more values afterwards.
                    let match_tokens = match_buffer.close()?;
                    self.matches.insert(match_tokens);
                    None
                }
                None => Some(match_buffer),
            };
            self.update_buffer(next_buffer)
        } else {
            Err(())
        }
    }

    fn match_option_short(&mut self, (left, right): (&str, Option<&str>)) -> Result<(), ()> {
        for (index, single) in left.chars().enumerate() {
            if let Some(name) = self.short_options.get(&single) {
                if let Some(bound) = self.option_bounds.remove(name) {
                    // If this is the final character from the short option token (the variable 'left').
                    if index + 1 == left.len() {
                        // Only the final option may accept values.
                        let mut match_buffer = MatchBuffer::new(name.to_string(), bound);

                        match right {
                            // If an equals delimited value was specified, use it.
                            Some(value) => {
                                match_buffer.push(value.to_string())?;
                                // Options using k=v syntax cannot follow up with more values afterwards.
                                let match_tokens = match_buffer.close()?;
                                self.matches.insert(match_tokens);
                            }
                            // If no equals delimited value was specified, allow the values to be fed as subsequent tokens.
                            None => {
                                self.update_buffer(Some(match_buffer))?;
                            }
                        };
                    } else {
                        // All characters in the head of the short option token (the variable 'left') must allow no values.
                        let match_tokens = MatchBuffer::new(name.to_string(), bound).close()?;
                        self.matches.insert(match_tokens);
                    }
                } else {
                    return Err(());
                }

                self.short_options.remove(&single).unwrap();
            } else {
                return Err(());
            }
        }

        Ok(())
    }

    fn update_buffer(&mut self, next_buffer: Option<MatchBuffer>) -> Result<(), ()> {
        let previous_buffer = std::mem::replace(&mut self.buffer, next_buffer);

        if let Some(match_buffer) = previous_buffer {
            let match_tokens = match_buffer.close()?;
            self.matches.insert(match_tokens);
        }

        Ok(())
    }

    pub(crate) fn matches(mut self) -> Result<HashSet<MatchTokens>, ()> {
        if let Some(match_buffer) = self.buffer {
            let match_tokens = match_buffer.close()?;
            self.matches.insert(match_tokens);
        }

        if !self.arguments.is_empty() {
            Err(())
        } else {
            Ok(self.matches)
        }
    }
}

fn split_equals_delimiter(token: &str) -> (&str, Option<&str>) {
    match token.split_once("=") {
        Some((n, v)) => (n, Some(v)),
        None => (token, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(Bound::Range(0, 0), 0, true)]
    #[case(Bound::Range(0, 0), 1, false)]
    #[case(Bound::Range(0, 1), 0, true)]
    #[case(Bound::Range(0, 1), 1, true)]
    #[case(Bound::Range(0, 1), 2, false)]
    fn option_range_upper(#[case] bound: Bound, #[case] feed: u8, #[case] expected_ok: bool) {
        // Setup
        let options = HashSet::from([OptionConfig::new("initial".to_string(), None, bound)]);
        let mut tp = TokenMatcher::new(options, VecDeque::default());
        let tokens: Vec<String> = (0..feed).map(|i| i.to_string()).collect();

        // Execute
        tp.feed("--initial").unwrap();
        for (i, token) in tokens.iter().enumerate() {
            let result = tp.feed(token);

            if !expected_ok && i + 1 == feed.into() {
                assert_eq!(result, Err(()));
            } else {
                result.unwrap();
            }
        }

        // Verify
        if expected_ok {
            assert_eq!(
                tp.matches().unwrap(),
                HashSet::from([MatchTokens {
                    name: "initial".to_string(),
                    values: tokens,
                }])
            );
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
        let mut tp = TokenMatcher::new(options, VecDeque::default());
        let tokens: Vec<String> = (0..feed).map(|i| i.to_string()).collect();

        // Execute
        tp.feed("--initial").unwrap();
        for token in &tokens {
            tp.feed(token).unwrap();
        }

        // Verify
        if expected_ok {
            assert_eq!(
                tp.matches().unwrap(),
                HashSet::from([MatchTokens {
                    name: "initial".to_string(),
                    values: tokens,
                }])
            );
        } else {
            assert_eq!(tp.matches(), Err(()));
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
        let mut tp = TokenMatcher::new(options, VecDeque::default());
        let tokens: Vec<String> = (0..feed).map(|i| i.to_string()).collect();

        // Execute
        tp.feed("--initial").unwrap();
        for token in &tokens {
            tp.feed(token).unwrap();
        }

        // Verify
        assert_eq!(
            tp.matches().unwrap(),
            HashSet::from([MatchTokens {
                name: "initial".to_string(),
                values: tokens,
            }])
        );
    }

    #[test]
    fn option_unmatched() {
        let options = HashSet::from([OptionConfig::new(
            "verbose".to_string(),
            None,
            Bound::Lower(0),
        )]);
        let mut tp = TokenMatcher::new(options, VecDeque::default());

        assert_eq!(tp.feed("--moot"), Err(()));
    }

    #[test]
    fn option_repeat() {
        let options = HashSet::from([OptionConfig::new(
            "verbose".to_string(),
            None,
            Bound::Lower(0),
        )]);
        let mut tp = TokenMatcher::new(options, VecDeque::default());

        tp.feed("--verbose").unwrap();
        assert_eq!(tp.feed("--verbose"), Err(()));
    }

    #[rstest]
    #[case(vec!["-v"], true, None)]
    #[case(vec!["-f"], false, Some(vec![]))]
    #[case(vec!["-f", "a"], false, Some(vec!["a"]))]
    #[case(vec!["-f", "a", "bc"], false, Some(vec!["a", "bc"]))]
    #[case(vec!["-vf"], true, Some(vec![]))]
    #[case(vec!["-vf", "a"], true, Some(vec!["a"]))]
    #[case(vec!["-vf", "a", "bc"], true, Some(vec!["a", "bc"]))]
    fn option_short(
        #[case] tokens: Vec<&str>,
        #[case] expected_verbose: bool,
        #[case] expected_flags: Option<Vec<&str>>,
    ) {
        // Setup
        let options = HashSet::from([
            OptionConfig::new("verbose".to_string(), Some('v'), Bound::Range(0, 0)),
            OptionConfig::new("flag".to_string(), Some('f'), Bound::Lower(0)),
        ]);
        let mut tp = TokenMatcher::new(options, VecDeque::default());

        // Execute
        for token in tokens.iter() {
            tp.feed(token).unwrap();
        }
        let matches = tp.matches().unwrap();

        // Verify
        if expected_verbose {
            assert!(matches.contains(&MatchTokens {
                name: "verbose".to_string(),
                values: Vec::default(),
            }));
        }

        match expected_flags {
            None => {
                assert_eq!(matches.len(), if expected_verbose { 1 } else { 0 });
            }
            Some(expected) => {
                assert_eq!(matches.len(), if expected_verbose { 2 } else { 1 });
                assert!(matches.contains(&MatchTokens {
                    name: "flag".to_string(),
                    values: expected.iter().map(|e| e.to_string()).collect(),
                }));
            }
        };
    }

    #[rstest]
    #[case(vec!["--initial="], Some(""))]
    #[case(vec!["--initial=a"], Some("a"))]
    #[case(vec!["--initial=a b "], Some("a b "))]
    #[case(vec!["--initial=a b c"], Some("a b c"))]
    #[case(vec!["--initial=", "x"], None)]
    #[case(vec!["--initial=a", "x"], None)]
    #[case(vec!["-i="], Some(""))]
    #[case(vec!["-i=a"], Some("a"))]
    #[case(vec!["-i=a b "], Some("a b "))]
    #[case(vec!["-i=a b c"], Some("a b c"))]
    #[case(vec!["-i=", "x"], None)]
    #[case(vec!["-i=a", "x"], None)]
    fn option_equals_delimiter(#[case] tokens: Vec<&str>, #[case] expected: Option<&str>) {
        // Setup
        let options = HashSet::from([OptionConfig::new(
            "initial".to_string(),
            Some('i'),
            Bound::Lower(0),
        )]);
        let mut tp = TokenMatcher::new(options, VecDeque::default());
        let mut result = Ok(());

        // Execute
        for token in &tokens {
            result.unwrap();
            result = tp.feed(token);
        }

        // Verify
        match expected {
            Some(value) => {
                result.unwrap();
                assert_eq!(
                    tp.matches().unwrap(),
                    HashSet::from([MatchTokens {
                        name: "initial".to_string(),
                        values: vec![value.to_string()],
                    }])
                );
            }
            None => {
                assert_eq!(result, Err(()));
            }
        }
    }

    #[rstest]
    #[case(vec!["--super-verbose"], 0, vec![])]
    #[case(vec!["--super-verbose="], 1, vec![""])]
    #[case(vec!["--super-verbose=a"], 1, vec!["a"])]
    #[case(vec!["--super-verbose", "a"], 1, vec!["a"])]
    #[case(vec!["--super-verbose", "a", "b"], 2, vec!["a", "b"])]
    #[case(vec!["-s"], 0, vec![])]
    #[case(vec!["-s="], 1, vec![""])]
    #[case(vec!["-s=a"], 1, vec!["a"])]
    #[case(vec!["-s", "a"], 1, vec!["a"])]
    #[case(vec!["-s", "a", "b"], 2, vec!["a", "b"])]
    fn option_dash_name(#[case] tokens: Vec<&str>, #[case] limit: u8, #[case] expected: Vec<&str>) {
        // Setup
        let options = HashSet::from([OptionConfig::new(
            "super-verbose".to_string(),
            Some('s'),
            Bound::Range(0, limit),
        )]);
        let mut tp = TokenMatcher::new(options, VecDeque::default());

        // Execute
        for token in &tokens {
            tp.feed(token).unwrap();
        }

        // Verify
        assert_eq!(
            tp.matches().unwrap(),
            HashSet::from([MatchTokens {
                name: "super-verbose".to_string(),
                values: expected.iter().map(|e| e.to_string()).collect(),
            }])
        );
    }

    #[test]
    fn option_short_too_few() {
        let options = HashSet::from([
            OptionConfig::new("verbose".to_string(), Some('v'), Bound::Lower(1)),
            OptionConfig::new("flag".to_string(), Some('f'), Bound::Lower(0)),
        ]);
        let mut tp = TokenMatcher::new(options, VecDeque::default());

        // Execute & verify
        assert_eq!(tp.feed("-vf"), Err(()));
    }

    #[test]
    #[should_panic]
    fn option_short_duplicate() {
        let options = HashSet::from([
            OptionConfig::new("verbose".to_string(), Some('v'), Bound::Lower(0)),
            OptionConfig::new("item".to_string(), Some('v'), Bound::Lower(0)),
        ]);
        TokenMatcher::new(options, VecDeque::default());
    }

    #[test]
    fn option_short_unmatched() {
        let options = HashSet::from([OptionConfig::new(
            "verbose".to_string(),
            Some('v'),
            Bound::Lower(0),
        )]);
        let mut tp = TokenMatcher::new(options, VecDeque::default());

        assert_eq!(tp.feed("-f"), Err(()));
    }

    #[test]
    fn option_short_repeat() {
        let options = HashSet::from([OptionConfig::new(
            "verbose".to_string(),
            Some('v'),
            Bound::Lower(0),
        )]);
        let mut tp = TokenMatcher::new(options, VecDeque::default());

        tp.feed("-v").unwrap();
        assert_eq!(tp.feed("-v"), Err(()));
    }

    #[rstest]
    #[case(Bound::Range(1, 1), 0, false)]
    #[case(Bound::Range(1, 1), 1, true)]
    #[case(Bound::Range(1, 1), 2, false)]
    #[case(Bound::Range(1, 2), 0, false)]
    #[case(Bound::Range(1, 2), 1, true)]
    #[case(Bound::Range(1, 2), 2, true)]
    #[case(Bound::Range(1, 2), 3, false)]
    fn argument_range_upper(#[case] bound: Bound, #[case] feed: u8, #[case] expected_ok: bool) {
        // Setup
        let arguments = VecDeque::from([ArgumentConfig::new("item".to_string(), bound).unwrap()]);
        let mut tp = TokenMatcher::new(HashSet::default(), arguments);
        let tokens: Vec<String> = (0..feed).map(|i| i.to_string()).collect();

        // Execute
        for (i, token) in tokens.iter().enumerate() {
            let result = tp.feed(token);

            if !expected_ok && i + 1 == feed.into() {
                assert_eq!(result, Err(()));
            } else {
                result.unwrap();
            }
        }

        // Verify
        if expected_ok {
            assert_eq!(
                tp.matches().unwrap(),
                HashSet::from([MatchTokens {
                    name: "item".to_string(),
                    values: tokens,
                }])
            );
        }
    }

    #[rstest]
    #[case(0, false)]
    #[case(1, true)]
    #[case(2, true)]
    fn argument_range_feed_lower(#[case] feed: u8, #[case] expected_ok: bool) {
        // Setup
        let arguments =
            VecDeque::from([ArgumentConfig::new("item".to_string(), Bound::Range(1, 3)).unwrap()]);
        let mut tp = TokenMatcher::new(HashSet::default(), arguments);
        let tokens: Vec<String> = (0..feed).map(|i| i.to_string()).collect();

        // Execute
        for token in tokens.iter() {
            tp.feed(token).unwrap();
        }

        // Verify
        if expected_ok {
            assert_eq!(
                tp.matches().unwrap(),
                HashSet::from([MatchTokens {
                    name: "item".to_string(),
                    values: tokens,
                }])
            );
        } else {
            assert_eq!(tp.matches(), Err(()));
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
        let arguments =
            VecDeque::from([ArgumentConfig::new("item".to_string(), Bound::Lower(1)).unwrap()]);
        let mut tp = TokenMatcher::new(HashSet::default(), arguments);
        let tokens: Vec<String> = (0..feed).map(|i| i.to_string()).collect();

        // Execute
        for token in &tokens {
            tp.feed(token).unwrap();
        }

        // Verify
        if expected_ok {
            assert_eq!(
                tp.matches().unwrap(),
                HashSet::from([MatchTokens {
                    name: "item".to_string(),
                    values: tokens,
                }])
            );
        } else {
            assert_eq!(tp.matches(), Err(()));
        }
    }

    #[test]
    fn arguments_multiple() {
        // Setup
        let arguments = VecDeque::from([
            ArgumentConfig::new("arg1".to_string(), Bound::Range(1, 2)).unwrap(),
            ArgumentConfig::new("arg2".to_string(), Bound::Lower(1)).unwrap(),
        ]);
        let mut tp = TokenMatcher::new(HashSet::default(), arguments);

        // Execute
        tp.feed("a").unwrap();
        tp.feed("b").unwrap();
        tp.feed("c").unwrap();

        // Verify
        assert_eq!(
            tp.matches().unwrap(),
            HashSet::from([
                MatchTokens {
                    name: "arg1".to_string(),
                    values: vec!["a".to_string(), "b".to_string()],
                },
                MatchTokens {
                    name: "arg2".to_string(),
                    values: vec!["c".to_string()],
                },
            ])
        );
    }

    #[test]
    fn arguments_with_preceeding_unlimited() {
        let arguments = VecDeque::from([
            ArgumentConfig::new("arg1".to_string(), Bound::Lower(1)).unwrap(),
            ArgumentConfig::new("arg2".to_string(), Bound::Range(1, 1)).unwrap(),
        ]);
        let mut tp = TokenMatcher::new(HashSet::default(), arguments);

        tp.feed("value1").unwrap();
        tp.feed("value2").unwrap();

        assert_eq!(tp.matches(), Err(()));
    }

    #[rstest]
    #[case(vec!["x", "y", "z"])]
    #[case(vec!["--verbose", "x", "y", "z"])]
    #[case(vec!["x", "y", "--verbose", "z"])]
    #[case(vec!["x", "y", "z", "--verbose"])]
    fn arguments_option_zero_mix(#[case] tokens: Vec<&str>) {
        let options = HashSet::from([OptionConfig::new(
            "verbose".to_string(),
            None,
            Bound::Range(0, 0),
        )]);
        let arguments = VecDeque::from([
            ArgumentConfig::new("arg1".to_string(), Bound::Range(1, 2)).unwrap(),
            ArgumentConfig::new("arg2".to_string(), Bound::Lower(1)).unwrap(),
        ]);
        let mut tp = TokenMatcher::new(options, arguments);

        for token in &tokens {
            tp.feed(token).unwrap();
        }

        let mut expected = HashSet::from([
            MatchTokens {
                name: "arg1".to_string(),
                values: vec!["x".to_string(), "y".to_string()],
            },
            MatchTokens {
                name: "arg2".to_string(),
                values: vec!["z".to_string()],
            },
        ]);
        if tokens.len() > 3 {
            expected.insert(MatchTokens {
                name: "verbose".to_string(),
                values: Vec::default(),
            });
        }

        assert_eq!(tp.matches().unwrap(), expected);
    }

    #[rstest]
    #[case(vec!["x", "y", "z"])]
    #[case(vec!["--initial", "a", "x", "y", "z"])]
    #[case(vec!["x", "y", "--initial", "a", "z"])]
    #[case(vec!["x", "y", "z", "--initial", "a"])]
    fn arguments_option_one_mix(#[case] tokens: Vec<&str>) {
        let options = HashSet::from([OptionConfig::new(
            "initial".to_string(),
            None,
            Bound::Range(1, 1),
        )]);
        let arguments = VecDeque::from([
            ArgumentConfig::new("arg1".to_string(), Bound::Range(1, 2)).unwrap(),
            ArgumentConfig::new("arg2".to_string(), Bound::Lower(1)).unwrap(),
        ]);
        let mut tp = TokenMatcher::new(options, arguments);

        for token in &tokens {
            tp.feed(token).unwrap();
        }

        let mut expected = HashSet::from([
            MatchTokens {
                name: "arg1".to_string(),
                values: vec!["x".to_string(), "y".to_string()],
            },
            MatchTokens {
                name: "arg2".to_string(),
                values: vec!["z".to_string()],
            },
        ]);
        if tokens.len() > 3 {
            expected.insert(MatchTokens {
                name: "initial".to_string(),
                values: vec!["a".to_string()],
            });
        }

        assert_eq!(tp.matches().unwrap(), expected);
    }

    #[test]
    fn arguments_option_breaker() {
        let options = HashSet::from([OptionConfig::new(
            "verbose".to_string(),
            None,
            Bound::Range(0, 0),
        )]);
        let arguments = VecDeque::from([
            ArgumentConfig::new("arg1".to_string(), Bound::Range(1, 2)).unwrap(),
            ArgumentConfig::new("arg2".to_string(), Bound::Lower(1)).unwrap(),
        ]);
        let mut tp = TokenMatcher::new(options, arguments);

        for token in vec!["x", "--verbose", "z"] {
            tp.feed(token).unwrap();
        }

        assert_eq!(
            tp.matches().unwrap(),
            HashSet::from([
                MatchTokens {
                    name: "verbose".to_string(),
                    values: Vec::default(),
                },
                MatchTokens {
                    name: "arg1".to_string(),
                    values: vec!["x".to_string()],
                },
                MatchTokens {
                    name: "arg2".to_string(),
                    values: vec!["z".to_string()],
                },
            ])
        );
    }
}
