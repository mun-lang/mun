pub mod myers;

use crate::{r#type::Field, r#type::Type};

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

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Diff {
    Insert {
        index: usize,
    },
    Edit {
        diff: Vec<FieldDiff>,
        old_index: usize,
        new_index: usize,
    },
    Move {
        old_index: usize,
        new_index: usize,
    },
    Delete {
        index: usize,
    },
}

impl Ord for Diff {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        fn get_index(diff: &Diff) -> usize {
            match diff {
                Diff::Insert { index }
                | Diff::Edit {
                    old_index: index, ..
                }
                | Diff::Move {
                    old_index: index, ..
                }
                | Diff::Delete { index } => *index,
            }
        }

        get_index(self).cmp(&get_index(other))
    }
}

impl PartialOrd for Diff {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Given an `old` and a `new` set of types, calculates the difference.
pub fn diff(old: &[Type], new: &[Type]) -> Vec<Diff> {
    let diff = myers::diff(old, new);
    let (deletions, insertions) = myers::split_diff(&diff);

    let deleted_structs = deletions
        .iter()
        .filter(|idx| old.get(**idx).expect("Type must exist.").is_struct())
        .cloned()
        .collect();

    let inserted_structs = insertions
        .iter()
        .filter(|idx| new.get(**idx).expect("Type must exist.").is_struct())
        .cloned()
        .collect();

    let mut mapping: Vec<Diff> = Vec::with_capacity(diff.len());
    append_struct_mapping(old, new, deleted_structs, inserted_structs, &mut mapping);

    mapping.shrink_to_fit();
    // Sort to guarantee order of execution when deleting and/or inserting
    mapping.sort();
    mapping
}

/// A helper struct to check equality between fields.
#[derive(Eq, PartialEq)]
struct UniqueFieldInfo<'a> {
    name: &'a str,
    type_info: Type,
}

impl<'a> From<Field<'a>> for UniqueFieldInfo<'a> {
    fn from(other: Field<'a>) -> Self {
        Self {
            name: other.name(),
            type_info: other.ty(),
        }
    }
}

/// Given a set of indices for `deletions` from the `old` slice of types and a set of indices
/// for `insertions` into the `new` slice of types, appends the corresponding `Diff` mapping
/// for all
fn append_struct_mapping(
    old: &[Type],
    new: &[Type],
    deletions: Vec<usize>,
    insertions: Vec<usize>,
    mapping: &mut Vec<Diff>,
) {
    let old_fields: Vec<Vec<UniqueFieldInfo>> = old
        .iter()
        .map(|ty| {
            ty.as_struct()
                .map(|s| s.fields().iter().map(UniqueFieldInfo::from).collect())
                .unwrap_or_else(Vec::new)
        })
        .collect();

    let new_fields: Vec<Vec<UniqueFieldInfo>> = new
        .iter()
        .map(|ty| {
            ty.as_struct()
                .map(|s| s.fields().iter().map(UniqueFieldInfo::from).collect())
                .unwrap_or_else(Vec::new)
        })
        .collect();

    let num_deleted = deletions.len();
    let num_inserted = insertions.len();
    // For all (insertion, deletion) pairs, calculate their `myers::diff_length`
    let mut myers_lengths: Vec<usize> = insertions
        .iter()
        .flat_map(|new_idx| {
            let new_ty = new.get(*new_idx).expect("Type must exist.");
            let new_fields = new_fields.get(*new_idx).expect("Fields must exist.");

            deletions
                .iter()
                .map(|old_idx| {
                    let old_ty = unsafe { old.get_unchecked(*old_idx) };
                    let old_fields = unsafe { old_fields.get_unchecked(*old_idx) };

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
                        length
                    } else {
                        // `std::usize::MAX` is used to indicate that the respective two fields are
                        // too different.
                        std::usize::MAX
                    }
                })
                .collect::<Vec<usize>>()
        })
        .collect();

    let mut used_deletions = vec![false; num_deleted];
    let mut used_insertions = vec![false; num_inserted];
    // Traverse all (insertion, deletion) pairs in ascending order of their `myers::diff_length`.
    while let Some((idx, length)) = myers_lengths.iter().enumerate().min_by(|x, y| x.1.cmp(y.1)) {
        // Skip marked fields
        if *length == std::usize::MAX {
            break;
        }

        let delete_idx = idx % num_deleted;
        unsafe { *used_deletions.get_unchecked_mut(delete_idx) = true };

        let insert_idx = idx / num_deleted;
        unsafe { *used_insertions.get_unchecked_mut(insert_idx) = true };

        let old_index = unsafe { *deletions.get_unchecked(delete_idx) };
        let new_index = unsafe { *insertions.get_unchecked(insert_idx) };
        // If there is no difference between the old and new fields
        mapping.push(if *length == 0 {
            // Move the struct
            Diff::Move {
                old_index,
                new_index,
            }
        } else {
            let old_fields = unsafe { old_fields.get_unchecked(old_index) };
            let new_fields = unsafe { new_fields.get_unchecked(new_index) };

            // ASSUMPTION: Don't use recursion, because all types are individually checked for
            // differences.
            // TODO: Support value struct vs heap struct?
            let diff = field_diff(old_fields, new_fields);

            // Edit the struct, potentially moving it in the process.
            Diff::Edit {
                diff,
                old_index,
                new_index,
            }
        });

        // Prevent the row corresponding to the insertion entry from being used again, by inserting
        // `std::usize::MAX`.
        for idx in 0..num_deleted {
            let idx = insert_idx * num_deleted + idx;
            unsafe { *myers_lengths.get_unchecked_mut(idx) = std::usize::MAX };
        }

        // Prevent the column corresponding to the deletion entry from being used again, by
        // inserting `std::usize::MAX`.
        for idx in 0..num_inserted {
            let idx = idx * num_deleted + delete_idx;
            unsafe { *myers_lengths.get_unchecked_mut(idx) = std::usize::MAX };
        }
    }

    // Any remaining unused deletions must have been deleted.
    for (idx, used) in used_deletions.into_iter().enumerate() {
        if !used {
            mapping.push(Diff::Delete {
                index: unsafe { *deletions.get_unchecked(idx) },
            });
        }
    }

    // Any remaining unused insertions must have been inserted.
    for (idx, used) in used_insertions.into_iter().enumerate() {
        if !used {
            let index = unsafe { insertions.get_unchecked(idx) };
            mapping.push(Diff::Insert { index: *index });
        }
    }
}

/// Given an `old` and a `new` set of fields, calculates the difference.
fn field_diff<'a, 'b>(old: &[UniqueFieldInfo<'a>], new: &[UniqueFieldInfo<'b>]) -> Vec<FieldDiff> {
    let diff = myers::diff(old, new);
    let (deletions, insertions) = myers::split_diff(&diff);
    let mut insertions: Vec<Option<(usize, &UniqueFieldInfo)>> = insertions
        .into_iter()
        .map(|idx| {
            let new_ty = new.get(idx).expect("New type must exist.");
            Some((idx, new_ty))
        })
        .collect();

    let mut mapping = Vec::with_capacity(diff.len());
    // For all deletions,
    #[allow(clippy::manual_flatten)]
    'outer: for old_idx in deletions {
        let old_ty = old.get(old_idx).expect("Old type must exist.");
        // is there an insertion with the same field name and type `T`?
        for insertion in insertions.iter_mut() {
            if let Some((new_idx, new_ty)) = insertion {
                if *old_ty == **new_ty {
                    // If so, move it.
                    mapping.push(FieldDiff::Move {
                        ty: old_ty.type_info.clone(),
                        old_index: old_idx,
                        new_index: *new_idx,
                    });
                    *insertion = None;
                    continue 'outer;
                }
            }
        }
        // Else, is there an insertion with the same field name but different type `T`?
        for insertion in insertions.iter_mut() {
            if let Some((new_idx, new_ty)) = insertion {
                if old_ty.name == new_ty.name {
                    // If so,
                    mapping.push({
                        // convert the type in-place.
                        FieldDiff::Edit {
                            old_type: old_ty.type_info.clone(),
                            new_type: new_ty.type_info.clone(),
                            old_index: if old_idx != *new_idx {
                                Some(old_idx)
                            } else {
                                None
                            },
                            new_index: *new_idx,
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
        for (insert_idx, insertion) in insertions.iter_mut().enumerate() {
            if let Some((new_idx, new_ty)) = insertion {
                if old_ty.type_info == new_ty.type_info {
                    let diff = old_idx.max(*new_idx) - old_idx.min(*new_idx);
                    // If so, select the closest candidate.
                    if let Some((closest_idx, closest_ty, closest_diff)) = &mut closest {
                        if diff < *closest_diff {
                            *closest_idx = *new_idx;
                            *closest_ty = new_ty.type_info.clone();
                            *closest_diff = diff;
                        }
                    } else {
                        closest = Some((insert_idx, new_ty.type_info.clone(), diff));
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
        if let Some((closest_idx, closest_type, _)) = closest {
            let (new_idx, _) = unsafe { insertions.get_unchecked_mut(closest_idx) }
                .take()
                .unwrap();
            mapping.push({
                // rename the field in-place.
                FieldDiff::Edit {
                    old_type: old_ty.type_info.clone(),
                    new_type: closest_type,
                    old_index: if old_idx != new_idx {
                        Some(old_idx)
                    } else {
                        None
                    },
                    new_index: new_idx,
                    kind: FieldEditKind::RenamedField,
                }
            });
            continue 'outer;
        }
        // If not, delete the field.
        mapping.push(FieldDiff::Delete { index: old_idx })
    }

    // If an insertion did not have a matching deletion, then insert it.
    for (index, new_type) in insertions.into_iter().flatten() {
        mapping.push(FieldDiff::Insert {
            index,
            new_type: new_type.type_info.clone(),
        });
    }

    mapping.shrink_to_fit();
    mapping
}
