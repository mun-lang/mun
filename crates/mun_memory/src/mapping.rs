use crate::{
    diff::{diff, Diff, FieldDiff, FieldEditKind},
    gc::GcPtr,
    TypeDesc, TypeFields, TypeLayout,
};
use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};

pub struct Mapping<T: Eq + Hash, U: TypeLayout> {
    pub deletions: HashSet<T>,
    pub conversions: HashMap<T, Conversion<U>>,
}

pub struct Conversion<T: TypeLayout> {
    pub field_mapping: Vec<Option<FieldMapping>>,
    pub new_ty: T,
}

/// Description of the mapping of a single field. When stored together with the new index, this
/// provides all information necessary for a mapping function.
pub struct FieldMapping {
    pub old_offset: usize,
    pub new_offset: usize,
    pub action: Action,
}

/// The `Action` to take when mapping memory from A to B.
#[derive(Eq, PartialEq)]
pub enum Action {
    Cast { old: abi::Guid, new: abi::Guid },
    Copy { size: usize },
}

impl<T> Mapping<T, T>
where
    T: TypeDesc + TypeFields<T> + TypeLayout + Copy + Eq + Hash,
{
    ///
    pub fn new(old: &[T], new: &[T]) -> Self {
        let diff = diff(old, new);

        let mut deletions = HashSet::new();
        let mut conversions = HashMap::new();

        for diff in diff.iter() {
            match diff {
                Diff::Delete { index } => {
                    deletions.insert(unsafe { *old.get_unchecked(*index) });
                }
                Diff::Edit {
                    diff,
                    old_index,
                    new_index,
                } => {
                    let old_ty = unsafe { *old.get_unchecked(*old_index) };
                    let new_ty = unsafe { *new.get_unchecked(*new_index) };
                    conversions.insert(old_ty, unsafe { field_mapping(old_ty, new_ty, diff) });
                }
                Diff::Insert { .. } | Diff::Move { .. } => (),
            }
        }

        Self {
            deletions,
            conversions,
        }
    }
}

/// Given a set of `old_fields` of type `T` and their corresponding `diff`, calculates the mapping
/// `new_index -> Option<FieldMappingDesc>` for each new field.
///
/// The indices of the returned `Vec`'s elements should be used as indices for the new fields.
///
/// # Safety
///
/// Expects the `diff` to be based on `old_ty` and `new_ty`. If not, it causes undefined behavior.
pub unsafe fn field_mapping<T: TypeDesc + TypeFields<T> + TypeLayout>(
    old_ty: T,
    new_ty: T,
    diff: &[FieldDiff],
) -> Conversion<T> {
    let old_fields = old_ty.fields();

    let deletions: HashSet<usize> = diff
        .iter()
        .filter_map(|diff| match diff {
            FieldDiff::Delete { index } => Some(*index),
            FieldDiff::Move { old_index, .. } => Some(*old_index),
            FieldDiff::Edit { .. } | FieldDiff::Insert { .. } => None,
        })
        .collect();

    struct FieldMappingDesc {
        old_index: usize,
        action: ActionDesc,
    }

    #[derive(PartialEq)]
    enum ActionDesc {
        Cast,
        Copy,
    }

    // Add mappings for all `old_fields`, unless they were deleted or moved.
    let mut mapping: Vec<Option<FieldMappingDesc>> = (0..old_fields.len())
        .filter_map(|idx| {
            if deletions.contains(&idx) {
                None
            } else {
                Some(Some(FieldMappingDesc {
                    old_index: idx,
                    action: ActionDesc::Copy,
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
                    action: edit.as_ref().map_or(ActionDesc::Copy, |kind| {
                        if *kind == FieldEditKind::ConvertType {
                            ActionDesc::Cast
                        } else {
                            ActionDesc::Copy
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
                    ActionDesc::Cast
                } else {
                    ActionDesc::Copy
                };
            }
        }
    }

    let new_fields = new_ty.fields();
    let old_offsets = old_ty.offsets();
    let new_offsets = new_ty.offsets();
    Conversion {
        field_mapping: mapping
            .into_iter()
            .enumerate()
            .map(|(new_index, desc)| {
                desc.map(|desc| {
                    let old_field = old_fields.get_unchecked(desc.old_index);
                    FieldMapping {
                        old_offset: usize::from(*old_offsets.get_unchecked(desc.old_index)),
                        new_offset: usize::from(*new_offsets.get_unchecked(new_index)),
                        action: if desc.action == ActionDesc::Cast {
                            Action::Cast {
                                old: *old_field.1.guid(),
                                new: *new_fields.get_unchecked(new_index).1.guid(),
                            }
                        } else {
                            Action::Copy {
                                size: old_field.1.layout().size(),
                            }
                        },
                    }
                })
            })
            .collect(),
        new_ty,
    }
}

/// A trait used to map allocated memory using type differences.
pub trait MemoryMapper<T: Eq + Hash + TypeLayout> {
    /// Maps its allocated memory using the provided `mapping`.
    ///
    /// A `Vec<GcPtr>` is returned containing all objects of types that were deleted. The
    /// corresponding types have to remain in-memory until the objects have been deallocated.
    fn map_memory(&self, mapping: Mapping<T, T>) -> Vec<GcPtr>;
}
