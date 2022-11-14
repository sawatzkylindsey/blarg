use std::collections::{HashMap, HashSet, VecDeque};
use thiserror::Error;

use crate::constant::*;
use crate::matcher::*;

// We need a (dyn .. [ignoring T] ..) here in order to put all the fields of varying types T under one collection.
// In other words, we want the bottom of the object graph to include the types T, but up here we want to work across all T.
pub(crate) type OptionCapture<'ap> = (OptionConfig, Box<(dyn AnonymousCapturable + 'ap)>);
pub(crate) type ArgumentCapture<'ap> = (ArgumentConfig, Box<(dyn AnonymousCapturable + 'ap)>);

#[derive(Debug, Error)]
#[error("Config error: {0}")]
pub struct ConfigError(pub(crate) String);

impl From<TokenMatcherError> for ConfigError {
    fn from(error: TokenMatcherError) -> Self {
        match error {
            TokenMatcherError::DuplicateOption(_) => {
                unreachable!("internal error - invalid option should have been caught")
            }
            TokenMatcherError::DuplicateShortOption(_) => ConfigError(error.to_string()),
        }
    }
}

#[derive(Debug, Error)]
#[error("Parse error: {0}")]
pub(crate) struct ParseError(pub(crate) String);

impl From<MatchError> for ParseError {
    fn from(error: MatchError) -> Self {
        ParseError(error.to_string())
    }
}

/// Behaviour to capture an implicit generic type T from an input `&str`.
///
/// We use this at the middle/top of the argument parser object graph so that different types may all be 'captured' in a single argument parser.
pub(crate) trait AnonymousCapturable {
    /// Declare that the parameter has been matched.
    fn matched(&mut self);

    /// Capture a value anonymously for this parameter.
    fn capture(&mut self, value: &str) -> Result<(), ParseError>;
}

#[cfg(test)]
pub mod test {
    use crate::parser::{AnonymousCapturable, ParseError};

    pub(crate) struct BlackHole {}

    impl Default for BlackHole {
        fn default() -> Self {
            Self {}
        }
    }

    impl AnonymousCapturable for BlackHole {
        fn matched(&mut self) {
            // Do nothing
        }

        fn capture(&mut self, _value: &str) -> Result<(), ParseError> {
            // Do nothing
            Ok(())
        }
    }
}

pub(crate) struct Parser<'ap> {
    token_matcher: TokenMatcher,
    captures: HashMap<String, Box<(dyn AnonymousCapturable + 'ap)>>,
    discriminator: Option<String>,
}

impl<'ap> std::fmt::Debug for Parser<'ap> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Parser{..}").finish()
    }
}

impl<'ap> Parser<'ap> {
    #[cfg(test)]
    pub(crate) fn empty() -> Self {
        Self::new(Vec::default(), Vec::default(), None).unwrap()
    }

    pub(crate) fn new(
        options: Vec<OptionCapture<'ap>>,
        arguments: Vec<ArgumentCapture<'ap>>,
        discriminator: Option<String>,
    ) -> Result<Self, ConfigError> {
        let help_config = OptionConfig::new(HELP_NAME, Some(HELP_SHORT), Bound::Range(0, 0));
        let mut option_configs = HashSet::from([help_config]);
        let mut argument_configs = VecDeque::default();
        let mut captures: HashMap<String, Box<(dyn AnonymousCapturable + 'ap)>> =
            HashMap::default();

        for (oc, f) in options.into_iter() {
            if captures.insert(oc.name().to_string(), f).is_some() {
                return Err(ConfigError(format!(
                    "Cannot duplicate the parameter '{}'.",
                    oc.name()
                )));
            }

            option_configs.insert(oc);
        }

        for (ac, f) in arguments.into_iter() {
            if captures.insert(ac.name().to_string(), f).is_some() {
                return Err(ConfigError(format!(
                    "Cannot duplicate the parameter '{}'.",
                    ac.name()
                )));
            }

            argument_configs.push_back(ac);
        }

        let token_matcher = TokenMatcher::new(option_configs, argument_configs)?;

        Ok(Self {
            token_matcher,
            captures,
            discriminator,
        })
    }

    pub(crate) fn consume(self, tokens: &[&str]) -> Result<Action, (usize, ParseError)> {
        let Parser {
            mut token_matcher,
            mut captures,
            discriminator,
        } = self;

        let mut token_iter = tokens.iter();
        let minimal_consume = discriminator.is_some();
        // 1. Feed the raw token strings to the matcher.
        let mut fed = 0;

        loop {
            match token_iter.next() {
                Some(token) => {
                    let token_length = token.len();
                    token_matcher
                        .feed(token)
                        .map_err(|e| (fed, ParseError::from(e)))?;
                    fed += token_length;

                    if minimal_consume && token_matcher.can_close() {
                        break;
                    }
                }
                None => break,
            }
        }

        let matches = match token_matcher.close() {
            Ok(matches) | Err((_, _, matches)) if matches.contains(HELP_NAME) => {
                return Ok(Action::PrintHelp);
            }
            Ok(matches) => Ok(matches),
            Err((offset, e, _)) => Err((offset, ParseError::from(e))),
        }?;

        let mut discriminee: Option<(String, OffsetValue)> = None;

        // 2. Get the matching between tokens-parameter/options, still as raw strings.
        for match_tokens in matches.values {
            // 3. Find the corresponding capture.
            let mut box_capture = captures
                .remove(&match_tokens.name)
                .expect("internal error - mismatch between matches and captures");
            // 4. Let the capture know it has been matched.
            // Some captures may do something based off the fact they were simply matched.
            box_capture.matched();

            // 5. Convert each of the raw value strings into the capture type.
            for (offset, value) in &match_tokens.values {
                box_capture
                    .capture(value)
                    .map_err(|error| (*offset, error))?;
            }

            if let Some(ref target) = &discriminator {
                if target == &match_tokens.name {
                    match &match_tokens.values[..] {
                        [(offset, value)] => {
                            if discriminee
                                .replace((target.clone(), (*offset, value.clone())))
                                .is_some()
                            {
                                unreachable!(
                                    "internal error - discriminator cannot have multiple matches"
                                );
                            }
                        }
                        _ => {
                            unreachable!(
                                "internal error - discriminator must result it precisely 1 token"
                            );
                        }
                    }
                }
            }
        }

        Ok(Action::Continue {
            discriminee,
            remaining: token_iter.map(|s| s.to_string()).collect(),
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Action {
    Continue {
        discriminee: Option<(String, OffsetValue)>,
        remaining: Vec<String>,
    },
    PrintHelp,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{AnonymousCapture, Collection, GenericCapturable, Scalar};
    use crate::model::Nargs;
    use crate::parser::base::test::BlackHole;
    use rand::{thread_rng, Rng};
    use rstest::rstest;

    #[test]
    fn parser_empty() {
        // Setup
        let parser = Parser::empty();

        // Execute
        let result = parser.consume(empty::slice()).unwrap();

        // Verify
        assert_eq!(
            result,
            Action::Continue {
                discriminee: None,
                remaining: vec![],
            }
        );
    }

    #[rstest]
    #[case(vec!["--variable", "1"])]
    #[case(vec!["--variable", "01"])]
    #[case(vec!["-v", "1"])]
    #[case(vec!["-v", "01"])]
    #[case(vec!["-v=1"])]
    #[case(vec!["-v=01"])]
    fn parser_option(#[case] tokens: Vec<&str>) {
        // Setup
        let mut variable: u32 = 0;
        let generic_capture = Scalar::new(&mut variable);
        let config = OptionConfig::new("variable", Some('v'), generic_capture.nargs().into());
        let capture = AnonymousCapture::bind(generic_capture);
        let parser = Parser::new(vec![(config, Box::new(capture))], Vec::default(), None).unwrap();

        // Execute
        let result = parser.consume(tokens.as_slice()).unwrap();

        // Verify
        assert_eq!(
            result,
            Action::Continue {
                discriminee: None,
                remaining: vec![],
            }
        );
        assert_eq!(variable, 1);
    }

    #[rstest]
    #[case(vec![], vec![])]
    #[case(vec!["1"], vec![1])]
    #[case(vec!["1", "3", "2", "1"], vec![1, 3, 2, 1])]
    #[case(vec!["01"], vec![1])]
    fn parser_argument(#[case] tokens: Vec<&str>, #[case] expected: Vec<u32>) {
        // Setup
        let mut variable: Vec<u32> = Vec::default();
        let generic_capture = Collection::new(&mut variable, Nargs::Any);
        let config = ArgumentConfig::new("variable", generic_capture.nargs().into());
        let capture = AnonymousCapture::bind(generic_capture);
        let parser = Parser::new(Vec::default(), vec![(config, Box::new(capture))], None).unwrap();

        // Execute
        let result = parser.consume(tokens.as_slice()).unwrap();

        // Verify
        assert_eq!(
            result,
            Action::Continue {
                discriminee: None,
                remaining: vec![],
            }
        );
        assert_eq!(variable, expected);
    }

    #[rstest]
    #[case(vec!["--help"])]
    #[case(vec!["-h"])]
    #[case(vec!["--help", "1"])]
    #[case(vec!["-h", "1"])]
    #[case(vec!["--help", "not-a-u32"])]
    #[case(vec!["-h", "not-a-u32"])]
    fn parser_help(#[case] tokens: Vec<&str>) {
        // Setup
        let mut variable: u32 = 0;
        let generic_capture = Scalar::new(&mut variable);
        let config = ArgumentConfig::new("variable", generic_capture.nargs().into());
        let capture = AnonymousCapture::bind(generic_capture);
        let parser = Parser::new(Vec::default(), vec![(config, Box::new(capture))], None).unwrap();

        // Execute
        let result = parser.consume(tokens.as_slice()).unwrap();

        // Verify
        assert_eq!(result, Action::PrintHelp);
        assert_eq!(variable, 0);
    }

    #[rstest]
    #[case(vec!["1"], 0, "1", vec![])]
    #[case(vec!["01"], 0, "01", vec![])]
    #[case(vec!["1", "abc"], 0, "1", vec!["abc"])]
    #[case(vec!["1", "abc", "2"], 0, "1", vec!["abc", "2"])]
    #[case(vec!["--flag", "1"], 6, "1", vec![])]
    fn parser_discriminator(
        #[case] tokens: Vec<&str>,
        #[case] discriminee_offset: usize,
        #[case] discriminee_value: &str,
        #[case] expected: Vec<&str>,
    ) {
        // Setup
        let mut variable: u32 = 0;
        let generic_capture = Scalar::new(&mut variable);
        let name = "variable".to_string();
        let config = ArgumentConfig::new(name.clone(), generic_capture.nargs().into());
        let capture = AnonymousCapture::bind(generic_capture);
        let parser = Parser::new(
            vec![(
                OptionConfig::new("flag", None, Bound::Range(0, 0)),
                Box::new(BlackHole::default()),
            )],
            vec![(config, Box::new(capture))],
            Some(name.clone()),
        )
        .unwrap();

        // Execute
        let result = parser.consume(tokens.as_slice()).unwrap();

        // Verify
        assert_eq!(
            result,
            Action::Continue {
                discriminee: Some((name, (discriminee_offset, discriminee_value.to_string()))),
                remaining: expected.into_iter().map(|s| s.to_string()).collect(),
            }
        );
    }

    #[test]
    fn parser_duplicate_option() {
        let result = Parser::new(
            vec![
                (
                    OptionConfig::new("flag", None, thread_rng().gen()),
                    Box::new(BlackHole::default()),
                ),
                (
                    OptionConfig::new("flag", None, thread_rng().gen()),
                    Box::new(BlackHole::default()),
                ),
            ],
            Vec::default(),
            None,
        );
        assert_matches!(result, Err(ConfigError(_)));
    }

    #[test]
    fn parser_duplicate_option_short() {
        let result = Parser::new(
            vec![
                (
                    OptionConfig::new("flagA", Some('f'), thread_rng().gen()),
                    Box::new(BlackHole::default()),
                ),
                (
                    OptionConfig::new("flagB", Some('f'), thread_rng().gen()),
                    Box::new(BlackHole::default()),
                ),
            ],
            Vec::default(),
            None,
        );
        assert_matches!(result, Err(ConfigError(_)));
    }

    #[test]
    fn parser_duplicate_argument() {
        let result = Parser::new(
            Vec::default(),
            vec![
                (
                    ArgumentConfig::new("flag", thread_rng().gen()),
                    Box::new(BlackHole::default()),
                ),
                (
                    ArgumentConfig::new("flag", thread_rng().gen()),
                    Box::new(BlackHole::default()),
                ),
            ],
            None,
        );
        assert_matches!(result, Err(ConfigError(_)));
    }

    #[test]
    fn parser_duplicate_option_argument() {
        let result = Parser::new(
            vec![(
                OptionConfig::new("value", None, thread_rng().gen()),
                Box::new(BlackHole::default()),
            )],
            vec![(
                ArgumentConfig::new("value", thread_rng().gen()),
                Box::new(BlackHole::default()),
            )],
            None,
        );
        assert_matches!(result, Err(ConfigError(_)));
    }
}
