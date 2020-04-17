use crate::{
    diff::{Diff, FieldDiff, FieldEditKind},
    gc::GcPtr,
};
use std::collections::HashSet;

/// A trait used to map allocated memory using type differences.
pub trait MemoryMapper<T> {
    /// Maps the values memory from `old` to `new` using `diff`.
    ///
    /// A `Vec<GcPtr>` is returned containing all objects of types that were deleted. The
    /// corresponding types have to remain in-memory until the objects have been deallocated.
    fn map_memory(&self, old: &[T], new: &[T], diff: &[Diff]) -> Vec<GcPtr>;
}

/// The `Action` to take when mapping memory from A to B.
#[derive(Eq, PartialEq)]
pub enum Action {
    Cast,
    Copy,
}

/// Description of the mapping of a single field. When stored together with the new index, this
/// provides all information necessary for a mapping function.
pub struct FieldMappingDesc {
    pub old_index: usize,
    pub action: Action,
}

/// Given a set of `old_fields` of type `T` and their corresponding `diff`, calculates the mapping
/// `new_index -> Option<FieldMappingDesc>` for each new field.
///
/// The indices of the returned `Vec`'s elements should be used as indices for the new fields.
pub fn field_mapping<T>(old_fields: &[T], diff: &[FieldDiff]) -> Vec<Option<FieldMappingDesc>> {
    let deletions: HashSet<usize> = diff
        .iter()
        .filter_map(|diff| match diff {
            FieldDiff::Delete { index } => Some(*index),
            FieldDiff::Move { old_index, .. } => Some(*old_index),
            FieldDiff::Edit { .. } | FieldDiff::Insert { .. } => None,
        })
        .collect();

    // Add mappings for all `old_fields`, unless they were deleted or moved.
    let mut mapping: Vec<Option<FieldMappingDesc>> = (0..old_fields.len())
        .filter_map(|idx| {
            if deletions.contains(&idx) {
                None
            } else {
                Some(Some(FieldMappingDesc {
                    old_index: idx,
                    action: Action::Copy,
                }))
            }
        })
        .collect();

    // Sort elements in ascending order of their insertion indices to guarantee that insertions
    // don't offset "later" insertions.
    let mut additions: Vec<(usize, Option<FieldMappingDesc>)> = diff
        .iter()
        .filter_map(|diff| match diff {
            FieldDiff::Insert { index } => Some((*index, None)),
            FieldDiff::Move {
                old_index,
                new_index,
                edit,
            } => Some((
                *new_index,
                Some(FieldMappingDesc {
                    old_index: *old_index,
                    action: edit.as_ref().map_or(Action::Copy, |kind| {
                        if *kind == FieldEditKind::ConvertType {
                            Action::Cast
                        } else {
                            Action::Copy
                        }
                    }),
                }),
            )),
            FieldDiff::Delete { .. } | FieldDiff::Edit { .. } => None,
        })
        .collect();
    additions.sort_by(|a, b| a.0.cmp(&b.0));

    // Add mappings for all inserted and moved fields.
    for (new_index, map) in additions {
        mapping.insert(new_index, map);
    }

    // Set the action for edited fields.
    for diff in diff.iter() {
        if let FieldDiff::Edit { index, kind } = diff {
            if let Some(map) = mapping.get_mut(*index).unwrap() {
                map.action = if *kind == FieldEditKind::ConvertType {
                    Action::Cast
                } else {
                    Action::Copy
                };
            }
        }
    }
    mapping
}
