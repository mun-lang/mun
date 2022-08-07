#![allow(dead_code)]

use mun_memory::{
    diff::{myers, Diff, FieldDiff, FieldEditKind},
    Field, StructType, StructTypeBuilder, Type,
};
use std::collections::VecDeque;

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

pub(crate) fn apply_diff(old: &[Type], new: &[Type], diff: Vec<Diff>) -> Vec<Type> {
    let mut combined: Vec<Type> = old.to_vec();
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

                let combined_struct_info = old_ty
                    .as_struct()
                    .expect("edit diffs can only be applied on structs")
                    .clone();
                let new_struct_info = new_ty
                    .as_struct()
                    .expect("edit diffs can only be applied on structs");

                *old_ty = apply_struct_mapping(
                    old_ty.name(),
                    combined_struct_info,
                    new_struct_info,
                    diff,
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
    old_struct: StructType,
    new_struct: StructType,
    mapping: &[FieldDiff],
) -> Type {
    let mut fields: VecDeque<_> = old_struct
        .fields()
        .iter()
        .map(|f| (f.name().to_owned(), f.ty()))
        .collect();

    for diff in mapping.iter().rev() {
        match diff {
            FieldDiff::Delete { index } => {
                fields.remove(*index);
            }

            FieldDiff::Move { old_index, .. } => {
                fields.remove(*old_index);
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

    fn edit_field(kind: &FieldEditKind, old_field: &mut (String, Type), new_field: Field) {
        match *kind {
            FieldEditKind::ChangedTyped => old_field.1 = new_field.ty(),
            FieldEditKind::RenamedField => old_field.0 = new_field.name().to_owned(),
        }
    }

    // Sort elements in ascending order of their insertion indices.
    let mut additions: Vec<_> = mapping
        .iter()
        .filter_map(|diff| match diff {
            FieldDiff::Edit {
                old_type,
                new_type,
                old_index: Some(old_index),
                new_index,
                kind,
            } => {
                let combined = fields.get_mut(*old_index).unwrap();
                let new_field = new_struct.fields().get(*new_index).unwrap();

                edit_field(kind, combined, new_field);

                Some((*new_index, combined.clone()))
            }
            FieldDiff::Insert { index, .. } => {
                let new_field = new_struct.fields().get(*index).unwrap();
                Some((*index, (new_field.name().to_owned(), new_field.ty())))
            }
            FieldDiff::Move {
                old_index,
                new_index,
                ..
            } => {
                let field = old_struct.fields().get(*old_index).unwrap();
                Some((*new_index, (field.name().to_owned(), field.ty())))
            }
            _ => None,
        })
        .collect();
    additions.sort_by(|a, b| a.0.cmp(&b.0));

    for (index, field) in additions {
        fields.insert(index, field);
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
                fields.get_mut(*new_index).unwrap(),
                new_struct.fields().get(*new_index).unwrap(),
            );
        }
    }

    StructTypeBuilder::new(name.to_owned())
        .add_fields(fields)
        .finish()
}
