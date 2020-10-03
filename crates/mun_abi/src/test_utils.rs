use crate::{
    AssemblyInfo, DispatchTable, FunctionDefinition, FunctionPrototype, FunctionSignature, Guid,
    ModuleInfo, StructInfo, StructMemoryKind, TypeGroup, TypeInfo, TypeRef, TypeRefData,
};
use std::{
    ffi::{c_void, CStr},
    os::raw::c_char,
    ptr,
};

pub(crate) const FAKE_TYPE_GUID: Guid =
    Guid([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]);
pub(crate) const FAKE_DEPENDENCY: &str = "path/to/dependency.munlib";
pub(crate) const FAKE_FIELD_NAME: &str = "field_name";
pub(crate) const FAKE_FN_NAME: &str = "fn_name";
pub(crate) const FAKE_MODULE_PATH: &str = "path::to::module";
pub(crate) const FAKE_STRUCT_NAME: &str = "StructName";
pub(crate) const FAKE_TYPE_NAME: &str = "TypeName";

/// A dummy struct for initializing a struct's `TypeInfo`
pub(crate) struct StructTypeInfo {
    _type_info: TypeInfo,
    _struct_info: StructInfo,
}

pub(crate) fn fake_assembly_info(
    symbols: ModuleInfo,
    dispatch_table: DispatchTable,
    dependencies: &[*const c_char],
    types: &[&TypeInfo],
) -> AssemblyInfo {
    AssemblyInfo {
        symbols,
        dispatch_table,
        dependencies: dependencies.as_ptr(),
        num_dependencies: dependencies.len() as u32,
        types: types.as_ptr().cast::<*const TypeInfo>(),
        num_types: types.len() as u32,
    }
}

pub(crate) fn fake_dispatch_table(
    fn_prototypes: &[FunctionPrototype],
    fn_ptrs: &mut [*const c_void],
) -> DispatchTable {
    assert_eq!(fn_prototypes.len(), fn_ptrs.len());

    DispatchTable {
        prototypes: fn_prototypes.as_ptr(),
        fn_ptrs: fn_ptrs.as_mut_ptr(),
        num_entries: fn_prototypes.len() as u32,
    }
}

pub(crate) fn fake_fn_signature(
    arg_types: &[TypeRef],
    return_type: Option<&TypeRef>,
) -> FunctionSignature {
    FunctionSignature {
        arg_types: arg_types.as_ptr(),
        return_type: return_type.map_or(ptr::null(), |t| t as *const TypeRef),
        num_arg_types: arg_types.len() as u16,
    }
}

pub(crate) fn fake_fn_prototype(
    name: &CStr,
    arg_types: &[TypeRef],
    return_type: Option<&TypeRef>,
) -> FunctionPrototype {
    FunctionPrototype {
        name: name.as_ptr(),
        signature: fake_fn_signature(arg_types, return_type),
    }
}

pub(crate) fn fake_module_info(path: &CStr, functions: &[FunctionDefinition]) -> ModuleInfo {
    ModuleInfo {
        path: path.as_ptr(),
        functions: functions.as_ptr(),
        num_functions: functions.len() as u32,
    }
}

pub(crate) fn fake_struct_info(
    field_names: &[*const c_char],
    field_types: &[TypeRef],
    field_offsets: &[u16],
    memory_kind: StructMemoryKind,
) -> StructInfo {
    assert!(field_names.len() == field_types.len());
    assert!(field_types.len() == field_offsets.len());

    StructInfo {
        field_names: field_names.as_ptr(),
        field_types: field_types.as_ptr(),
        field_offsets: field_offsets.as_ptr(),
        num_fields: field_names.len() as u16,
        memory_kind,
    }
}

pub(crate) fn fake_struct_type_info(
    name: &CStr,
    struct_info: StructInfo,
    size: u32,
    alignment: u8,
) -> StructTypeInfo {
    StructTypeInfo {
        _type_info: fake_type_info(name, TypeGroup::StructTypes, size, alignment),
        _struct_info: struct_info,
    }
}

pub(crate) fn fake_type_info(name: &CStr, group: TypeGroup, size: u32, alignment: u8) -> TypeInfo {
    TypeInfo {
        guid: FAKE_TYPE_GUID,
        name: name.as_ptr(),
        size_in_bits: size,
        alignment,
        group,
    }
}

pub(crate) fn fake_type_ref(name: &CStr, data: TypeRefData) -> TypeRef {
    TypeRef {
        name: name.as_ptr(),
        guid: Guid(md5::compute(&name.to_bytes()).0),
        data,
    }
}
