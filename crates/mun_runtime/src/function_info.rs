use std::{ffi::c_void, sync::Arc};

use memory::{type_table::TypeTable, TryFromAbiError, Type};

/// A linked version of [`mun_abi::FunctionDefinition`] that has resolved all occurrences of `TypeId` with `TypeInfo`.
#[derive(Clone)]
pub struct FunctionDefinition {
    /// Function prototype
    pub prototype: FunctionPrototype,
    /// Function pointer
    pub fn_ptr: *const c_void,
}

unsafe impl Send for FunctionDefinition {}
unsafe impl Sync for FunctionDefinition {}

impl FunctionDefinition {
    /// Tries to convert from an `abi::FunctionDefinition`.
    pub fn try_from_abi<'abi>(
        fn_def: &'abi abi::FunctionDefinition<'abi>,
        type_table: &TypeTable,
    ) -> Result<Self, TryFromAbiError<'abi>> {
        let prototype = FunctionPrototype::try_from_abi(&fn_def.prototype, type_table)?;

        Ok(Self {
            prototype,
            fn_ptr: fn_def.fn_ptr,
        })
    }
}

/// A linked version of [`mun_abi::FunctionPrototype`] that has resolved all occurrences of `TypeId` with `TypeInfo`.
#[derive(Clone)]
pub struct FunctionPrototype {
    /// Function name
    pub name: String,
    /// The type signature of the function
    pub signature: FunctionSignature,
}

impl FunctionPrototype {
    /// Tries to convert from an `abi::FunctionPrototype`.
    pub fn try_from_abi<'abi>(
        fn_prototype: &'abi abi::FunctionPrototype<'abi>,
        type_table: &TypeTable,
    ) -> Result<Self, TryFromAbiError<'abi>> {
        let signature = FunctionSignature::try_from_abi(&fn_prototype.signature, type_table)?;

        Ok(Self {
            name: fn_prototype.name().to_owned(),
            signature,
        })
    }
}

/// A linked version of [`mun_abi::FunctionSignature`] that has resolved all occurrences of `TypeId` with `TypeInfo`.
#[derive(Clone)]
pub struct FunctionSignature {
    /// Argument types
    pub arg_types: Vec<Arc<Type>>,
    /// Return type
    pub return_type: Arc<Type>,
}

impl FunctionSignature {
    /// Tries to convert from an `abi::FunctionSignature`.
    pub fn try_from_abi<'abi>(
        fn_sig: &'abi abi::FunctionSignature<'abi>,
        type_table: &TypeTable,
    ) -> Result<Self, TryFromAbiError<'abi>> {
        let arg_types: Vec<Arc<Type>> = fn_sig
            .arg_types()
            .iter()
            .map(|type_id| {
                type_table
                    .find_type_info_by_id(type_id)
                    .ok_or_else(|| TryFromAbiError::UnknownTypeId(type_id.clone()))
            })
            .collect::<Result<_, _>>()?;

        let return_type = type_table
            .find_type_info_by_id(&fn_sig.return_type)
            .ok_or_else(|| TryFromAbiError::UnknownTypeId(fn_sig.return_type.clone()))?;

        Ok(Self {
            arg_types,
            return_type,
        })
    }
}

/// A value-to-`FunctionDefinition` conversion that consumes the input value.
pub trait IntoFunctionDefinition {
    /// Performs the conversion.
    fn into<S: Into<String>>(self, name: S) -> FunctionDefinition;
}

macro_rules! into_function_info_impl {
    ($(
        extern "C" fn($($T:ident),*) -> $R:ident;
    )+) => {
        $(
            impl<$R: memory::HasStaticType, $($T: memory::HasStaticType,)*> IntoFunctionDefinition
            for extern "C" fn($($T),*) -> $R
            {
                fn into<S: Into<String>>(self, name: S) -> FunctionDefinition {
                    FunctionDefinition {
                        fn_ptr: self as *const std::ffi::c_void,
                        prototype: FunctionPrototype {
                            name: name.into(),
                            signature: FunctionSignature {
                                arg_types: vec![$(<$T as memory::HasStaticType>::type_info().clone(),)*],
                                return_type: <R as memory::HasStaticType>::type_info().clone(),
                            }
                        }
                    }
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
