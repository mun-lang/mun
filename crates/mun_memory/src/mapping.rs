use crate::{
    diff::{diff, Diff, FieldDiff, FieldEditKind},
    gc::GcPtr,
    type_info::TypeInfo,
    TypeFields,
};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

pub struct Mapping {
    pub deletions: HashSet<Arc<TypeInfo>>,
    pub conversions: HashMap<Arc<TypeInfo>, Conversion>,
    pub identical: Vec<(Arc<TypeInfo>, Arc<TypeInfo>)>,
}

pub struct Conversion {
    pub field_mapping: Vec<FieldMapping>,
    pub new_ty: Arc<TypeInfo>,
}

/// Description of the mapping of a single field. When stored together with the new index, this
/// provides all information necessary for a mapping function.
pub struct FieldMapping {
    pub new_ty: Arc<TypeInfo>,
    pub new_offset: usize,
    pub action: Action,
}

/// The `Action` to take when mapping memory from A to B.
#[derive(Eq, PartialEq)]
pub enum Action {
    Cast {
        old_offset: usize,
        old_ty: Arc<TypeInfo>,
    },
    Copy {
        old_offset: usize,
    },
    Insert,
}

impl Mapping {
    #[allow(clippy::mutable_key_type)]
    pub fn new(old: &[Arc<TypeInfo>], new: &[Arc<TypeInfo>]) -> Self {
        let diff = diff(old, new);

        let mut conversions = HashMap::new();
        let mut deletions = HashSet::new();
        let mut insertions = HashSet::new();

        let mut identical = Vec::new();

        for diff in diff.iter() {
            match diff {
                Diff::Delete { index } => {
                    deletions.insert(unsafe { old.get_unchecked(*index).clone() });
                }
                Diff::Edit {
                    diff,
                    old_index,
                    new_index,
                } => {
                    let old_ty = unsafe { old.get_unchecked(*old_index) };
                    let new_ty = unsafe { new.get_unchecked(*new_index) };
                    conversions.insert(old_ty.clone(), unsafe {
                        field_mapping(old_ty, new_ty, diff)
                    });
                }
                Diff::Insert { index } => {
                    insertions.insert(unsafe { new.get_unchecked(*index).clone() });
                }
                Diff::Move {
                    old_index,
                    new_index,
                } => identical.push(unsafe {
                    (
                        old.get_unchecked(*old_index).clone(),
                        new.get_unchecked(*new_index).clone(),
                    )
                }),
            }
        }

        // These candidates are used to collect a list of `new_index -> old_index` mappings for
        // identical types.
        let mut new_candidates: HashSet<_> = new
            .iter()
            // Filter non-struct types
            .filter(|ty| ty.data.is_struct())
            // Filter inserted structs
            .filter(|ty| !insertions.contains(*ty))
            .cloned()
            .collect();

        let mut old_candidates: HashSet<_> = old
            .iter()
            // Filter non-struct types
            .filter(|ty| ty.data.is_struct())
            // Filter deleted structs
            .filter(|ty| !deletions.contains(*ty))
            // Filter edited types
            .filter(|ty| {
                if let Some(conversion) = conversions.get(*ty) {
                    // Remove its new counterpart too
                    new_candidates.remove(&conversion.new_ty);
                    false
                } else {
                    true
                }
            })
            .cloned()
            .collect();

        // Remove moved types from the candidates, since we already know they are identical
        for (old_ty, new_ty) in identical.iter() {
            old_candidates.remove(old_ty);
            new_candidates.remove(new_ty);
        }

        // Find matching (old_ty, new_ty) pairs
        for old_ty in old_candidates {
            let new_ty = new_candidates.take(&old_ty).unwrap();
            identical.push((old_ty, new_ty));
        }

        // We should have matched all remaining candidates
        debug_assert!(new_candidates.is_empty());

        Self {
            deletions,
            conversions,
            identical,
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
pub unsafe fn field_mapping(
    old_ty: &Arc<TypeInfo>,
    new_ty: &Arc<TypeInfo>,
    diff: &[FieldDiff],
) -> Conversion {
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
        old_index: Option<usize>,
        action: ActionDesc,
    }

    #[derive(PartialEq)]
    enum ActionDesc {
        Cast,
        Copy,
        Insert,
    }

    // Add mappings for all `old_fields`, unless they were deleted or moved.
    let mut mapping: Vec<FieldMappingDesc> = (0..old_fields.len())
        .filter_map(|idx| {
            if deletions.contains(&idx) {
                None
            } else {
                Some(FieldMappingDesc {
                    old_index: Some(idx),
                    action: ActionDesc::Copy,
                })
            }
        })
        .collect();

    // Sort elements in ascending order of their insertion indices to guarantee that insertions
    // don't offset "later" insertions.
    let mut additions: Vec<(usize, FieldMappingDesc)> = diff
        .iter()
        .filter_map(|diff| match diff {
            FieldDiff::Insert { index } => Some((
                *index,
                FieldMappingDesc {
                    old_index: None,
                    action: ActionDesc::Insert,
                },
            )),
            FieldDiff::Move {
                old_index,
                new_index,
                edit,
            } => Some((
                *new_index,
                FieldMappingDesc {
                    old_index: Some(*old_index),
                    action: edit.as_ref().map_or(ActionDesc::Copy, |kind| {
                        if *kind == FieldEditKind::ConvertType {
                            ActionDesc::Cast
                        } else {
                            ActionDesc::Copy
                        }
                    }),
                },
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
            let map = mapping.get_mut(*index).unwrap();
            map.action = if *kind == FieldEditKind::ConvertType {
                ActionDesc::Cast
            } else {
                ActionDesc::Copy
            };
        }
    }

    let new_fields = new_ty.fields();
    Conversion {
        field_mapping: mapping
            .into_iter()
            .enumerate()
            .map(|(new_index, desc)| {
                let old_offset = desc
                    .old_index
                    .map(|idx| usize::from(old_fields.get_unchecked(idx).offset));

                FieldMapping {
                    new_ty: new_fields.get_unchecked(new_index).type_info.clone(),
                    new_offset: usize::from(new_fields.get_unchecked(new_index).offset),
                    action: match desc.action {
                        ActionDesc::Cast => Action::Cast {
                            old_offset: old_offset.unwrap(),
                            old_ty: old_fields
                                .get_unchecked(desc.old_index.unwrap())
                                .type_info
                                .clone(),
                        },
                        ActionDesc::Copy => Action::Copy {
                            old_offset: old_offset.unwrap(),
                        },
                        ActionDesc::Insert => Action::Insert,
                    },
                }
            })
            .collect(),
        new_ty: new_ty.clone(),
    }
}

/// A trait used to map allocated memory using type differences.
pub trait MemoryMapper {
    /// Maps its allocated memory using the provided `mapping`.
    ///
    /// A `Vec<GcPtr>` is returned containing all objects of types that were deleted. The
    /// corresponding types have to remain in-memory until the objects have been deallocated.
    fn map_memory(&self, mapping: Mapping) -> Vec<GcPtr>;
}
