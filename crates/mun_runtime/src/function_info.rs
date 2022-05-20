use memory::{type_table::TypeTable, TypeInfo};
use std::{ffi::c_void, sync::Arc};

#[derive(Clone)]
pub struct FunctionDefinition {
    /// Function prototype
    pub prototype: FunctionPrototype,
    /// Function pointer
    pub fn_ptr: *const c_void,
}

impl FunctionDefinition {
    pub fn try_from_abi(fn_def: &abi::FunctionDefinition, type_table: &TypeTable) -> Option<Self> {
        let prototype = FunctionPrototype::try_from_abi(&fn_def.prototype, type_table)?;

        Some(Self {
            prototype,
            fn_ptr: fn_def.fn_ptr,
        })
    }
}

#[derive(Clone)]
pub struct FunctionPrototype {
    /// Function name
    pub name: String,
    /// The type signature of the function
    pub signature: FunctionSignature,
}

impl FunctionPrototype {
    pub fn try_from_abi(
        fn_prototype: &abi::FunctionPrototype,
        type_table: &TypeTable,
    ) -> Option<Self> {
        let signature = FunctionSignature::try_from_abi(&fn_prototype.signature, type_table)?;

        Some(Self {
            name: fn_prototype.name().to_owned(),
            signature,
        })
    }
}

#[derive(Clone)]
pub struct FunctionSignature {
    /// Argument types
    pub arg_types: Vec<Arc<TypeInfo>>,
    /// Optional return type
    pub return_type: Arc<TypeInfo>,
}

impl FunctionSignature {
    pub fn try_from_abi(fn_sig: &abi::FunctionSignature, type_table: &TypeTable) -> Option<Self> {
        let arg_types: Vec<Arc<TypeInfo>> = fn_sig
            .arg_types()
            .iter()
            .map(|type_id| type_table.find_type_info_by_id(type_id))
            .collect::<Option<_>>()?;

        let return_type = type_table.find_type_info_by_id(&fn_sig.return_type)?;

        Some(Self {
            arg_types,
            return_type,
        })
    }
}
