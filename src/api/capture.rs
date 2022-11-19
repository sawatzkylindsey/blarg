use thiserror::Error;

use crate::model::Nargs;

/// Behaviour for multiple (0 to many) items T to be collected together.
pub trait Collectable<T> {
    /// Add a value to this `Collectable`.
    fn add(&mut self, item: T);
}

/// Marker trait for capturable types that can formulate an option in the Cli
pub trait CliOption {}

/// Marker trait for capturable types that can formulate an argument in the Cli.
pub trait CliArgument {}

/// Behaviour to capture an explicit generic type T from an input `&str`.
///
/// We use this at the bottom of the command line parser object graph so the compiler can maintain each field's type.
#[doc(hidden)]
pub trait GenericCapturable<'ap, T> {
    /// Declare that the parameter has been matched.
    fn matched(&mut self);

    /// Capture a value into the generic type T for this parameter.
    fn capture(&mut self, token: &str) -> Result<(), InvalidConversion>;

    /// Get the `Nargs` for this implementation.
    fn nargs(&self) -> Nargs;
}

#[derive(Debug, Error)]
#[doc(hidden)]
#[error("'{token}' cannot convert to {type_name}.")]
pub struct InvalidConversion {
    pub(crate) token: String,
    pub(crate) type_name: &'static str,
}
