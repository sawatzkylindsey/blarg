use std::collections::{HashMap, HashSet, VecDeque};
use std::convert::TryFrom;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum Narg {
    Limit(usize),
    Unlimited,
}

#[derive(Debug, Clone)]
pub(crate) struct ArgumentConfig {
    name: String,
    narg: Narg,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct OptionConfig {
    name: String,
    short: Option<char>,
    limit: usize,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub(crate) struct Parameter {
    name: String,
    values: Vec<String>,
}

pub(crate) struct TokenParser {
    option_nargs: HashMap<String, Narg>,
    short_options: HashMap<char, String>,
    arguments: VecDeque<ArgumentConfig>,
    parameters: HashSet<Parameter>,
    buffer: Option<ParameterBuffer>,
}

struct ParameterBuffer {
    name: String,
    narg: Narg,
    values: Vec<String>,
}

impl ParameterBuffer {
    fn new(name: String, narg: Narg) -> Self {
        Self {
            name,
            narg,
            values: Vec::default(),
        }
    }
}

impl ParameterBuffer {
    fn push(&mut self, value: String) -> Result<(), ()> {
        if self.is_open() {
            self.values.push(value);
            Ok(())
        } else {
            Err(())
        }
    }

    fn is_open(&self) -> bool {
        match self.narg {
            Narg::Limit(n) => self.values.len() < n,
            Narg::Unlimited => true,
        }
    }

    fn close(self) -> Result<Parameter, ()> {
        match self.narg {
            Narg::Limit(0) => {
                if !self.values.is_empty() {
                    return Err(());
                }
            }
            Narg::Unlimited | Narg::Limit(_) => {
                if self.values.is_empty() {
                    return Err(());
                }
            }
        }

        Ok(Parameter {
            name: self.name,
            values: self.values,
        })
    }
}

impl TokenParser {
    pub(crate) fn new(options: HashSet<OptionConfig>, arguments: VecDeque<ArgumentConfig>) -> Self {
        let mut option_nargs = HashMap::default();
        let mut short_options = HashMap::default();

        for option_config in options.into_iter() {
            option_nargs.insert(option_config.name.clone(), Narg::Limit(option_config.limit));

            if let Some(short) = option_config.short {
                // TODO: return as error, or something.
                assert!(short_options
                    .insert(short.clone(), option_config.name)
                    .is_none());
            }
        }

        Self {
            option_nargs,
            short_options,
            arguments,
            parameters: HashSet::default(),
            buffer: None,
        }
    }

    pub(crate) fn feed(&mut self, token: &str) -> Result<(), ()> {
        // Find a 'long' flag, such as:
        //  --initial
        //  --initial ..
        //  --initial=..
        if let Some(token) = token.strip_prefix("--") {
            return self.match_option(token);
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
            return self.match_option_short(token);
        }

        // TODO: simplify this take/insert flow
        return self.match_argument(token);
    }

    fn match_argument(&mut self, token: &str) -> Result<(), ()> {
        let mut buffer = match self.buffer.take() {
            Some(buffer) => {
                if buffer.is_open() {
                    buffer
                } else {
                    // flip to the next argument
                    let parameter = buffer.close()?;
                    self.parameters.insert(parameter);

                    match self.arguments.pop_front() {
                        Some(argument_config) => {
                            ParameterBuffer::new(argument_config.name, argument_config.narg)
                        }
                        None => {
                            return Err(());
                        }
                    }
                }
            }
            None => {
                // Flip to the next argument.
                let argument_config = self.arguments.pop_front().unwrap();
                ParameterBuffer::new(argument_config.name, argument_config.narg)
            }
        };

        buffer.push(token.to_string());
        self.buffer.insert(buffer);

        Ok(())
    }

    fn match_option(&mut self, token: &str) -> Result<(), ()> {
        let (name, value) = match token.split_once("=") {
            Some((n, v)) => (n, Some(v)),
            None => (token, None),
        };

        if let Some(narg) = self.option_nargs.remove(name) {
            let mut buffer = ParameterBuffer::new(name.to_string(), narg);

            let next_buffer = match value {
                Some(v) => {
                    buffer.push(v.to_string())?;
                    // Options using k=v syntax cannot follow up with more values afterwards.
                    let parameter = buffer.close()?;
                    self.parameters.insert(parameter);
                    None
                }
                None => Some(buffer),
            };
            self.update_buffer(next_buffer)
        } else {
            Err(())
        }
    }

    fn match_option_short(&mut self, token: &str) -> Result<(), ()> {
        let mut index = 0;

        for single in token.chars() {
            if let Some(name) = self.short_options.get(&single) {
                index += 1;
                let mut exhausted = false;

                if let Some(narg) = self.option_nargs.remove(name) {
                    let next_buffer = match narg {
                        Narg::Limit(0) => {
                            // 0-limit options are the head of the 'short option token'.
                            let parameter = ParameterBuffer::new(name.to_string(), narg).close()?;
                            self.parameters.insert(parameter);
                            None
                        }
                        _ => {
                            let mut buffer = ParameterBuffer::new(name.to_string(), narg);
                            let remaining = &token[index..];

                            if remaining.is_empty() {
                                // Only when the full 'short option token' is finished, then we can treat this as a regular option
                                // (which may accept multiple nargs as upcoming tokens).
                                Some(buffer)
                            } else {
                                // Otherwise, we close the option right now based off what is remaining.

                                if remaining.starts_with("=") {
                                    // Strip out a prefix '='.
                                    buffer.push(remaining[1..].to_string())?;
                                } else {
                                    buffer.push(remaining.to_string())?;
                                }

                                let parameter = buffer.close()?;
                                self.parameters.insert(parameter);
                                exhausted = true;
                                None
                            }
                        }
                    };
                    self.update_buffer(next_buffer)?;
                } else {
                    return Err(());
                }

                self.short_options.remove(&single).unwrap();

                if exhausted {
                    break;
                }
            } else {
                return Err(());
            }
        }

        Ok(())
    }

    fn update_buffer(&mut self, next_buffer: Option<ParameterBuffer>) -> Result<(), ()> {
        let previous_buffer = std::mem::replace(&mut self.buffer, next_buffer);

        if let Some(buffer) = previous_buffer {
            let parameter = buffer.close()?;
            self.parameters.insert(parameter);
        }

        Ok(())
    }

    fn parameters(mut self) -> Result<HashSet<Parameter>, ()> {
        if let Some(buffer) = self.buffer {
            let parameter = buffer.close()?;
            self.parameters.insert(parameter);
        }

        if !self.arguments.is_empty() {
            Err(())
        } else {
            Ok(self.parameters)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(0)]
    #[case(1)]
    #[case(2)]
    fn option_limit(#[case] limit: usize) {
        // Setup
        let options = HashSet::from([OptionConfig {
            name: "initial".to_string(),
            short: None,
            limit,
        }]);
        let mut tp = TokenParser::new(options, VecDeque::default());
        let nargs: Vec<String> = (0..limit).map(|i| i.to_string()).collect();

        // Execute
        tp.feed("--initial").unwrap();
        for token in &nargs {
            tp.feed(token).unwrap();
        }

        // Verify
        assert_eq!(
            tp.parameters().unwrap(),
            HashSet::from([Parameter {
                name: "initial".to_string(),
                values: nargs,
            }])
        );
    }

    #[rstest]
    #[case(vec!["--initial="], "")]
    #[case(vec!["--initial=a"], "a")]
    #[case(vec!["--initial=a b "], "a b ")]
    #[case(vec!["--initial=a b c"], "a b c")]
    fn option_with_equals(#[case] tokens: Vec<&str>, #[case] expected: &str) {
        let options = HashSet::from([OptionConfig {
            name: "initial".to_string(),
            short: None,
            limit: 1,
        }]);
        let mut tp = TokenParser::new(options, VecDeque::default());

        for token in &tokens {
            tp.feed(token).unwrap();
        }

        assert_eq!(
            tp.parameters().unwrap(),
            HashSet::from([Parameter {
                name: "initial".to_string(),
                values: vec![expected.to_string()],
            }])
        );
    }

    #[rstest]
    #[case(0)]
    #[case(1)]
    #[case(2)]
    fn option_limit_too_many(#[case] limit: usize) {
        // Setup
        let options = HashSet::from([OptionConfig {
            name: "initial".to_string(),
            short: None,
            limit,
        }]);
        let mut tp = TokenParser::new(options, VecDeque::default());
        let nargs: Vec<String> = (0..(limit + 1)).map(|i| i.to_string()).collect();
        let mut reached_check = false;

        // Execute & verify
        tp.feed("--initial").unwrap();

        for (i, token) in nargs.iter().enumerate() {
            if i + 1 < nargs.len() {
                tp.feed(token).unwrap();
            } else {
                // The final arg - which is too much.
                let result = tp.feed(token);
                assert!(matches!(result, Err(())));
                reached_check = true;
            }
        }

        assert!(
            reached_check,
            "test never reached the critical assertion (ie: potential false positive)"
        );
    }

    #[rstest]
    #[case(1)]
    #[case(2)]
    fn option_limit_too_few(#[case] limit: usize) {
        // Setup
        let options = HashSet::from([OptionConfig {
            name: "initial".to_string(),
            short: None,
            limit,
        }]);
        let mut tp = TokenParser::new(options, VecDeque::default());
        let nargs: Vec<String> = (0..(limit - 1)).map(|i| i.to_string()).collect();

        // Execute
        tp.feed("--initial").unwrap();
        let mut added = false;

        for (i, token) in nargs.iter().enumerate() {
            tp.feed(token).unwrap();
            added = true;
        }

        // Verify
        if added {
            assert_eq!(
                tp.parameters().unwrap(),
                HashSet::from([Parameter {
                    name: "initial".to_string(),
                    values: nargs,
                }])
            );
        } else {
            assert_eq!(tp.parameters(), Err(()));
        }
    }

    #[test]
    fn option_unmatched() {
        let options = HashSet::from([OptionConfig {
            name: "verbose".to_string(),
            short: None,
            limit: 0,
        }]);
        let mut tp = TokenParser::new(options, VecDeque::default());

        assert_eq!(tp.feed("--moot"), Err(()));
    }

    #[test]
    fn option_repeat() {
        let options = HashSet::from([OptionConfig {
            name: "verbose".to_string(),
            short: None,
            limit: 0,
        }]);
        let mut tp = TokenParser::new(options, VecDeque::default());

        tp.feed("--verbose").unwrap();
        assert_eq!(tp.feed("--verbose"), Err(()));
    }

    #[test]
    fn option_short() {
        let options = HashSet::from([OptionConfig {
            name: "verbose".to_string(),
            short: Some('v'),
            limit: 0,
        }]);
        let mut tp = TokenParser::new(options, VecDeque::default());

        tp.feed("-v").unwrap();

        assert_eq!(
            tp.parameters().unwrap(),
            HashSet::from([Parameter {
                name: "verbose".to_string(),
                values: Vec::default(),
            }])
        );
    }

    #[rstest]
    #[case(vec!["-vfa"], vec!["a"])]
    #[case(vec!["-vfab"], vec!["ab"])]
    #[case(vec!["-vf="], vec![""])]
    #[case(vec!["-vf=ab"], vec!["ab"])]
    #[case(vec!["-vf", "ab"], vec!["ab"])]
    #[case(vec!["-vf", "a", "b"], vec!["a", "b"])]
    fn option_short_combined(#[case] tokens: Vec<&str>, #[case] expected: Vec<&str>) {
        let options = HashSet::from([
            OptionConfig {
                name: "verbose".to_string(),
                short: Some('v'),
                limit: 0,
            },
            OptionConfig {
                name: "flag".to_string(),
                short: Some('f'),
                limit: 2,
            },
        ]);
        let mut tp = TokenParser::new(options, VecDeque::default());

        // Execute
        for token in &tokens {
            tp.feed(token).unwrap();
        }

        // Verify
        assert_eq!(
            tp.parameters().unwrap(),
            HashSet::from([
                Parameter {
                    name: "verbose".to_string(),
                    values: Vec::default(),
                },
                Parameter {
                    name: "flag".to_string(),
                    values: expected.iter().map(|e| e.to_string()).collect(),
                }
            ])
        );
    }

    #[test]
    fn option_short_combined_too_few() {
        let options = HashSet::from([
            OptionConfig {
                name: "verbose".to_string(),
                short: Some('v'),
                limit: 0,
            },
            OptionConfig {
                name: "flag".to_string(),
                short: Some('f'),
                limit: 1,
            },
        ]);
        let mut tp = TokenParser::new(options, VecDeque::default());

        // Execute
        tp.feed("-vf").unwrap();

        // Verify
        assert_eq!(tp.parameters(), Err(()));
    }

    #[test]
    fn option_short_combined_multiple() {
        let options = HashSet::from([
            OptionConfig {
                name: "verbose".to_string(),
                short: Some('v'),
                limit: 0,
            },
            OptionConfig {
                name: "flag".to_string(),
                short: Some('f'),
                limit: 1,
            },
            OptionConfig {
                name: "other".to_string(),
                short: Some('o'),
                limit: 1,
            },
        ]);
        let mut tp = TokenParser::new(options, VecDeque::default());

        // Execute
        tp.feed("-vfo").unwrap();

        // Verify
        assert_eq!(
            tp.parameters().unwrap(),
            HashSet::from([
                Parameter {
                    name: "verbose".to_string(),
                    values: Vec::default(),
                },
                Parameter {
                    name: "flag".to_string(),
                    values: vec!["o".to_string()],
                }
            ])
        );
    }

    #[test]
    #[should_panic]
    fn option_short_duplicate() {
        let options = HashSet::from([
            OptionConfig {
                name: "verbose".to_string(),
                short: Some('v'),
                limit: 0,
            },
            OptionConfig {
                name: "item".to_string(),
                short: Some('v'),
                limit: 1,
            },
        ]);
        TokenParser::new(options, VecDeque::default());
    }

    #[test]
    fn option_short_unmatcedh() {
        let options = HashSet::from([OptionConfig {
            name: "verbose".to_string(),
            short: Some('v'),
            limit: 0,
        }]);
        let mut tp = TokenParser::new(options, VecDeque::default());

        assert_eq!(tp.feed("-f"), Err(()));
    }

    #[test]
    fn option_short_repeat() {
        let options = HashSet::from([OptionConfig {
            name: "verbose".to_string(),
            short: Some('v'),
            limit: 0,
        }]);
        let mut tp = TokenParser::new(options, VecDeque::default());

        tp.feed("-v").unwrap();
        assert_eq!(tp.feed("-v"), Err(()));
    }

    #[test]
    fn argument_limit_zero() {
        // Setup
        let arguments = VecDeque::from([ArgumentConfig {
            name: "item".to_string(),
            narg: Narg::Limit(0),
        }]);
        let mut tp = TokenParser::new(HashSet::default(), arguments);

        // Execute & verify
        assert_eq!(tp.parameters(), Err(()));
    }

    #[rstest]
    #[case(1)]
    #[case(2)]
    fn argument_limit(#[case] limit: usize) {
        // Setup
        let arguments = VecDeque::from([ArgumentConfig {
            name: "item".to_string(),
            narg: Narg::Limit(limit),
        }]);
        let mut tp = TokenParser::new(HashSet::default(), arguments);
        let nargs: Vec<String> = (0..limit).map(|i| i.to_string()).collect();

        // Execute
        for token in &nargs {
            tp.feed(token).unwrap();
        }

        // Verify
        assert_eq!(
            tp.parameters().unwrap(),
            HashSet::from([Parameter {
                name: "item".to_string(),
                values: nargs,
            }])
        );
    }

    #[rstest]
    #[case(0)]
    #[case(1)]
    #[case(2)]
    fn argument_unlimited(#[case] values: usize) {
        // Setup
        let arguments = VecDeque::from([ArgumentConfig {
            name: "item".to_string(),
            narg: Narg::Unlimited,
        }]);
        let mut tp = TokenParser::new(HashSet::default(), arguments);
        let nargs: Vec<String> = (0..values).map(|i| i.to_string()).collect();
        let mut added = false;

        // Execute
        for token in &nargs {
            tp.feed(token).unwrap();
            added = true;
        }

        // Verify
        if added {
            assert_eq!(
                tp.parameters().unwrap(),
                HashSet::from([Parameter {
                    name: "item".to_string(),
                    values: nargs,
                }])
            );
        } else {
            assert_eq!(tp.parameters(), Err(()));
        }
    }

    #[rstest]
    #[case(1)]
    #[case(2)]
    fn argument_limit_too_many(#[case] limit: usize) {
        // Setup
        let arguments = VecDeque::from([ArgumentConfig {
            name: "item".to_string(),
            narg: Narg::Limit(limit),
        }]);
        let mut tp = TokenParser::new(HashSet::default(), arguments);
        let nargs: Vec<String> = (0..(limit + 1)).map(|i| i.to_string()).collect();
        let mut reached_check = false;

        // Execute & verify
        for (i, token) in nargs.iter().enumerate() {
            if i + 1 < nargs.len() {
                tp.feed(token).unwrap();
            } else {
                // The final arg - which is too much.
                let result = tp.feed(token);
                assert!(matches!(result, Err(())));
                reached_check = true;
            }
        }

        assert!(
            reached_check,
            "test never reached the critical assertion (ie: potential false positive)"
        );
    }

    #[rstest]
    #[case(1)]
    #[case(2)]
    fn argument_limit_too_few(#[case] limit: usize) {
        // Setup
        let arguments = VecDeque::from([ArgumentConfig {
            name: "item".to_string(),
            narg: Narg::Limit(limit),
        }]);
        let mut tp = TokenParser::new(HashSet::default(), arguments);
        let nargs: Vec<String> = (0..(limit - 1)).map(|i| i.to_string()).collect();
        let mut added = false;

        // Execute
        for (i, token) in nargs.iter().enumerate() {
            tp.feed(token).unwrap();
            added = true;
        }

        // Verify
        if added {
            assert_eq!(
                tp.parameters().unwrap(),
                HashSet::from([Parameter {
                    name: "item".to_string(),
                    values: nargs,
                }])
            );
        } else {
            assert_eq!(tp.parameters(), Err(()));
        }
    }

    #[rstest]
    #[case(vec!["a"])]
    #[case(vec!["a", "b"])]
    #[case(vec!["a", "b", "c"])]
    fn arguments(#[case] final_tokens: Vec<&str>) {
        let arguments = VecDeque::from([
            ArgumentConfig {
                name: "arg1".to_string(),
                narg: Narg::Limit(1),
            },
            ArgumentConfig {
                name: "arg2".to_string(),
                narg: Narg::Unlimited,
            },
        ]);
        let mut tp = TokenParser::new(HashSet::default(), arguments);

        tp.feed("value").unwrap();
        for token in &final_tokens {
            tp.feed(token).unwrap();
        }

        assert_eq!(
            tp.parameters().unwrap(),
            HashSet::from([
                Parameter {
                    name: "arg1".to_string(),
                    values: vec!["value".to_string()],
                },
                Parameter {
                    name: "arg2".to_string(),
                    values: final_tokens.iter().map(|s| s.to_string()).collect(),
                },
            ])
        );
    }

    #[test]
    fn argument_unlimited_without_value() {
        let arguments = VecDeque::from([ArgumentConfig {
            name: "arg2".to_string(),
            narg: Narg::Unlimited,
        }]);
        let mut tp = TokenParser::new(HashSet::default(), arguments);

        assert_eq!(tp.parameters(), Err(()));
    }

    #[test]
    fn arguments_with_preceeding_unlimited() {
        let arguments = VecDeque::from([
            ArgumentConfig {
                name: "arg1".to_string(),
                narg: Narg::Unlimited,
            },
            ArgumentConfig {
                name: "arg2".to_string(),
                narg: Narg::Limit(1),
            },
        ]);
        let mut tp = TokenParser::new(HashSet::default(), arguments);

        tp.feed("value1").unwrap();
        tp.feed("value2").unwrap();

        assert_eq!(tp.parameters(), Err(()));
    }

    #[test]
    fn arguments_with_multiple_unlimited() {
        let arguments = VecDeque::from([
            ArgumentConfig {
                name: "arg1".to_string(),
                narg: Narg::Unlimited,
            },
            ArgumentConfig {
                name: "arg2".to_string(),
                narg: Narg::Unlimited,
            },
        ]);
        let mut tp = TokenParser::new(HashSet::default(), arguments);

        tp.feed("value1").unwrap();
        tp.feed("value2").unwrap();

        assert_eq!(tp.parameters(), Err(()));
    }

    #[rstest]
    #[case(vec!["x", "y", "z"])]
    #[case(vec!["--verbose", "x", "y", "z"])]
    #[case(vec!["x", "y", "--verbose", "z"])]
    #[case(vec!["x", "y", "z", "--verbose"])]
    fn arguments_option_zero_mix(#[case] tokens: Vec<&str>) {
        let options = HashSet::from([OptionConfig {
            name: "verbose".to_string(),
            short: None,
            limit: 0,
        }]);
        let arguments = VecDeque::from([
            ArgumentConfig {
                name: "arg1".to_string(),
                narg: Narg::Limit(2),
            },
            ArgumentConfig {
                name: "arg2".to_string(),
                narg: Narg::Unlimited,
            },
        ]);
        let mut tp = TokenParser::new(options, arguments);

        for token in &tokens {
            tp.feed(token).unwrap();
        }

        let mut expected = HashSet::from([
            Parameter {
                name: "arg1".to_string(),
                values: vec!["x".to_string(), "y".to_string()],
            },
            Parameter {
                name: "arg2".to_string(),
                values: vec!["z".to_string()],
            },
        ]);
        if tokens.len() > 3 {
            expected.insert(Parameter {
                name: "verbose".to_string(),
                values: Vec::default(),
            });
        }

        assert_eq!(tp.parameters().unwrap(), expected);
    }

    #[rstest]
    #[case(vec!["x", "y", "z"])]
    #[case(vec!["--switch", "a", "x", "y", "z"])]
    #[case(vec!["--switch=a", "x", "y", "z"])]
    #[case(vec!["-sa", "x", "y", "z"])]
    #[case(vec!["-s=a", "x", "y", "z"])]
    #[case(vec!["-s", "a", "x", "y", "z"])]
    #[case(vec!["x", "y", "--switch", "a", "z"])]
    #[case(vec!["x", "y", "--switch=a", "z"])]
    #[case(vec!["x", "y", "-sa", "z"])]
    #[case(vec!["x", "y", "-s=a", "z"])]
    #[case(vec!["x", "y", "-s", "a", "z"])]
    #[case(vec!["x", "y", "z", "--switch", "a"])]
    #[case(vec!["x", "y", "z", "--switch=a"])]
    #[case(vec!["x", "y", "z", "-sa"])]
    #[case(vec!["x", "y", "z", "-s=a"])]
    #[case(vec!["x", "y", "z", "-s", "a"])]
    fn arguments_option_one_mix(#[case] tokens: Vec<&str>) {
        let options = HashSet::from([OptionConfig {
            name: "switch".to_string(),
            short: Some('s'),
            limit: 1,
        }]);
        let arguments = VecDeque::from([
            ArgumentConfig {
                name: "arg1".to_string(),
                narg: Narg::Limit(2),
            },
            ArgumentConfig {
                name: "arg2".to_string(),
                narg: Narg::Unlimited,
            },
        ]);
        let mut tp = TokenParser::new(options, arguments);

        for token in &tokens {
            tp.feed(token).unwrap();
        }

        let mut expected = HashSet::from([
            Parameter {
                name: "arg1".to_string(),
                values: vec!["x".to_string(), "y".to_string()],
            },
            Parameter {
                name: "arg2".to_string(),
                values: vec!["z".to_string()],
            },
        ]);
        if tokens.len() > 3 {
            expected.insert(Parameter {
                name: "switch".to_string(),
                values: vec!["a".to_string()],
            });
        }
        assert_eq!(tp.parameters().unwrap(), expected);
    }

    #[rstest]
    #[case(vec!["x", "y", "z"])]
    #[case(vec!["--switch", "a", "b", "x", "y", "z"])]
    #[case(vec!["-s", "a", "b", "x", "y", "z"])]
    #[case(vec!["x", "y", "--switch", "a", "b", "z"])]
    #[case(vec!["x", "y", "-s", "a", "b", "z"])]
    #[case(vec!["x", "y", "z", "--switch", "a", "b"])]
    #[case(vec!["x", "y", "z", "-s", "a", "b"])]
    fn arguments_option_two_mix(#[case] tokens: Vec<&str>) {
        let options = HashSet::from([OptionConfig {
            name: "switch".to_string(),
            short: Some('s'),
            limit: 2,
        }]);
        let arguments = VecDeque::from([
            ArgumentConfig {
                name: "arg1".to_string(),
                narg: Narg::Limit(2),
            },
            ArgumentConfig {
                name: "arg2".to_string(),
                narg: Narg::Unlimited,
            },
        ]);
        let mut tp = TokenParser::new(options, arguments);

        for token in &tokens {
            tp.feed(token).unwrap();
        }

        let mut expected = HashSet::from([
            Parameter {
                name: "arg1".to_string(),
                values: vec!["x".to_string(), "y".to_string()],
            },
            Parameter {
                name: "arg2".to_string(),
                values: vec!["z".to_string()],
            },
        ]);
        if tokens.len() > 3 {
            expected.insert(Parameter {
                name: "switch".to_string(),
                values: vec!["a".to_string(), "b".to_string()],
            });
        }
        assert_eq!(tp.parameters().unwrap(), expected);
        /*
        assert_eq!(
            tp.parameters().unwrap(),
            HashSet::from([
                Parameter {
                    name: "switch".to_string(),
                    values: vec!["a".to_string()],
                },
                Parameter {
                    name: "arg1".to_string(),
                    values: vec!["x".to_string(), "y".to_string()],
                },
                Parameter {
                    name: "arg2".to_string(),
                    values: vec!["z".to_string()],
                },
            ])
        );
        */
    }

    #[rstest]
    #[case(vec!["--switch=a", "x"])]
    #[case(vec!["-s=a", "x"])]
    #[case(vec!["-sa", "x"])]
    fn arguments_option_two_too_few(#[case] tokens: Vec<&str>) {
        let options = HashSet::from([OptionConfig {
            name: "switch".to_string(),
            short: Some('s'),
            limit: 2,
        }]);
        let arguments = VecDeque::from([ArgumentConfig {
            name: "arg".to_string(),
            narg: Narg::Unlimited,
        }]);
        let mut tp = TokenParser::new(options, arguments);

        for token in &tokens {
            tp.feed(token).unwrap();
        }

        assert_eq!(
            tp.parameters().unwrap(),
            HashSet::from([
                Parameter {
                    name: "switch".to_string(),
                    values: vec!["a".to_string()],
                },
                Parameter {
                    name: "arg".to_string(),
                    values: vec!["x".to_string()],
                },
            ])
        );
    }

    #[test]
    fn arguments_option_breaker() {
        let options = HashSet::from([OptionConfig {
            name: "verbose".to_string(),
            short: None,
            limit: 0,
        }]);
        let arguments = VecDeque::from([
            ArgumentConfig {
                name: "arg1".to_string(),
                narg: Narg::Limit(2),
            },
            ArgumentConfig {
                name: "arg2".to_string(),
                narg: Narg::Unlimited,
            },
        ]);
        let mut tp = TokenParser::new(options, arguments);

        for token in vec!["x", "--verbose", "z"] {
            tp.feed(token).unwrap();
        }

        assert_eq!(
            tp.parameters().unwrap(),
            HashSet::from([
                Parameter {
                    name: "verbose".to_string(),
                    values: Vec::default(),
                },
                Parameter {
                    name: "arg1".to_string(),
                    values: vec!["x".to_string()],
                },
                Parameter {
                    name: "arg2".to_string(),
                    values: vec!["z".to_string()],
                },
            ])
        );
    }
}
