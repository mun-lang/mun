#![allow(dead_code)]
use mun_memory::{
    diff::{myers, Diff, FieldDiff, FieldEditKind},
    FieldInfo, StructInfo, TypeInfo,
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

                let mut combined_struct_info = old_ty
                    .as_struct()
                    .expect("edit diffs can only be applied on structs")
                    .clone();
                let new_struct_info = new_ty
                    .as_struct()
                    .expect("edit diffs can only be applied on structs");

                apply_struct_mapping(
                    &old_ty.name,
                    &mut combined_struct_info,
                    new_struct_info,
                    diff,
                );

                *old_ty = TypeInfo::new_struct(
                    &old_ty.name,
                    fake_layout(&combined_struct_info),
                    combined_struct_info,
                );
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

fn apply_struct_mapping(
    name: &str,
    old_struct: &mut StructInfo,
    new_struct: &StructInfo,
    mapping: &[FieldDiff],
) {
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
            FieldDiff::Insert { index, .. } => *index,
            FieldDiff::Move { new_index, .. } => *new_index,
            _ => std::usize::MAX,
        }
    }

    fn edit_field(kind: &FieldEditKind, old_field: &mut FieldInfo, new_field: &FieldInfo) {
        match *kind {
            FieldEditKind::ChangedTyped => old_field.type_info = new_field.type_info.clone(),
            FieldEditKind::RenamedField => old_field.name = new_field.name.to_owned(),
        }
    }

    // Sort elements in ascending order of their insertion indices.
    let mut additions: Vec<(usize, FieldInfo)> = mapping
        .iter()
        .filter_map(|diff| match diff {
            FieldDiff::Edit {
                old_type,
                new_type,
                old_index: Some(old_index),
                new_index,
                kind,
            } => {
                let old_field = unsafe { combined.fields.get_unchecked_mut(*old_index) };
                let new_field = unsafe { new_struct.fields.get_unchecked(*new_index) };

                edit_field(kind, old_field, new_field);

                Some((*new_index, old_field.clone()))
            }
            FieldDiff::Insert { index, .. } => Some((
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

    // Handle edits
    for diff in mapping.iter() {
        if let FieldDiff::Edit {
            old_type,
            new_type,
            old_index: None,
            new_index,
            kind,
        } = diff
        {
            edit_field(
                kind,
                unsafe { combined.fields.get_unchecked_mut(*new_index) },
                unsafe { new_struct.fields.get_unchecked(*new_index) },
            );
        }
    }

    *old_struct = combined;
    old_struct.guid = struct_guid(name, old_struct.fields.iter());
}
