use rand::{distributions::Standard, prelude::Distribution, Rng};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Bound {
    Range(u8, u8),
    Lower(u8),
}

impl Distribution<Bound> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Bound {
        match rng.gen_range(0..2) {
            0 => {
                let upper: u8 = rng.gen();

                if upper == 0 {
                    Bound::Range(0, upper)
                } else {
                    Bound::Range(rng.gen_range(0..upper), upper)
                }
            }
            1 => Bound::Lower(rng.gen()),
            _ => panic!("internal error - impossible gen_range()"),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ArgumentConfig {
    name: String,
    bound: Bound,
}

impl ArgumentConfig {
    pub(crate) fn new(name: String, bound: Bound) -> Result<Self, ()> {
        match &bound {
            &Bound::Lower(n) | &Bound::Range(n, _) if n == 0 => {
                return Err(());
            }
            _ => Ok(Self { name, bound }),
        }
    }

    pub(crate) fn name(&self) -> String {
        self.name.clone()
    }

    pub(crate) fn bound(&self) -> Bound {
        self.bound
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct OptionConfig {
    name: String,
    short: Option<char>,
    bound: Bound,
}

impl OptionConfig {
    pub(crate) fn new(name: String, short: Option<char>, bound: Bound) -> Self {
        Self { name, short, bound }
    }

    pub(crate) fn name(&self) -> String {
        self.name.clone()
    }

    pub(crate) fn short(&self) -> &Option<char> {
        &self.short
    }

    pub(crate) fn bound(&self) -> Bound {
        self.bound
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub(crate) struct MatchTokens {
    pub name: String,
    pub values: Vec<OffsetValue>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub(super) enum CloseError {
    #[error("Too few values provided for '{name}' (provided={provided}, expected={expected}).")]
    TooFewValues {
        name: String,
        provided: usize,
        expected: u8,
    },

    #[error("Too many values provided for '{name}' (provided={provided}, expected={expected}).")]
    TooManyValues {
        name: String,
        provided: usize,
        expected: u8,
    },
}

type OffsetValue = (usize, String);

#[derive(Debug)]
pub(super) struct MatchBuffer {
    name: String,
    bound: Bound,
    values: Vec<OffsetValue>,
}

impl MatchBuffer {
    pub(super) fn new(name: String, bound: Bound) -> Self {
        Self {
            name,
            bound,
            values: Vec::default(),
        }
    }

    pub(super) fn push(&mut self, offset: usize, value: String) {
        self.values.push((offset, value));
    }

    pub(super) fn is_open(&self) -> bool {
        match self.bound {
            Bound::Range(_, n) => self.values.len() < n as usize,
            Bound::Lower(_) => true,
        }
    }

    pub(super) fn close(self) -> Result<MatchTokens, CloseError> {
        match self.bound {
            Bound::Lower(n) => {
                if self.values.len() < n as usize {
                    return Err(CloseError::TooFewValues {
                        name: self.name,
                        provided: self.values.len(),
                        expected: n,
                    });
                }
            }
            Bound::Range(i, j) => {
                if self.values.len() < i as usize {
                    return Err(CloseError::TooFewValues {
                        name: self.name,
                        provided: self.values.len(),
                        expected: i,
                    });
                } else if self.values.len() > j as usize {
                    return Err(CloseError::TooManyValues {
                        name: self.name,
                        provided: self.values.len(),
                        expected: j,
                    });
                }
            }
        };

        Ok(MatchTokens {
            name: self.name,
            values: self.values,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{thread_rng, Rng};
    use rstest::rstest;

    #[test]
    fn argument_config() {
        let name = "name".to_string();
        let bound: Bound = thread_rng().gen();
        let config = ArgumentConfig::new(name.clone(), bound).unwrap();
        assert_eq!(config.name(), name);
        assert_eq!(config.bound(), bound);
    }

    #[rstest]
    #[case(Bound::Range(0, 0))]
    #[case(Bound::Range(0, 1))]
    #[case(Bound::Range(0, 2))]
    #[case(Bound::Lower(0))]
    fn argument_config_invalid(#[case] bound: Bound) {
        let error = ArgumentConfig::new("name".to_string(), bound).unwrap_err();
        assert_eq!(error, ());
    }

    #[rstest]
    #[case(None)]
    #[case(Some('n'))]
    fn option_config(#[case] short: Option<char>) {
        let name = "name".to_string();
        let bound: Bound = thread_rng().gen();
        let config = OptionConfig::new(name.clone(), short.clone(), bound);
        assert_eq!(config.name(), name);
        assert_eq!(config.short(), &short);
        assert_eq!(config.bound(), bound);
    }

    #[rstest]
    #[case(Bound::Lower(0), 0, true)]
    #[case(Bound::Lower(0), 1, true)]
    #[case(Bound::Lower(1), 0, false)]
    #[case(Bound::Lower(1), 1, true)]
    #[case(Bound::Lower(1), 2, true)]
    #[case(Bound::Lower(10), 2, false)]
    #[case(Bound::Range(0, 2), 0, true)]
    #[case(Bound::Range(0, 2), 1, true)]
    #[case(Bound::Range(1, 2), 0, false)]
    #[case(Bound::Range(1, 2), 1, true)]
    #[case(Bound::Range(1, 2), 2, true)]
    #[case(Bound::Range(10, 20), 2, false)]
    fn match_buffer_lower(#[case] bound: Bound, #[case] feed: u8, #[case] expected_ok: bool) {
        let name = "name".to_string();
        let lower = match &bound {
            &Bound::Range(lower, _) => lower,
            &Bound::Lower(lower) => lower,
        };
        let remains_open = match &bound {
            &Bound::Range(_, upper) => upper > feed,
            _ => true,
        };
        let mut pb = MatchBuffer::new(name.clone(), bound);
        assert!(pb.is_open());
        let tokens: Vec<(usize, String)> = (0..feed)
            .map(|i| (thread_rng().gen(), i.to_string()))
            .collect();

        for (offset, token) in &tokens {
            pb.push(*offset, token.clone());
        }

        assert_eq!(pb.is_open(), remains_open);

        if expected_ok {
            assert_eq!(
                pb.close().unwrap(),
                MatchTokens {
                    name,
                    values: tokens,
                }
            );
        } else {
            assert_eq!(
                pb.close().unwrap_err(),
                CloseError::TooFewValues {
                    name,
                    provided: feed as usize,
                    expected: lower,
                }
            );
        }
    }

    #[rstest]
    #[case(Bound::Range(0, 0), 0, true)]
    #[case(Bound::Range(0, 0), 1, false)]
    #[case(Bound::Range(0, 1), 0, true)]
    #[case(Bound::Range(0, 1), 1, true)]
    #[case(Bound::Range(0, 1), 2, false)]
    #[case(Bound::Range(0, 10), 20, false)]
    fn match_buffer_upper(#[case] bound: Bound, #[case] feed: u8, #[case] expected_ok: bool) {
        let name = "name".to_string();
        let upper = match &bound {
            &Bound::Range(_, upper) => upper,
            _ => panic!("un-planned test case"),
        };
        let starts_open = upper > 0;
        let remains_open = upper > feed;
        let mut pb = MatchBuffer::new(name.clone(), bound);
        assert_eq!(pb.is_open(), starts_open);
        let tokens: Vec<(usize, String)> = (0..feed)
            .map(|i| (thread_rng().gen(), i.to_string()))
            .collect();

        for (offset, token) in &tokens {
            pb.push(*offset, token.clone());
        }

        if expected_ok {
            assert_eq!(pb.is_open(), remains_open);
            assert_eq!(
                pb.close().unwrap(),
                MatchTokens {
                    name,
                    values: tokens,
                }
            );
        } else {
            assert_eq!(
                pb.close().unwrap_err(),
                CloseError::TooManyValues {
                    name,
                    provided: feed as usize,
                    expected: upper,
                }
            );
        }
    }
}
