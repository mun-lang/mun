pub mod myers;

use crate::{r#type::Field, r#type::Type};

use self::myers::Change;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum FieldEditKind {
    ChangedTyped,
    RenamedField,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FieldDiff {
    Insert {
        index: usize,
        new_type: Type,
    },
    Edit {
        old_type: Type,
        new_type: Type,
        old_index: Option<usize>,
        new_index: usize,
        kind: FieldEditKind,
    },
    Move {
        ty: Type,
        old_index: usize,
        new_index: usize,
    },
    Delete {
        index: usize,
    },
}

/// The difference between an old and new ordered set of structs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StructDiff {
    /// The struct was newly inserted
    Insert { index: usize, ty: Type },
    /// An existing struct was modified
    Edit {
        diff: Vec<FieldDiff>,
        old_index: usize,
        new_index: usize,
        old_ty: Type,
        new_ty: Type,
    },
    /// An existing struct was moved to another position
    Move {
        old_index: usize,
        new_index: usize,
        old_ty: Type,
        new_ty: Type,
    },
    /// An existing struct was deleted
    Delete { index: usize, ty: Type },
}

impl Ord for StructDiff {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        fn get_index(diff: &StructDiff) -> usize {
            match diff {
                StructDiff::Insert { index, .. }
                | StructDiff::Edit {
                    old_index: index, ..
                }
                | StructDiff::Move {
                    old_index: index, ..
                }
                | StructDiff::Delete { index, .. } => *index,
            }
        }

        get_index(self).cmp(&get_index(other))
    }
}

impl PartialOrd for StructDiff {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Given an `old` and a `new` ordered set of types, computes the difference based on ordering and equality of struct types.
/// Thus, a diff can consist of inserted, deleted, moved, and edited (i.e. fields of) struct types.
pub fn compute_struct_diff(old: &[Type], new: &[Type]) -> Vec<StructDiff> {
    let diff = myers::compute_diff(old, new);
    let (deletions, insertions) = myers::split_diff(&diff);

    let deleted_structs = deletions
        .into_iter()
        .filter(|Change { element, .. }| element.is_struct())
        .collect();

    let inserted_structs = insertions
        .into_iter()
        .filter(|Change { element, .. }| element.is_struct())
        .collect();

    let mut mapping: Vec<StructDiff> = Vec::with_capacity(diff.len());
    append_struct_mapping(deleted_structs, inserted_structs, &mut mapping);

    mapping.shrink_to_fit();
    // Sort to guarantee order of execution when deleting and/or inserting
    mapping.sort();
    mapping
}

/// A helper struct to check equality between fields.
#[derive(Clone, Eq, PartialEq)]
struct UniqueFieldInfo<'a> {
    name: &'a str,
    ty: Type,
}

impl<'a> From<Field<'a>> for UniqueFieldInfo<'a> {
    fn from(other: Field<'a>) -> Self {
        Self {
            name: other.name(),
            ty: other.ty(),
        }
    }
}

/// Given a set of indices for `deletions` from the `old` slice of types and a set of indices
/// for `insertions` into the `new` slice of types, appends the corresponding `Diff` mapping
/// for all
fn append_struct_mapping(
    deletions: Vec<Change<Type>>,
    insertions: Vec<Change<Type>>,
    mapping: &mut Vec<StructDiff>,
) {
    let deletions: Vec<_> = deletions
        .iter()
        .enumerate()
        .map(|(deletion_index, Change { index, element })| {
            let fields = element
                .as_struct()
                .map(|s| s.fields().iter().map(UniqueFieldInfo::from).collect())
                .unwrap_or_else(Vec::new);

            (deletion_index, *index, element.clone(), fields)
        })
        .collect();

    let insertions: Vec<_> = insertions
        .iter()
        .enumerate()
        .map(|(insertion_index, Change { index, element })| {
            let fields = element
                .as_struct()
                .map(|s| s.fields().iter().map(UniqueFieldInfo::from).collect())
                .unwrap_or_else(Vec::new);

            (insertion_index, *index, element.clone(), fields)
        })
        .collect();

    struct LengthDescription<'f> {
        deletion_idx: usize,
        insertion_idx: usize,
        old_index: usize,
        new_index: usize,
        old_ty: Type,
        new_ty: Type,
        old_fields: &'f Vec<UniqueFieldInfo<'f>>,
        new_fields: &'f Vec<UniqueFieldInfo<'f>>,
        length: usize,
    }

    // For all (insertion, deletion) pairs, calculate their `myers::diff_length`
    let mut myers_lengths: Vec<_> = insertions
        .iter()
        .flat_map(|(insertion_idx, new_idx, new_ty, new_fields)| {
            deletions
                .iter()
                .filter_map(|(deletion_idx, old_idx, old_ty, old_fields)| {
                    let length = myers::diff_length(old_fields, new_fields);

                    // Given N old fields and M new fields, the smallest set capable of
                    // completely changing a struct is N + M.
                    // E.g.
                    // old: [("a", Foo)]
                    // new: [("b", Bar), "c", Baz]
                    // `old` can be converted to `new` in 3 steps: 1 deletion + 2 insertions
                    // let min = new_fields.len() + old_fields.len();

                    // If the type's name is equal
                    if old_ty.name() == new_ty.name() || length == 0 {
                        // TODO: Potentially we want to retain an X% for types with equal names,
                        // whilst allowing types with different names to be modified for up to Y%.
                        Some(LengthDescription {
                            deletion_idx: *deletion_idx,
                            insertion_idx: *insertion_idx,
                            old_index: *old_idx,
                            new_index: *new_idx,
                            old_ty: old_ty.clone(),
                            new_ty: new_ty.clone(),
                            old_fields,
                            new_fields,
                            length,
                        })
                    } else {
                        // Indicate that the respective two fields are too different.
                        None
                    }
                })
                .collect::<Vec<LengthDescription>>()
        })
        .collect();

    // Sort in ascending order of their `myers::diff_length`.
    myers_lengths.sort_by(
        |LengthDescription { length: lhs, .. }, LengthDescription { length: rhs, .. }| lhs.cmp(rhs),
    );

    let mut used_deletions = vec![false; deletions.len()];
    let mut used_insertions = vec![false; insertions.len()];
    for LengthDescription {
        deletion_idx,
        insertion_idx,
        old_index,
        new_index,
        old_ty,
        new_ty,
        length,
        old_fields,
        new_fields,
    } in myers_lengths
    {
        // Skip marked fields
        if used_deletions[deletion_idx] || used_insertions[insertion_idx] {
            continue;
        }

        used_deletions[deletion_idx] = true;
        used_insertions[insertion_idx] = true;

        // If there is no difference between the old and new fields
        mapping.push(if length == 0 {
            // Move the struct
            StructDiff::Move {
                old_index,
                new_index,
                old_ty,
                new_ty,
            }
        } else {
            // ASSUMPTION: Don't use recursion, because all types are individually checked for
            // differences.
            // TODO: Support value struct vs heap struct?
            let diff = field_diff(old_fields, new_fields);

            // Edit the struct, potentially moving it in the process.
            StructDiff::Edit {
                diff,
                old_index,
                new_index,
                old_ty,
                new_ty,
            }
        });
    }

    // Any remaining unused deletions must have been deleted.
    used_deletions
        .into_iter()
        .zip(deletions)
        .for_each(|(used, (_, old_index, ty, _))| {
            if !used {
                mapping.push(StructDiff::Delete {
                    index: old_index,
                    ty,
                });
            }
        });

    // Any remaining unused insertions must have been inserted.
    used_insertions
        .into_iter()
        .zip(insertions)
        .for_each(|(used, (_, new_index, ty, _))| {
            if !used {
                mapping.push(StructDiff::Insert {
                    index: new_index,
                    ty,
                });
            }
        });
}

/// Given an `old` and a `new` set of fields, calculates the difference.
fn field_diff(old: &[UniqueFieldInfo<'_>], new: &[UniqueFieldInfo<'_>]) -> Vec<FieldDiff> {
    let diff = myers::compute_diff(old, new);
    let (deletions, insertions) = myers::split_diff(&diff);
    let mut insertions: Vec<Option<Change<UniqueFieldInfo>>> =
        insertions.into_iter().map(Some).collect();

    let mut mapping = Vec::with_capacity(diff.len());
    // For all deletions,
    #[allow(clippy::manual_flatten)]
    'outer: for Change {
        index: old_index,
        element: old_field,
    } in deletions
    {
        // is there an insertion with the same field name and type `T`?
        for insertion in insertions.iter_mut() {
            if let Some(Change {
                index: new_index,
                element: new_field,
            }) = insertion
            {
                if old_field == *new_field {
                    // If so, move it.
                    mapping.push(FieldDiff::Move {
                        ty: old_field.ty,
                        old_index,
                        new_index: *new_index,
                    });
                    *insertion = None;
                    continue 'outer;
                }
            }
        }
        // Else, is there an insertion with the same field name but different type `T`?
        for insertion in insertions.iter_mut() {
            if let Some(Change {
                index: new_index,
                element: new_field,
            }) = insertion
            {
                if old_field.name == new_field.name {
                    // If so,
                    mapping.push({
                        // convert the type in-place.
                        FieldDiff::Edit {
                            old_type: old_field.ty,
                            new_type: new_field.ty.clone(),
                            old_index: if old_index != *new_index {
                                Some(old_index)
                            } else {
                                None
                            },
                            new_index: *new_index,
                            kind: FieldEditKind::ChangedTyped,
                        }
                    });
                    *insertion = None;
                    continue 'outer;
                }
            }
        }
        // Else, is there an insertion with a different name but same type?
        // As there can be multiple fields with the same type, we want to find the closest one.
        let mut closest = None;
        for (insert_index, insertion) in insertions.iter_mut().enumerate() {
            if let Some(Change {
                index: new_index,
                element: new_field,
            }) = insertion
            {
                if old_field.ty == new_field.ty {
                    let diff = old_index.max(*new_index) - old_index.min(*new_index);
                    // If so, select the closest candidate.
                    if let Some((closest_insert_index, closest_index, closest_ty, closest_diff)) =
                        &mut closest
                    {
                        if diff < *closest_diff {
                            *closest_insert_index = insert_index;
                            *closest_index = *new_index;
                            *closest_ty = new_field.ty.clone();
                            *closest_diff = diff;
                        }
                    } else {
                        closest = Some((insert_index, *new_index, new_field.ty.clone(), diff));
                    }

                    // Terminate early if we managed to find the optimal solution (i.e. the field's
                    // position did not change).
                    if diff == 0 {
                        break;
                    }
                }
            }
        }
        // If there is one, use the closest match
        if let Some((closest_insert_index, closest_index, closest_type, _)) = closest {
            // Remove the insertion
            insertions
                .get_mut(closest_insert_index)
                .expect("Closest index must be within insertions")
                .take();

            mapping.push({
                // rename the field in-place.
                FieldDiff::Edit {
                    old_type: old_field.ty.clone(),
                    new_type: closest_type,
                    old_index: if old_index != closest_index {
                        Some(old_index)
                    } else {
                        None
                    },
                    new_index: closest_index,
                    kind: FieldEditKind::RenamedField,
                }
            });
            continue 'outer;
        }
        // If not, delete the field.
        mapping.push(FieldDiff::Delete { index: old_index })
    }

    // If an insertion did not have a matching deletion, then insert it.
    for Change {
        index,
        element: new_field,
    } in insertions.into_iter().flatten()
    {
        mapping.push(FieldDiff::Insert {
            index,
            new_type: new_field.ty,
        });
    }

    mapping.shrink_to_fit();
    mapping
}
