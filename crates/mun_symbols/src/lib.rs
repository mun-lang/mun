#[macro_use]
extern crate lazy_static;
extern crate libloading;
extern crate uuid;

mod field;
mod method;
mod module;
mod reflection;

pub mod prelude {
    pub use crate::field::FieldInfo;
    pub use crate::method::*;
    pub use crate::module::ModuleInfo;
    pub use crate::reflection::{Reflectable, Reflection, TypeInfo};
    pub use crate::Privacy;
    pub use uuid::Uuid;
}

/// The privacy level of an identifier, either public or private.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Privacy {
    Public,
    Private,
}
