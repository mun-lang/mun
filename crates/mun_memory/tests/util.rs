#![allow(dead_code)]

use std::alloc::Layout;

use mun_memory::{FieldInfo, StructInfo, TypeInfo};

/// Generates the Guid string for the struct.
///
/// This does not support recursive struct types!
pub fn struct_guid<'t>(name: &str, fields: impl Iterator<Item = &'t FieldInfo> + 't) -> abi::Guid {
    let guid_string = struct_guid_string(name, fields);
    abi::Guid::from_str(&guid_string)
}

fn struct_guid_string<'t>(name: &str, fields: impl Iterator<Item = &'t FieldInfo> + 't) -> String {
    let fields: Vec<String> = fields
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
        mun_memory::TypeInfoData::Struct(s) => {
            if s.memory_kind == abi::StructMemoryKind::Gc {
                format!("struct {}", type_info.name)
            } else {
                struct_guid_string(&type_info.name, s.fields.iter())
            }
        }
        mun_memory::TypeInfoData::Primitive(_) | mun_memory::TypeInfoData::Pointer(_) => {
            type_info.name.clone()
        }
    }
}

pub fn fake_layout(struct_info: &StructInfo) -> Layout {
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

        let fields: Vec<_ > = itertools::izip!(field_names, field_types, field_offsets)
            .map(|(name, type_info, offset)| mun_memory::FieldInfo {
                name,
                type_info,
                offset,
            })
            .collect();

        let name = String::from($struct_name);
        let guid = crate::util::struct_guid(&name, fields.iter());

        let struct_info = mun_memory::StructInfo {
            guid: guid.clone(),
            fields,
            memory_kind: abi::StructMemoryKind::Gc,
        };

        std::sync::Arc::new(mun_memory::TypeInfo {
            name,
            layout: crate::util::fake_layout(&struct_info),
            data: mun_memory::TypeInfoData::Struct(struct_info),
        })
    }};
}
