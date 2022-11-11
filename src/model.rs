/// Describes the number of command inputs associated with the argument/option.
/// Inspired by argparse: <https://docs.python.org/3/library/argparse.html#nargs>
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Nargs {
    /// N: Limited by precisely `N` values.
    Precisely(u8),
    /// *: May be any number of values, including `0`.
    Any,
    /// +: At least one value must be specified.
    AtLeastOne,
}

impl std::fmt::Display for Nargs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
