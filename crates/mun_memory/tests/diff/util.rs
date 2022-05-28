#![allow(dead_code)]
use mun_memory::{
    diff::{myers, Diff, FieldDiff, FieldEditKind},
    FieldInfo, TypeInfo, TypeInfoData,
};
use std::sync::Arc;

use crate::util::{fake_layout, struct_guid};

pub fn apply_myers_diff<T: Clone + Eq>(old: &[T], new: &[T], diff: Vec<myers::Diff>) -> Vec<T> {
    let mut combined: Vec<_> = old.to_vec();
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

pub(crate) fn apply_diff(
    old: &[Arc<TypeInfo>],
    new: &[Arc<TypeInfo>],
    diff: Vec<Diff>,
) -> Vec<Arc<TypeInfo>> {
    let mut combined: Vec<Arc<TypeInfo>> = old.to_vec();
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

fn apply_mapping(old: &mut TypeInfo, new: &TypeInfo, mapping: &[FieldDiff]) {
    if let TypeInfoData::Struct(old_struct) = &mut old.data {
        if let TypeInfoData::Struct(new_struct) = &new.data {
            let mut combined = old_struct.clone();
            for diff in mapping.iter().rev() {
                match diff {
                    FieldDiff::Delete { index } => {
                        combined.fields.remove(*index);
                    }

                    FieldDiff::Move { old_index, .. } => {
                        combined.fields.remove(*old_index);
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
            let mut additions: Vec<(usize, FieldInfo)> = mapping
                .iter()
                .filter_map(|diff| match diff {
                    FieldDiff::Insert { index } => Some((
                        *index,
                        unsafe { new_struct.fields.get_unchecked(*index) }.clone(),
                    )),
                    FieldDiff::Move {
                        old_index,
                        new_index,
                        ..
                    } => Some((
                        *new_index,
                        unsafe { old_struct.fields.get_unchecked(*old_index) }.clone(),
                    )),
                    _ => None,
                })
                .collect();
            additions.sort_by(|a, b| a.0.cmp(&b.0));

            for (index, field) in additions {
                combined.fields.insert(index, field);
            }

            fn edit_field(kind: &FieldEditKind, old_field: &mut FieldInfo, new_field: &FieldInfo) {
                match *kind {
                    FieldEditKind::ConvertType => old_field.type_info = new_field.type_info.clone(),
                    FieldEditKind::Rename => old_field.name = new_field.name.to_owned(),
                }
            }

            // Handle edits
            for diff in mapping.iter() {
                match diff {
                    FieldDiff::Edit { index, kind } => edit_field(
                        kind,
                        unsafe { combined.fields.get_unchecked_mut(*index) },
                        unsafe { new_struct.fields.get_unchecked(*index) },
                    ),
                    FieldDiff::Move {
                        old_index,
                        new_index,
                        edit: Some(kind),
                    } => edit_field(
                        kind,
                        unsafe { combined.fields.get_unchecked_mut(*old_index) },
                        unsafe { new_struct.fields.get_unchecked(*new_index) },
                    ),
                    _ => (),
                }
            }

            *old_struct = combined;
            old.layout = fake_layout(old_struct);
            old.id.guid = struct_guid(&old.name, old_struct);
        } else {
            unreachable!()
        }
    } else {
        unreachable!()
    }
}
