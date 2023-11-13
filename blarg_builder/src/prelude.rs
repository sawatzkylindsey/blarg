//! Traits which, typically, may be imported without concern: `use blarg::prelude::*`.

/// Behaviour for multiple (0 to many) items `T` to be collected together.
///
/// Must be imported in order to implement a custom `Collectable`.
pub trait Collectable<T> {
    /// Add a value to this `Collectable`.
    /// Return `Ok` on success, and `Err(message)` on failure.
    fn add(&mut self, item: T) -> Result<(), String>;
}

/// Behaviour for documenting choices on a [`Parameter`](../struct.Parameter.html) or [`Condition`](../struct.Condition.html).
///
/// Must be imported in order to document choices.
pub trait Choices<T> {
    fn choice(self, variant: T, description: impl Into<String>) -> Self;
}
