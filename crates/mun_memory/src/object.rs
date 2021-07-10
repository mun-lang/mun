/// Represents the interface to an object handled by the Garbage Collector.
pub trait Object<Type: Copy + Send + Sync, Value: Send + Sync> {
    /// Returns the type of the object
    fn prototype(&self) -> Type;

    /// Returns a pointer to the instance data of the object
    fn value_ptr(&self) -> Value;
}

