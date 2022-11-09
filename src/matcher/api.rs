use rand::{distributions::Standard, prelude::Distribution, Rng};

use crate::matcher::MatchTokens;
use crate::model::Nargs;

pub(crate) type OffsetValue = (usize, String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Bound {
    Range(u8, u8),
    Lower(u8),
}

impl From<Nargs> for Bound {
    fn from(value: Nargs) -> Self {
        match value {
            Nargs::Precisely(n) => Bound::Range(n, n),
            Nargs::Any => Bound::Lower(0),
            Nargs::AtLeastOne => Bound::Lower(1),
        }
    }
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
            _ => unreachable!("internal error - impossible gen_range()"),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ArgumentConfig {
    name: String,
    bound: Bound,
}

impl ArgumentConfig {
    pub(crate) fn new(name: impl Into<String>, bound: Bound) -> Self {
        Self {
            name: name.into(),
            bound,
        }
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
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
    pub(crate) fn new(name: impl Into<String>, short: Option<char>, bound: Bound) -> Self {
        Self {
            name: name.into(),
            short,
            bound,
        }
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn short(&self) -> &Option<char> {
        &self.short
    }

    pub(crate) fn bound(&self) -> Bound {
        self.bound
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Matches {
    pub values: Vec<MatchTokens>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Nargs;

    #[test]
    fn from_nargs() {
        assert_eq!(Bound::from(Nargs::Precisely(0)), Bound::Range(0, 0));
        assert_eq!(Bound::from(Nargs::Precisely(1)), Bound::Range(1, 1));
        assert_eq!(Bound::from(Nargs::Any), Bound::Lower(0));
        assert_eq!(Bound::from(Nargs::AtLeastOne), Bound::Lower(1));
    }
}
