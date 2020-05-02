//! A statically-allocated, concurrent data structure for storage of Rust objects that are utilized
//! through Mun Runtime C API calls.

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;

use lazy_static::lazy_static;
use parking_lot::{RwLock, RwLockReadGuard};

use crate::error::ErrorHandle;
use crate::TypedHandle;

fn generate_handle<H: TypedHandle>() -> H {
    loop {
        let token = rand::random();
        if token != 0 {
            return H::new(token);
        }
    }
}

/// A concurrent registry for uniquely indexed values.
pub struct Registry<H, T> {
    data: RwLock<HashMap<H, T>>,
}

impl<H, T> Registry<H, T>
where
    H: Copy + Eq + Hash + TypedHandle,
{
    /// Inserts `value` and returns a unique handle to it.
    pub fn register(&self, value: T) -> H {
        let handle = {
            let data = self.data.read();

            let mut handle = generate_handle();
            while data.contains_key(&handle) {
                handle = generate_handle();
            }
            handle
        };

        self.data.write().insert(handle, value);
        handle
    }

    /// Removes and returns the value corresponding to `handle`, if it is found.
    pub fn unregister(&self, handle: H) -> Option<T> {
        self.data.write().remove(&handle)
    }

    /// Retrieves the inner data
    pub fn get_data(&self) -> RwLockReadGuard<HashMap<H, T>> {
        self.data.read()
    }
}

impl<H, T> Default for Registry<H, T>
where
    H: Eq + Hash + TypedHandle,
{
    fn default() -> Self {
        Registry {
            data: RwLock::new(HashMap::new()),
        }
    }
}

/// Concurrent data structure for storage of Rust objects that are utilized through Mun Runtime
/// C API calls.
#[derive(Default)]
pub struct Hub {
    /// Error registry
    pub errors: Arc<Registry<ErrorHandle, failure::Error>>,
}

lazy_static! {
    /// Storage for Rust objects that are utilized through Mun Runtime C API calls.
    pub static ref HUB: Hub = Hub::default();
}
