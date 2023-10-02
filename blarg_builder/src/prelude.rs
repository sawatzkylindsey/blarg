/// Behaviour for multiple (0 to many) items T to be collected together.
// Needs to be imported in order to implement a custom `Collectable`.
pub trait Collectable<T> {
    /// Add a value to this `Collectable`.
    fn add(&mut self, item: T);
}

/// Behaviour for documenting choices on a `Parameter` or `Condition`.
// Needs to be imported in order to document choices.
#[doc(hidden)]
pub trait Choices<T> {
    fn choice(self, variant: T, description: impl Into<String>) -> Self;
}
