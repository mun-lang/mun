#![allow(dead_code)]
use mun_memory::{StructInfo, TypeInfo};
use std::alloc::Layout;

/// Generates the Guid string for the struct.
///
/// This does not support recursive struct types!
pub(crate) fn struct_guid(name: &str, struct_info: &StructInfo) -> abi::Guid {
    let guid_string = struct_guid_string(name, struct_info);
    abi::Guid::from(guid_string.as_bytes())
}

fn struct_guid_string(name: &str, struct_info: &StructInfo) -> String {
    let fields: Vec<String> = struct_info
        .fields
        .iter()
        .map(|field| {
            let ty_string = type_guid_string(&field.type_info);
            format!("{}: {}", field.name, ty_string)
        })
        .collect();

    format!(
        "struct {name}{{{fields}}}",
        name = name,
        fields = fields.join(",")
    )
}

fn type_guid_string(type_info: &TypeInfo) -> String {
    match &type_info.data {
        mun_memory::TypeInfoData::Primitive => type_info.name.clone(),
        mun_memory::TypeInfoData::Struct(s) => {
            if s.memory_kind == abi::StructMemoryKind::Gc {
                format!("struct {}", type_info.name)
            } else {
                struct_guid_string(&type_info.name, &s)
            }
        }
    }
}

pub(crate) fn fake_layout(struct_info: &StructInfo) -> Layout {
    let size = struct_info
        .fields
        .iter()
        .map(|field| field.type_info.layout.size())
        .sum();

    let alignment = struct_info
        .fields
        .iter()
        .map(|field| field.type_info.layout.align())
        .max()
        .unwrap();

    Layout::from_size_align(size, alignment).unwrap()
}

#[macro_export]
macro_rules! fake_struct {
    ($type_table:expr, $struct_name:expr, $($field_name:expr => $field_ty:ident),+) => {{
        let mut field_names = Vec::new();
        let mut field_types = Vec::new();

        $(
            field_names.push(String::from($field_name));
            field_types.push($type_table.find_type_info_by_name(format!("core::{}", stringify!($field_ty))).unwrap());
        )+

        let mut total_size = 0;
        let field_offsets: Vec<u16> = field_types
            .iter()
            .map(|ty| {
                let offset = total_size;
                total_size += ty.layout.size();
                offset as u16
            })
            .collect();

        let fields = itertools::izip!(field_names, field_types, field_offsets)
            .map(|(name, type_info, offset)| mun_memory::FieldInfo {
                name,
                type_info,
                offset,
            })
            .collect();

        let struct_info = mun_memory::StructInfo {
            fields,
            memory_kind: abi::StructMemoryKind::Gc,
        };

        let name = String::from($struct_name);

        std::sync::Arc::new(mun_memory::TypeInfo {
            id: crate::util::struct_guid(&name, &struct_info).into(),
            name,
            layout: crate::util::fake_layout(&struct_info),
            data: mun_memory::TypeInfoData::Struct(struct_info),
        })
    }};
}
