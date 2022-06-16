use crate::type_info::HasStaticTypeId;
use crate::{
    AssemblyInfo, DispatchTable, FunctionDefinition, FunctionPrototype, FunctionSignature, Guid,
    ModuleInfo, StructInfo, StructMemoryKind, TypeId, TypeInfo, TypeInfoData, TypeLut,
};
use std::{
    ffi::{self, CStr},
    os::raw::c_char,
};

pub(crate) const FAKE_TYPE_GUID: Guid =
    Guid([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]);
pub(crate) const FAKE_TYPE_ID: TypeId = TypeId::Concrete(FAKE_TYPE_GUID);
pub(crate) const FAKE_DEPENDENCY: &str = "path/to/dependency.munlib";
pub(crate) const FAKE_FIELD_NAME: &str = "field_name";
pub(crate) const FAKE_FN_NAME: &str = "fn_name";
pub(crate) const FAKE_MODULE_PATH: &str = "path::to::module";
pub(crate) const FAKE_STRUCT_NAME: &str = "StructName";
pub(crate) const FAKE_TYPE_NAME: &str = "TypeName";

pub(crate) fn fake_assembly_info<'a>(
    symbols: ModuleInfo<'a>,
    dispatch_table: DispatchTable<'a>,
    type_lut: TypeLut<'a>,
    dependencies: &[*const c_char],
) -> AssemblyInfo<'a> {
    AssemblyInfo {
        symbols,
        dispatch_table,
        type_lut,
        dependencies: dependencies.as_ptr(),
        num_dependencies: dependencies.len() as u32,
    }
}

pub(crate) fn fake_type_lut<'a>(
    type_ids: &[TypeId<'a>],
    type_handles: &mut [*const ffi::c_void],
    type_names: &[*const c_char],
) -> TypeLut<'a> {
    assert_eq!(type_ids.len(), type_handles.len());

    TypeLut {
        type_ids: type_ids.as_ptr(),
        type_handles: type_handles.as_mut_ptr(),
        type_names: type_names.as_ptr(),
        num_entries: type_ids.len() as u32,
    }
}

pub(crate) fn fake_dispatch_table<'a>(
    fn_prototypes: &[FunctionPrototype<'a>],
    fn_ptrs: &mut [*const ffi::c_void],
) -> DispatchTable<'a> {
    assert_eq!(fn_prototypes.len(), fn_ptrs.len());

    DispatchTable {
        prototypes: fn_prototypes.as_ptr(),
        fn_ptrs: fn_ptrs.as_mut_ptr(),
        num_entries: fn_prototypes.len() as u32,
    }
}

pub(crate) fn fake_fn_signature<'a>(
    arg_types: &[TypeId<'a>],
    return_type: Option<TypeId<'a>>,
) -> FunctionSignature<'a> {
    FunctionSignature {
        arg_types: arg_types.as_ptr(),
        return_type: return_type.unwrap_or_else(|| <()>::type_id().clone()),
        num_arg_types: arg_types.len() as u16,
    }
}

pub(crate) fn fake_fn_prototype<'a>(
    name: &CStr,
    arg_types: &[TypeId<'a>],
    return_type: Option<TypeId<'a>>,
) -> FunctionPrototype<'a> {
    FunctionPrototype {
        name: name.as_ptr(),
        signature: fake_fn_signature(arg_types, return_type),
    }
}

pub(crate) fn fake_module_info<'a>(
    path: &CStr,
    functions: &[FunctionDefinition<'a>],
    types: &[TypeInfo<'a>],
) -> ModuleInfo<'a> {
    ModuleInfo {
        path: path.as_ptr(),
        functions: functions.as_ptr(),
        num_functions: functions.len() as u32,
        types: types.as_ptr(),
        num_types: types.len() as u32,
    }
}

pub(crate) fn fake_struct_info<'a>(
    name: &CStr,
    field_names: &[*const c_char],
    field_types: &[TypeId<'a>],
    field_offsets: &[u16],
    memory_kind: StructMemoryKind,
) -> StructInfo<'a> {
    assert!(field_names.len() == field_types.len());
    assert!(field_types.len() == field_offsets.len());

    StructInfo {
        guid: Guid::from_cstr(name),
        field_names: field_names.as_ptr(),
        field_types: field_types.as_ptr(),
        field_offsets: field_offsets.as_ptr(),
        num_fields: field_names.len() as u16,
        memory_kind,
    }
}

pub(crate) fn fake_type_info<'a>(
    name: &CStr,
    size: u32,
    alignment: u8,
    data: TypeInfoData<'a>,
) -> TypeInfo<'a> {
    TypeInfo {
        name: name.as_ptr(),
        size_in_bits: size,
        alignment,
        data,
    }
}

pub(crate) fn fake_primitive_type_info(
    name: &CStr,
    size: u32,
    alignment: u8,
) -> (TypeInfo, TypeId) {
    let guid = Guid::from(name.to_bytes());
    (
        TypeInfo {
            name: name.as_ptr(),
            size_in_bits: size,
            alignment,
            data: TypeInfoData::Primitive(guid.clone()),
        },
        guid.into(),
    )
}
