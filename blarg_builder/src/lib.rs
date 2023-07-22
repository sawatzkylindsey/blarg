//! Core builder module for `blarg`.
//! See documentation root for more details.
#![deny(missing_docs)]
mod api;
mod constant;
mod matcher;
mod model;
mod parser;

pub use api::*;
pub use model::*;
pub use parser::GeneralParser;

#[cfg(test)]
#[macro_use]
extern crate assert_matches;

#[cfg(test)]
pub(crate) mod test {
    macro_rules! assert_contains {
        ($base:expr, $sub:expr) => {
            assert!(
                $base.contains($sub),
                "'{b}' does not contain '{s}'",
                b = $base,
                s = $sub,
            );
        };
    }

    pub(crate) use assert_contains;
}
