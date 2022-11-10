pub mod api;
mod constant;
mod matcher;
pub mod model;
mod parser;

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
