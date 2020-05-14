pub mod myers;

use crate::{TypeDesc, TypeFields};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FieldEditKind {
    ConvertType,
    Rename,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FieldDiff {
    Insert {
        index: usize,
    },
    Edit {
        index: usize,
        kind: FieldEditKind,
    },
    Move {
        old_index: usize,
        new_index: usize,
        edit: Option<FieldEditKind>,
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

/// Given an `old` and a `new` set of types `T`, calculates the difference.
pub fn diff<T>(old: &[T], new: &[T]) -> Vec<Diff>
where
    T: Copy + Eq + TypeDesc + TypeFields<T>,
{
    let diff = myers::diff(old, new);
    let mut mapping: Vec<Diff> = Vec::with_capacity(diff.len());
    let (deletions, insertions) = myers::split_diff(&diff);

    // ASSUMPTION: `FundamentalTypes` can never be converted to `StructTypes`, hence they can be
    // compared separately.
    let deleted_fundamentals = deletions
        .iter()
        .filter(|idx| unsafe { old.get_unchecked(**idx) }.group().is_fundamental())
        .cloned()
        .collect();
    let deleted_structs = deletions
        .iter()
        .filter(|idx| unsafe { old.get_unchecked(**idx) }.group().is_struct())
        .cloned()
        .collect();

    let inserted_fundamentals = insertions
        .iter()
        .filter(|idx| unsafe { new.get_unchecked(**idx) }.group().is_fundamental())
        .cloned()
        .collect();
    let inserted_structs = insertions
        .iter()
        .filter(|idx| unsafe { new.get_unchecked(**idx) }.group().is_struct())
        .cloned()
        .collect();

    append_fundamental_mapping(
        old,
        new,
        deleted_fundamentals,
        inserted_fundamentals,
        &mut mapping,
    );
    append_struct_mapping(old, new, deleted_structs, inserted_structs, &mut mapping);

    mapping.shrink_to_fit();
    // Sort to guarantee order of execution when deleting and/or inserting
    mapping.sort();
    mapping
}

fn append_fundamental_mapping<T>(
    old: &[T],
    new: &[T],
    deletions: Vec<usize>,
    insertions: Vec<usize>,
    mapping: &mut Vec<Diff>,
) where
    T: Eq,
{
    let mut insertions: Vec<Option<usize>> = insertions.into_iter().map(Some).collect();

    // For all deletions,
    'outer: for old_idx in deletions {
        let old_ty = unsafe { old.get_unchecked(old_idx) };
        // is there an insertion
        for insertion in insertions.iter_mut() {
            if let Some(new_idx) = insertion {
                let new_ty = unsafe { new.get_unchecked(*new_idx) };
                // with the same type `T`?
                if *old_ty == *new_ty {
                    // If so, then `Move` it.
                    mapping.push(Diff::Move {
                        old_index: old_idx,
                        new_index: *new_idx,
                    });
                    *insertion = None;
                    continue 'outer;
                }
            }
        }
        // If not, `Delete` it.
        mapping.push(Diff::Delete { index: old_idx })
    }

    // If an insertion did not have a matching deletion, then `Insert` it.
    for insertion in insertions {
        if let Some(index) = insertion {
            mapping.push(Diff::Insert { index });
        }
    }
}

/// Given a set of indices for `deletions` from the `old` slice of types `T` and a set of indices
/// for `insertions` into the `new` slice of types `T`, appends the corresponding `Diff` mapping
/// for all
fn append_struct_mapping<T>(
    old: &[T],
    new: &[T],
    deletions: Vec<usize>,
    insertions: Vec<usize>,
    mapping: &mut Vec<Diff>,
) where
    T: Eq + TypeDesc + TypeFields<T>,
{
    let num_deleted = deletions.len();
    let num_inserted = insertions.len();
    // For all (insertion, deletion) pairs, calculate their `myers::diff_length`
    let mut myers_lengths: Vec<usize> = insertions
        .iter()
        .flat_map(|new_idx| {
            let new_ty = unsafe { new.get_unchecked(*new_idx) };
            let new_fields = new_ty.fields();
            deletions
                .iter()
                .map(|old_idx| {
                    let old_ty = unsafe { old.get_unchecked(*old_idx) };
                    let old_fields = old_ty.fields();
                    let length = myers::diff_length(&old_fields, &new_fields);

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
            let old_ty = unsafe { old.get_unchecked(old_index) };
            let new_ty = unsafe { new.get_unchecked(new_index) };

            // ASSUMPTION: Don't use recursion, because all types are individually checked for
            // differences.
            // TODO: Support value struct vs heap struct?
            let diff = field_diff(&old_ty.fields(), &new_ty.fields());

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
fn field_diff<T>(old: &[(&str, T)], new: &[(&str, T)]) -> Vec<FieldDiff>
where
    T: Eq,
{
    let diff = myers::diff(old, new);
    let (deletions, insertions) = myers::split_diff(&diff);
    let mut insertions: Vec<Option<usize>> = insertions.into_iter().map(Some).collect();

    let mut mapping = Vec::with_capacity(diff.len());
    // For all deletions,
    'outer: for old_idx in deletions {
        let old_ty = unsafe { old.get_unchecked(old_idx) };
        // is there an insertion with the same name and type `T`?
        for insertion in insertions.iter_mut() {
            if let Some(new_idx) = insertion {
                let new_ty = unsafe { new.get_unchecked(*new_idx) };
                if *old_ty == *new_ty {
                    // If so, move it.
                    mapping.push(FieldDiff::Move {
                        old_index: old_idx,
                        new_index: *new_idx,
                        edit: None,
                    });
                    *insertion = None;
                    continue 'outer;
                }
            }
        }
        // Else, is there an insertion with the same name but different type `T`?
        for insertion in insertions.iter_mut() {
            if let Some(new_idx) = insertion {
                let new_ty = unsafe { new.get_unchecked(*new_idx) };
                if old_ty.0 == new_ty.0 {
                    // If so,
                    mapping.push(if old_idx == *new_idx {
                        // convert the type in-place.
                        FieldDiff::Edit {
                            index: old_idx,
                            kind: FieldEditKind::ConvertType,
                        }
                    } else {
                        // convert the type and move it.
                        FieldDiff::Move {
                            old_index: old_idx,
                            new_index: *new_idx,
                            edit: Some(FieldEditKind::ConvertType),
                        }
                    });
                    *insertion = None;
                    continue 'outer;
                }
            }
        }
        // Else, is there an insertion with a different name but same type `T`?
        // As there can be multiple fields with the same type `T`, we want to find the closest one.
        let mut closest = None;
        for (insert_idx, insertion) in insertions.iter_mut().enumerate() {
            if let Some(new_idx) = insertion {
                let new_ty = unsafe { new.get_unchecked(*new_idx) };
                if old_ty.1 == new_ty.1 {
                    let diff = old_idx.max(*new_idx) - old_idx.min(*new_idx);
                    // If so, select the closest candidate.
                    if let Some((closest_idx, closest_diff)) = &mut closest {
                        if diff < *closest_diff {
                            *closest_idx = *new_idx;
                            *closest_diff = diff;
                        }
                    } else {
                        closest = Some((insert_idx, diff));
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
        if let Some((closest_idx, _)) = closest {
            let new_idx = unsafe { insertions.get_unchecked_mut(closest_idx) }
                .take()
                .unwrap();
            mapping.push(if old_idx == new_idx {
                // rename the field in-place.
                FieldDiff::Edit {
                    index: old_idx,
                    kind: FieldEditKind::Rename,
                }
            } else {
                // move and rename the field.
                FieldDiff::Move {
                    old_index: old_idx,
                    new_index: new_idx,
                    edit: Some(FieldEditKind::Rename),
                }
            });
            continue 'outer;
        }
        // If not, delete the field.
        mapping.push(FieldDiff::Delete { index: old_idx })
    }

    // If an insertion did not have a matching deletion, then insert it.
    for insertion in insertions {
        if let Some(index) = insertion {
            mapping.push(FieldDiff::Insert { index });
        }
    }

    mapping.shrink_to_fit();
    mapping
}
