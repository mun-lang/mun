use crate::type_id::HasStaticTypeId;
use std::{
    ffi::{c_void, CStr},
    os::raw::c_char,
    slice, str,
};

use crate::type_id::TypeId;

/// Represents a function definition. A function definition contains the name, type signature, and
/// a pointer to the implementation.
///
/// `fn_ptr` can be used to call the declared function.
#[repr(C)]
#[derive(Clone)]
pub struct FunctionDefinition<'a> {
    /// Function prototype
    pub prototype: FunctionPrototype<'a>,
    /// Function pointer
    pub fn_ptr: *const c_void,
}

/// Represents a function prototype. A function prototype contains the name, type signature, but
/// not an implementation.
#[repr(C)]
#[derive(Clone)]
pub struct FunctionPrototype<'a> {
    /// Function name
    pub name: *const c_char,
    /// The type signature of the function
    pub signature: FunctionSignature<'a>,
}

/// Represents a function signature.
#[repr(C)]
#[derive(Clone)]
pub struct FunctionSignature<'a> {
    /// Argument types
    pub arg_types: *const TypeId<'a>,
    /// Optional return type
    pub return_type: TypeId<'a>,
    /// Number of argument types
    pub num_arg_types: u16,
}

// /// Owned storage for C-style `FunctionDefinition`.
// pub struct FunctionDefinitionStorage {
//     _name: CString,
//     _arg_types: Vec<TypeId>,
// }

unsafe impl<'a> Send for FunctionDefinition<'a> {}
unsafe impl<'a> Sync for FunctionDefinition<'a> {}

impl<'a> FunctionPrototype<'a> {
    /// Returns the function's name.
    pub fn name(&self) -> &str {
        unsafe { str::from_utf8_unchecked(CStr::from_ptr(self.name).to_bytes()) }
    }
}

unsafe impl<'a> Send for FunctionPrototype<'a> {}
unsafe impl<'a> Sync for FunctionPrototype<'a> {}

impl<'a> FunctionSignature<'a> {
    /// Returns the function's arguments' types.
    pub fn arg_types(&self) -> &[TypeId<'a>] {
        if self.num_arg_types == 0 {
            &[]
        } else {
            unsafe { slice::from_raw_parts(self.arg_types, self.num_arg_types as usize) }
        }
    }

    /// Returns the function's return type.
    pub fn return_type(&self) -> Option<TypeId<'a>> {
        if <()>::type_id() == &self.return_type {
            None
        } else {
            Some(self.return_type.clone())
        }
    }
}

impl<'a> PartialEq for FunctionSignature<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.return_type == other.return_type
            && self.arg_types().len() == other.arg_types().len()
            && self
                .arg_types()
                .iter()
                .zip(other.arg_types().iter())
                .all(|(a, b)| PartialEq::eq(a, b))
    }
}

impl<'a> Eq for FunctionSignature<'a> {}

unsafe impl<'a> Send for FunctionSignature<'a> {}
unsafe impl<'a> Sync for FunctionSignature<'a> {}

#[cfg(feature = "serde")]
impl<'a> serde::Serialize for FunctionDefinition<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;

        let mut s = serializer.serialize_struct("FunctionDefinition", 1)?;
        s.serialize_field("prototype", &self.prototype)?;
        s.skip_field("fn_ptr")?;
        s.end()
    }
}

#[cfg(feature = "serde")]
impl<'a> serde::Serialize for FunctionPrototype<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;

        let mut s = serializer.serialize_struct("FunctionPrototype", 2)?;
        s.serialize_field("name", self.name())?;
        s.serialize_field("signature", &self.signature)?;
        s.end()
    }
}

#[cfg(feature = "serde")]
impl<'a> serde::Serialize for FunctionSignature<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;

        let mut s = serializer.serialize_struct("FunctionSignature", 2)?;
        s.serialize_field("arg_types", self.arg_types())?;
        s.serialize_field("return_type", &self.return_type())?;
        s.end()
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::CString;

    use crate::test_utils::{fake_fn_prototype, fake_fn_signature, FAKE_FN_NAME};
    use crate::type_id::HasStaticTypeId;

    #[test]
    fn test_fn_prototype_name() {
        let fn_name = CString::new(FAKE_FN_NAME).expect("Invalid fake fn name.");
        let fn_signature = fake_fn_prototype(&fn_name, &[], None);

        assert_eq!(fn_signature.name(), FAKE_FN_NAME);
    }

    #[test]
    fn test_fn_signature_arg_types_none() {
        let arg_types = &[];
        let fn_signature = fake_fn_signature(arg_types, None);

        assert_eq!(fn_signature.arg_types(), arg_types);
    }

    #[test]
    fn test_fn_signature_arg_types_some() {
        let type_id = i32::type_id();

        let arg_types = &[type_id.clone()];
        let fn_signature = fake_fn_signature(arg_types, None);

        assert_eq!(fn_signature.arg_types(), arg_types);
    }

    #[test]
    fn test_fn_signature_return_type_none() {
        let return_type = None;
        let fn_signature = fake_fn_signature(&[], return_type.clone());

        assert_eq!(fn_signature.return_type(), return_type);
    }

    #[test]
    fn test_fn_signature_return_type_some() {
        let type_id = i32::type_id();

        let return_type = Some(type_id.clone());
        let fn_signature = fake_fn_signature(&[], return_type.clone());

        assert_eq!(fn_signature.return_type(), return_type);
    }
}
