use std::any::TypeId;
use std::collections::HashMap;
use std::sync::RwLock;

pub struct StaticTypeMap<T: 'static> {
    map: RwLock<HashMap<TypeId, &'static T>>,
}

impl<T: 'static> StaticTypeMap<T> {
    pub fn new() -> Self {
        Self {
            map: RwLock::new(HashMap::new()),
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
        {
            let reader = self.map.read().unwrap();
            if let Some(ref reference) = reader.get(&TypeId::of::<Type>()) {
                return &reference;
            }
        }

        // Construct new value and put inside map allocate value on heap
        let boxed = Box::new(f());

        // Get exclusive access
        let mut writer = self.map.write().unwrap();

        // Recheck because maybe we are the second writer and the previous writer inserted the
        // value.
        if let Some(ref reference) = writer.get(&TypeId::of::<Type>()) {
            return &reference;
        }

        // leak it's value until program is terminated
        let reference: &'static T = Box::leak(boxed);

        // Insert the value into the map
        let old = writer.insert(TypeId::of::<Type>(), reference);
        if old.is_some() {
            panic!("StaticTypeMap value was reinitialized. This is a bug.")
        }
        reference
    }
}
