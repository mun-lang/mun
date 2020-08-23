use crate::{HasStaticTypeInfo, TypeInfo};
use std::{
    ffi::{c_void, CStr, CString},
    fmt::{self, Formatter},
    os::raw::c_char,
    ptr, slice, str,
};

/// Represents a function definition. A function definition contains the name, type signature, and
/// a pointer to the implementation.
///
/// `fn_ptr` can be used to call the declared function.
#[repr(C)]
#[derive(Clone)]
pub struct FunctionDefinition {
    /// Function prototype
    pub prototype: FunctionPrototype,
    /// Function pointer
    pub fn_ptr: *const c_void,
}

/// Represents a function prototype. A function prototype contains the name, type signature, but
/// not an implementation.
#[repr(C)]
#[derive(Clone)]
pub struct FunctionPrototype {
    /// Function name
    pub name: *const c_char,
    /// The type signature of the function
    pub signature: FunctionSignature,
}

/// Represents a function signature.
#[repr(C)]
#[derive(Clone)]
pub struct FunctionSignature {
    /// Argument types
    pub(crate) arg_types: *const *const TypeInfo,
    /// Optional return type
    pub(crate) return_type: *const TypeInfo,
    /// Number of argument types
    pub num_arg_types: u16,
}

/// Owned storage for C-style `FunctionDefinition`.
pub struct FunctionDefinitionStorage {
    _name: CString,
    _type_infos: Vec<&'static TypeInfo>,
}

unsafe impl Send for FunctionDefinition {}
unsafe impl Sync for FunctionDefinition {}

impl FunctionPrototype {
    /// Returns the function's name.
    pub fn name(&self) -> &str {
        unsafe { str::from_utf8_unchecked(CStr::from_ptr(self.name).to_bytes()) }
    }
}

impl fmt::Display for FunctionPrototype {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "fn {}(", self.name())?;
        for (i, arg) in self.signature.arg_types().iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", arg)?;
        }
        write!(f, ")")?;
        if let Some(ret_type) = self.signature.return_type() {
            write!(f, ":{}", ret_type)?
        }
        Ok(())
    }
}

unsafe impl Send for FunctionPrototype {}
unsafe impl Sync for FunctionPrototype {}

impl FunctionSignature {
    /// Returns the function's arguments' types.
    pub fn arg_types(&self) -> &[&TypeInfo] {
        if self.num_arg_types == 0 {
            &[]
        } else {
            unsafe {
                slice::from_raw_parts(
                    self.arg_types.cast::<&TypeInfo>(),
                    self.num_arg_types as usize,
                )
            }
        }
    }

    /// Returns the function's return type
    pub fn return_type(&self) -> Option<&TypeInfo> {
        unsafe { self.return_type.as_ref() }
    }
}

impl fmt::Display for FunctionSignature {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "fn(")?;
        for (i, arg) in self.arg_types().iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", arg)?;
        }
        write!(f, ")")?;
        if let Some(ret_type) = self.return_type() {
            write!(f, ":{}", ret_type)?
        }
        Ok(())
    }
}

impl PartialEq for FunctionSignature {
    fn eq(&self, other: &Self) -> bool {
        self.return_type() == other.return_type()
            && self.arg_types().len() == other.arg_types().len()
            && self
                .arg_types()
                .iter()
                .zip(other.arg_types().iter())
                .all(|(a, b)| PartialEq::eq(a, b))
    }
}

impl Eq for FunctionSignature {}

unsafe impl Send for FunctionSignature {}
unsafe impl Sync for FunctionSignature {}

impl FunctionDefinitionStorage {
    /// Constructs a new `FunctionDefinition`, the data of which is stored in a
    /// `FunctionDefinitionStorage`.
    pub fn new_function(
        name: &str,
        args: &[&'static TypeInfo],
        ret: Option<&'static TypeInfo>,
        fn_ptr: *const c_void,
    ) -> (FunctionDefinition, FunctionDefinitionStorage) {
        let name = CString::new(name).unwrap();
        let type_infos: Vec<&'static TypeInfo> = args.iter().copied().collect();

        let num_arg_types = type_infos.len() as u16;
        let return_type = if let Some(ty) = ret {
            ty as *const _
        } else {
            ptr::null()
        };

        let fn_info = FunctionDefinition {
            prototype: FunctionPrototype {
                name: name.as_ptr(),
                signature: FunctionSignature {
                    arg_types: type_infos.as_ptr() as *const *const _,
                    return_type,
                    num_arg_types,
                },
            },
            fn_ptr,
        };

        let fn_storage = FunctionDefinitionStorage {
            _name: name,
            _type_infos: type_infos,
        };

        (fn_info, fn_storage)
    }
}

/// A value-to-`FunctionDefinition` conversion that consumes the input value.
pub trait IntoFunctionDefinition {
    /// Performs the conversion.
    fn into<S: AsRef<str>>(self, name: S) -> (FunctionDefinition, FunctionDefinitionStorage);
}

macro_rules! into_function_info_impl {
    ($(
        extern "C" fn($($T:ident),*) -> $R:ident;
    )+) => {
        $(
            impl<$R: HasStaticTypeInfo, $($T: HasStaticTypeInfo,)*> IntoFunctionDefinition
            for extern "C" fn($($T),*) -> $R
            {
                fn into<S: AsRef<str>>(self, name: S) -> (FunctionDefinition, FunctionDefinitionStorage) {
                    FunctionDefinitionStorage::new_function(
                        name.as_ref(),
                        &[$($T::type_info(),)*],
                        Some($R::type_info()),
                        self as *const std::ffi::c_void,
                    )
                }
            }

            impl<$($T: HasStaticTypeInfo,)*> IntoFunctionDefinition
            for extern "C" fn($($T),*)
            {
                fn into<S: AsRef<str>>(self, name: S) -> (FunctionDefinition, FunctionDefinitionStorage) {
                    FunctionDefinitionStorage::new_function(
                        name.as_ref(),
                        &[$($T::type_info(),)*],
                        None,
                        self as *const std::ffi::c_void,
                    )
                }
            }
        )+
    }
}

into_function_info_impl! {
    extern "C" fn() -> R;
    extern "C" fn(A) -> R;
    extern "C" fn(A, B) -> R;
    extern "C" fn(A, B, C) -> R;
    extern "C" fn(A, B, C, D) -> R;
    extern "C" fn(A, B, C, D, E) -> R;
    extern "C" fn(A, B, C, D, E, F) -> R;
    extern "C" fn(A, B, C, D, E, F, G) -> R;
    extern "C" fn(A, B, C, D, E, F, G, H) -> R;
    extern "C" fn(A, B, C, D, E, F, G, H, I) -> R;
    extern "C" fn(A, B, C, D, E, F, G, H, I, J) -> R;
}

#[cfg(test)]
mod tests {
    use crate::{
        test_utils::{
            fake_fn_prototype, fake_fn_signature, fake_type_info, FAKE_FN_NAME, FAKE_TYPE_NAME,
        },
        TypeGroup,
    };
    use std::ffi::CString;

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
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");
        let type_info = fake_type_info(&type_name, TypeGroup::FundamentalTypes, 1, 1);

        let arg_types = &[&type_info];
        let fn_signature = fake_fn_signature(arg_types, None);

        assert_eq!(fn_signature.arg_types(), arg_types);
    }

    #[test]
    fn test_fn_signature_return_type_none() {
        let return_type = None;
        let fn_signature = fake_fn_signature(&[], return_type);

        assert_eq!(fn_signature.return_type(), return_type);
    }

    #[test]
    fn test_fn_signature_return_type_some() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");
        let type_info = fake_type_info(&type_name, TypeGroup::FundamentalTypes, 1, 1);

        let return_type = Some(&type_info);
        let fn_signature = fake_fn_signature(&[], return_type);

        assert_eq!(fn_signature.return_type(), return_type);
    }
}
