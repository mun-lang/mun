use crate::Runtime;
use std::cell::RefCell;
use std::ptr::NonNull;
use std::rc::Rc;

/// Used to do value-to-value conversions that require runtime type information while consuming the
/// input value.
///
/// If no `TypeInfo` is provided, the type is `()`.
pub trait Marshal<T>: Sized {
    /// Marshals itself into a `T`.
    fn marshal_value(self, runtime: Rc<RefCell<Runtime>>, type_info: Option<&abi::TypeInfo>) -> T;

    /// Marshals the value at memory location `ptr` into a `T`.
    fn marshal_from_ptr(
        ptr: NonNull<Self>,
        runtime: Rc<RefCell<Runtime>>,
        type_info: Option<&abi::TypeInfo>,
    ) -> T;

    /// Marshals `value` to memory location `ptr`.
    fn marshal_to_ptr(value: Self, ptr: NonNull<Self>, type_info: Option<&abi::TypeInfo>);
}

impl<T> Marshal<T> for T {
    fn marshal_value(
        self,
        _runtime: Rc<RefCell<Runtime>>,
        _type_info: Option<&abi::TypeInfo>,
    ) -> T {
        self
    }

    fn marshal_from_ptr(
        ptr: NonNull<Self>,
        _runtime: Rc<RefCell<Runtime>>,
        _type_info: Option<&abi::TypeInfo>,
    ) -> T {
        // TODO: Avoid unsafe `read` fn by using adding `Clone` trait to T.
        // This also requires changes to the `impl Struct`
        unsafe { ptr.as_ptr().read() }
    }

    fn marshal_to_ptr(value: T, mut ptr: NonNull<Self>, _type_info: Option<&abi::TypeInfo>) {
        unsafe { *ptr.as_mut() = value };
    }
}
