/// A [`StaticTypeMap`] is a map that maps from a type to a value.

use parking_lot::ReentrantMutex;
use std::any::TypeId;
use std::cell::RefCell;
use std::collections::HashMap;

/// An map with static types as keys
pub struct StaticTypeMap<T: 'static> {
    map: ReentrantMutex<RefCell<HashMap<TypeId, &'static T>>>,
}

impl<T: 'static> StaticTypeMap<T> {
    /// Instantiates a new instance
    pub fn new() -> Self {
        Self {
            map: ReentrantMutex::new(RefCell::new(HashMap::default())),
        }
    }

    /// Initialize static value corresponding to provided type.
    ///
    /// Initialized value will stay on heap until program terminated.
    /// No drop method will be called.
    pub fn call_once<Type, Init>(&'static self, f: Init) -> &'static T
    where
        Type: 'static,
        Init: FnOnce() -> T,
    {
        // If already initialized, just return stored value
        let map = self.map.lock();
        if let Some(r) = map.borrow().get(&TypeId::of::<Type>()) {
            return r;
        }

        // leak it's value until program is terminated
        let reference = Box::leak(Box::new(f()));

        // Insert the value into the map
        let old = map.borrow_mut().insert(TypeId::of::<Type>(), reference);
        if old.is_some() {
            panic!("StaticTypeMap value was reinitialized. This is a bug.")
        }
        reference
    }
}
