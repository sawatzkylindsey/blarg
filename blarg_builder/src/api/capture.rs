use thiserror::Error;

use crate::model::Nargs;

/// Marker trait for capturable types that can formulate an option in the Cli
pub trait CliOption {}

/// Marker trait for capturable types that can formulate an argument in the Cli.
pub trait CliArgument {}

/// Behaviour to capture an explicit generic type T from an input `&str`.
///
/// We use this at the bottom of the command line parser object graph so the compiler can maintain each field's type.
#[doc(hidden)]
pub trait GenericCapturable<'a, T> {
    /// Declare that the parameter has been matched.
    fn matched(&mut self);

    /// Capture a value into the generic type T for this parameter.
    fn capture(&mut self, token: &str) -> Result<(), InvalidCapture>;

    /// Get the `Nargs` for this implementation.
    fn nargs(&self) -> Nargs;
}

#[derive(Debug, Error)]
#[doc(hidden)]
pub enum InvalidCapture {
    #[error("cannot convert '{token}' to {type_name}.")]
    InvalidConversion {
        token: String,
        type_name: &'static str,
    },
    #[error("cannot collect '{token}': {message}.")]
    InvalidAdd { token: String, message: String },
}
