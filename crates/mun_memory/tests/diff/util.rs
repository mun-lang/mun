#![allow(dead_code)]
use mun_memory::{
    diff::{myers, Diff, FieldDiff, FieldEditKind},
    StructInfo, TypeInfo, TypeInfoData,
};
use std::{alloc::Layout, sync::Arc};

pub const STRUCT1_NAME: &str = "struct1";
pub const STRUCT2_NAME: &str = "struct2";

pub(crate) fn fake_layout(struct_info: &StructInfo) -> Layout {
    let size = struct_info
        .field_types
        .iter()
        .map(|ty| ty.layout.size())
        .sum();

    let alignment = struct_info
        .field_types
        .iter()
        .map(|ty| ty.layout.align())
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
            field_types.push(std::sync::Arc::new(mun_memory::TypeInfo::try_from_abi(<$field_ty as abi::HasStaticTypeInfo>::type_info(), &$type_table).unwrap()));
        )+

        let mut total_size = 0;
        let field_offsets = field_types
            .iter()
            .map(|ty| {
                let offset = total_size;
                total_size += ty.layout.size();
                offset as u16
            })
            .collect();

        let struct_info = mun_memory::StructInfo {
            field_names,
            field_types,
            field_offsets,
            memory_kind: abi::StructMemoryKind::Gc,
        };

        let name = String::from($struct_name);

        std::sync::Arc::new(mun_memory::TypeInfo {
            // TODO: Calculate proper GUID!!
            id: abi::Guid::from(name.as_bytes()).into(),
            name,
            layout: crate::diff::util::fake_layout(&struct_info),
            data: mun_memory::TypeInfoData::Struct(struct_info),
        })
    }};
}

pub fn apply_myers_diff<'t, T: Clone + Eq>(old: &[T], new: &[T], diff: Vec<myers::Diff>) -> Vec<T> {
    let mut combined: Vec<_> = old.iter().cloned().collect();
    for diff in diff.iter().rev() {
        if let myers::Diff::Delete { index } = diff {
            combined.remove(*index);
        }
    }
    for diff in diff {
        if let myers::Diff::Insert { index } = diff {
            let value = unsafe { new.get_unchecked(index) };
            combined.insert(index, value.clone());
        }
    }
    combined
}

pub(crate) fn apply_diff<'t>(
    old: &[Arc<TypeInfo>],
    new: &[Arc<TypeInfo>],
    diff: Vec<Diff>,
) -> Vec<Arc<TypeInfo>> {
    let mut combined: Vec<Arc<TypeInfo>> = old.iter().cloned().collect();
    for diff in diff.iter().rev() {
        match diff {
            Diff::Delete { index } => {
                combined.remove(*index);
            }
            Diff::Edit {
                diff,
                old_index,
                new_index,
            } => {
                let old_ty = unsafe { combined.get_unchecked_mut(*old_index) };
                let new_ty = unsafe { new.get_unchecked(*new_index) };
                apply_mapping(Arc::make_mut(old_ty), new_ty, diff);
            }
            Diff::Move { old_index, .. } => {
                combined.remove(*old_index);
            }
            _ => (),
        }
    }
    for diff in diff {
        match diff {
            Diff::Insert { index } => {
                let new_ty = unsafe { new.get_unchecked(index) };
                combined.insert(index, (*new_ty).clone());
            }
            Diff::Move {
                old_index,
                new_index,
            } => {
                let old_ty = unsafe { old.get_unchecked(old_index) };
                combined.insert(new_index, (*old_ty).clone());
            }
            _ => (),
        }
    }
    combined
}

fn apply_mapping<'t>(old: &mut TypeInfo, new: &TypeInfo, mapping: &[FieldDiff]) {
    if let TypeInfoData::Struct(old_struct) = &mut old.data {
        if let TypeInfoData::Struct(new_struct) = &new.data {
            let mut combined = old_struct.clone();
            for diff in mapping.iter().rev() {
                match diff {
                    FieldDiff::Delete { index } => {
                        combined.field_names.remove(*index);
                        combined.field_types.remove(*index);
                        combined.field_offsets.remove(*index);
                    }

                    FieldDiff::Move { old_index, .. } => {
                        combined.field_names.remove(*old_index);
                        combined.field_types.remove(*old_index);
                        combined.field_offsets.remove(*old_index);
                    }
                    _ => (),
                }
            }

            fn get_new_index(diff: &FieldDiff) -> usize {
                match diff {
                    FieldDiff::Insert { index } => *index,
                    FieldDiff::Move { new_index, .. } => *new_index,
                    _ => std::usize::MAX,
                }
            }

            // Sort elements in ascending order of their insertion indices.
            let mut additions: Vec<(usize, String, Arc<TypeInfo>, u16)> = mapping
                .iter()
                .filter_map(|diff| match diff {
                    FieldDiff::Insert { index } => Some((
                        *index,
                        unsafe { new_struct.field_names.get_unchecked(*index) }.clone(),
                        unsafe { new_struct.field_types.get_unchecked(*index) }.clone(),
                        unsafe { *new_struct.field_offsets.get_unchecked(*index) },
                    )),
                    FieldDiff::Move {
                        old_index,
                        new_index,
                        ..
                    } => Some((
                        *new_index,
                        unsafe { old_struct.field_names.get_unchecked(*old_index) }.clone(),
                        unsafe { old_struct.field_types.get_unchecked(*old_index) }.clone(),
                        unsafe { *old_struct.field_offsets.get_unchecked(*old_index) },
                    )),
                    _ => None,
                })
                .collect();
            additions.sort_by(|a, b| a.0.cmp(&b.0));

            for (index, name, ty, offset) in additions {
                combined.field_names.insert(index, name);
                combined.field_types.insert(index, ty);
                combined.field_offsets.insert(index, offset);
            }

            fn edit_field(
                kind: &FieldEditKind,
                field_index: usize,
                old_struct: &mut StructInfo,
                new_field_name: &str,
                new_field_type: &Arc<TypeInfo>,
            ) {
                match *kind {
                    FieldEditKind::ConvertType => {
                        old_struct.field_types[field_index] = new_field_type.clone()
                    }
                    FieldEditKind::Rename => {
                        old_struct.field_names[field_index] = new_field_name.to_owned()
                    }
                }
            }

            // Handle edits
            for diff in mapping.iter() {
                match diff {
                    FieldDiff::Edit { index, kind } => edit_field(
                        kind,
                        *index,
                        &mut combined,
                        unsafe { new_struct.field_names.get_unchecked(*index) },
                        unsafe { new_struct.field_types.get_unchecked(*index) },
                    ),
                    FieldDiff::Move {
                        old_index,
                        new_index,
                        edit: Some(kind),
                    } => edit_field(
                        kind,
                        *old_index,
                        &mut combined,
                        unsafe { new_struct.field_names.get_unchecked(*new_index) },
                        unsafe { new_struct.field_types.get_unchecked(*new_index) },
                    ),
                    _ => (),
                }
            }

            *old_struct = combined;
            old.layout = fake_layout(&old_struct);
        } else {
            unreachable!()
        }
    } else {
        unreachable!()
    }
}
