use abi::Guid;

use crate::{
    diff::{diff, Diff, FieldDiff},
    gc::GcPtr,
    r#type::Type,
    ArrayType, TypeKind,
};
use std::collections::{HashMap, HashSet};

pub struct Mapping {
    pub deletions: HashSet<Type>,
    pub struct_mappings: HashMap<Type, StructMapping>,
    pub identical: Vec<(Type, Type)>,
}

pub struct StructMapping {
    pub field_mapping: Vec<FieldMapping>,
    pub new_ty: Type,
}

/// Description of the mapping of a single field. When stored together with the new index, this
/// provides all information necessary for a mapping function.
#[derive(Debug)]
pub struct FieldMapping {
    pub new_ty: Type,
    pub new_offset: usize,
    pub action: Action,
}

/// The `Action` to take when mapping memory from A to B.
#[derive(Debug, Eq, PartialEq)]
pub enum Action {
    /// Allocate a new array.
    ArrayAlloc,
    /// Allocate a new array and initialize it with a single value.
    ArrayFromValue {
        element_action: Box<Action>,
        old_offset: usize,
    },
    /// Allocate a new array and map values from an old array.
    ArrayMap {
        element_action: Box<Action>,
        old_offset: usize,
    },
    /// Cast a primitive type.
    Cast { old_ty: Type, old_offset: usize },
    /// Copy bytes.
    Copy {
        old_offset: usize,
        /// Size in bytes
        size: usize,
    },
    /// Replace an array with its element type, copying its first element - if any.
    ElementFromArray {
        element_action: Box<Action>,
        old_offset: usize,
    },
    /// Allocate a new struct and ensure zero-initalization.
    StructAlloc,
    /// Allocate a new struct and map from a heap-allocated struct.
    StructMapFromGc { old_ty: Type, old_offset: usize },
    /// Allocate a new struct and map from a value struct.
    StructMapFromValue { old_ty: Type, old_offset: usize },
    /// Map a value struct in-place.
    StructMapInPlace { old_ty: Type, old_offset: usize },
    /// Ensure the memory is zero-initialized.
    ZeroInitialize,
}

impl Mapping {
    #[allow(clippy::mutable_key_type)]
    pub fn new(old: &[Type], new: &[Type]) -> Self {
        let diff = diff(old, new);

        let mut conversions = HashMap::new();
        let mut deletions = HashSet::new();
        let mut insertions = HashSet::new();

        let mut identical = Vec::new();

        for diff in diff.iter() {
            match diff {
                Diff::Delete { index } => {
                    deletions.insert(old.get(*index).expect("Old type must exist.").clone());
                }
                Diff::Edit {
                    diff,
                    old_index,
                    new_index,
                } => {
                    let old_ty = old.get(*old_index).expect("Old type must exist.");
                    let new_ty = new.get(*new_index).expect("New type must exist.");
                    conversions.insert(old_ty.clone(), unsafe {
                        field_mapping(old_ty, new_ty, diff)
                    });
                }
                Diff::Insert { index } => {
                    insertions.insert(new.get(*index).expect("New type must exist.").clone());
                }
                Diff::Move {
                    old_index,
                    new_index,
                } => identical.push((
                    old.get(*old_index).expect("Old type must exist.").clone(),
                    new.get(*new_index).expect("New type must exist.").clone(),
                )),
            }
        }

        // These candidates are used to collect a list of `new_index -> old_index` mappings for
        // identical types.
        let mut new_candidates: HashSet<_> = new
            .iter()
            // Filter non-struct types
            .filter(|ty| ty.is_struct())
            // Filter inserted structs
            .filter(|ty| !insertions.contains(*ty))
            .cloned()
            .collect();

        let mut old_candidates: HashSet<_> = old
            .iter()
            // Filter non-struct types
            .filter(|ty| ty.is_struct())
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
            struct_mappings: conversions,
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
pub unsafe fn field_mapping(old_ty: &Type, new_ty: &Type, diff: &[FieldDiff]) -> StructMapping {
    let old_fields = old_ty
        .as_struct()
        .map(|s| s.fields().iter().collect())
        .unwrap_or_else(Vec::new);

    let deletions: HashSet<usize> = diff
        .iter()
        .filter_map(|diff| match diff {
            FieldDiff::Delete { index } => Some(*index),
            FieldDiff::Move { old_index, .. } => Some(*old_index),
            FieldDiff::Edit { old_index, .. } => *old_index,
            FieldDiff::Insert { .. } => None,
        })
        .collect();

    // Add mappings for all `old_fields`, unless they were deleted or moved.
    let mut mapping: Vec<Action> = old_fields
        .iter()
        .enumerate()
        .filter_map(|(idx, old_field)| {
            if deletions.contains(&idx) {
                None
            } else {
                Some(Action::Copy {
                    old_offset: old_field.offset(),
                    size: old_field.ty().reference_layout().size(),
                })
            }
        })
        .collect();

    // Sort elements in ascending order of their insertion indices to guarantee that insertions
    // don't offset "later" insertions.
    let mut additions: Vec<(usize, Action)> = diff
        .iter()
        .filter_map(|diff| match diff {
            FieldDiff::Edit {
                old_type,
                new_type,
                old_index,
                new_index,
                ..
            } => old_index.map(|old_index| {
                let old_offset = old_fields
                    .get(old_index)
                    .map(|field| field.offset())
                    .expect("The old field must exist.");
                (*new_index, resolve_edit(old_type, new_type, old_offset))
            }),
            FieldDiff::Insert { index, new_type } => Some((
                *index,
                if new_type.is_struct() && !new_type.is_value_type() {
                    Action::StructAlloc
                } else if new_type.is_array() {
                    Action::ArrayAlloc
                } else {
                    Action::ZeroInitialize
                },
            )),
            FieldDiff::Move {
                ty,
                old_index,
                new_index,
            } => {
                let old_offset = old_fields
                    .get(*old_index)
                    .map(|field| field.offset())
                    .expect("Old field must exist.");

                Some((
                    *new_index,
                    Action::Copy {
                        old_offset,
                        size: ty.reference_layout().size(),
                    },
                ))
            }
            FieldDiff::Delete { .. } => None,
        })
        .collect();
    additions.sort_by(|a, b| a.0.cmp(&b.0));

    // Add mappings for all inserted and moved fields.
    for (new_index, map) in additions {
        mapping.insert(new_index, map);
    }

    // Modify the action for edited fields.
    for diff in diff.iter() {
        if let FieldDiff::Edit {
            old_type,
            new_type,
            old_index: None,
            new_index,
            ..
        } = diff
        {
            let old_offset = old_fields
                .get(*new_index)
                .map(|field| field.offset())
                .expect("The old field must exist.");

            let action = mapping.get_mut(*new_index).unwrap();
            *action = resolve_edit(old_type, new_type, old_offset);
        }
    }

    let new_fields = new_ty
        .as_struct()
        .map(|s| Vec::from_iter(s.fields().iter()))
        .unwrap_or_else(Vec::new);
    StructMapping {
        field_mapping: mapping
            .into_iter()
            .enumerate()
            .map(|(new_index, action)| {
                let new_field = new_fields
                    .get(new_index)
                    .unwrap_or_else(|| panic!("New field at index: '{}' must exist.", new_index));
                FieldMapping {
                    new_ty: new_field.ty(),
                    new_offset: new_field.offset(),
                    action,
                }
            })
            .collect(),
        new_ty: new_ty.clone(),
    }
}

pub fn resolve_edit(old_ty: &Type, new_ty: &Type, old_offset: usize) -> Action {
    match &old_ty.kind() {
        TypeKind::Primitive(old_guid) => {
            resolve_primitive_edit(old_ty, new_ty, old_guid, old_offset)
        }
        TypeKind::Struct(_) => resolve_struct_edit(old_ty, new_ty, old_offset),
        TypeKind::Pointer(_) => resolve_pointer_edit(old_ty, new_ty),
        TypeKind::Array(old_array) => resolve_array_edit(old_array, new_ty, old_offset),
    }
}

fn resolve_primitive_edit(
    old_ty: &Type,
    new_ty: &Type,
    old_guid: &Guid,
    old_offset: usize,
) -> Action {
    match &new_ty.kind() {
        TypeKind::Primitive(new_guid) => {
            resolve_primitive_to_primitive_edit(old_ty, old_guid, old_offset, new_guid)
        }
        TypeKind::Struct(s) => {
            if s.is_value_struct() {
                Action::ZeroInitialize
            } else {
                Action::StructAlloc
            }
        }
        TypeKind::Pointer(_) => unreachable!(),
        TypeKind::Array(new_array) => {
            resolve_primitive_to_array_edit(old_ty, new_array, old_offset)
        }
    }
}

fn resolve_primitive_to_primitive_edit(
    old_ty: &Type,
    old_guid: &Guid,
    old_offset: usize,
    new_guid: &Guid,
) -> Action {
    if *old_guid == *new_guid {
        Action::Copy {
            old_offset,
            size: old_ty.value_layout().size(),
        }
    } else {
        Action::Cast {
            old_ty: old_ty.clone(),
            old_offset,
        }
    }
}

fn resolve_primitive_to_array_edit(
    old_ty: &Type,
    new_array: &ArrayType,
    old_offset: usize,
) -> Action {
    Action::ArrayFromValue {
        element_action: Box::new(resolve_edit(old_ty, &new_array.element_type(), 0)),
        old_offset,
    }
}

fn resolve_struct_edit(old_ty: &Type, new_ty: &Type, old_offset: usize) -> Action {
    match &new_ty.kind() {
        TypeKind::Primitive(_) => Action::ZeroInitialize,
        TypeKind::Struct(_) => resolve_struct_to_struct_edit(old_ty, new_ty, old_offset),
        TypeKind::Pointer(_) => unreachable!(),
        TypeKind::Array(new_array) => resolve_struct_to_array_edit(old_ty, new_array, old_offset),
    }
}

pub fn resolve_struct_to_struct_edit(old_ty: &Type, new_ty: &Type, old_offset: usize) -> Action {
    // Early opt-out for when we are recursively resolving types (e.g. for arrays)
    if *old_ty == *new_ty {
        return Action::Copy {
            old_offset,
            size: old_ty.reference_layout().size(),
        };
    }

    // ASSUMPTION: When the name is the same, we are dealing with the same struct,
    // but different internals
    let is_same_struct = old_ty.name() == new_ty.name();

    if old_ty.is_value_type() && new_ty.is_value_type() {
        // struct(value) -> struct(value)
        if is_same_struct {
            Action::StructMapInPlace {
                old_ty: old_ty.clone(),
                old_offset,
            }
        } else {
            Action::ZeroInitialize
        }
    } else if old_ty.is_value_type() {
        // struct(value) -> struct(gc)
        if is_same_struct {
            Action::StructMapFromValue {
                old_ty: old_ty.clone(),
                old_offset,
            }
        } else {
            Action::StructAlloc
        }
    } else if new_ty.is_value_type() {
        // struct(gc) -> struct(value)
        if is_same_struct {
            Action::StructMapFromGc {
                old_ty: old_ty.clone(),
                old_offset,
            }
        } else {
            Action::ZeroInitialize
        }
    } else {
        // struct(gc) -> struct(gc)
        if is_same_struct {
            Action::Copy {
                old_offset,
                size: std::mem::size_of::<GcPtr>(),
            }
        } else {
            Action::StructAlloc
        }
    }
}

fn resolve_struct_to_array_edit(old_ty: &Type, new_array: &ArrayType, old_offset: usize) -> Action {
    Action::ArrayFromValue {
        element_action: Box::new(resolve_edit(old_ty, &new_array.element_type(), 0)),
        old_offset,
    }
}

fn resolve_pointer_edit(_old_ty: &Type, _new_ty: &Type) -> Action {
    // Not supported in the language - yet
    unreachable!()
}

fn resolve_array_edit(old_array: &ArrayType, new_ty: &Type, old_offset: usize) -> Action {
    match &new_ty.kind() {
        TypeKind::Primitive(_) => resolve_array_to_primitive_edit(old_array, new_ty, old_offset),
        TypeKind::Struct(_) => resolve_array_to_struct_edit(old_array, new_ty, old_offset),
        TypeKind::Pointer(_) => unreachable!(),
        TypeKind::Array(new_array) => resolve_array_to_array_edit(old_array, new_array, old_offset),
    }
}

fn resolve_array_to_primitive_edit(
    old_array: &ArrayType,
    new_ty: &Type,
    old_offset: usize,
) -> Action {
    Action::ElementFromArray {
        old_offset,
        element_action: Box::new(resolve_edit(&old_array.element_type(), new_ty, 0)),
    }
}

fn resolve_array_to_struct_edit(old_array: &ArrayType, new_ty: &Type, old_offset: usize) -> Action {
    Action::ElementFromArray {
        old_offset,
        element_action: Box::new(resolve_edit(&old_array.element_type(), new_ty, 0)),
    }
}

fn resolve_array_to_array_edit(
    old_array: &ArrayType,
    new_array: &ArrayType,
    old_offset: usize,
) -> Action {
    let old_element_type = old_array.element_type();
    let new_element_type = new_array.element_type();
    if old_element_type == new_element_type {
        Action::Copy {
            old_offset,
            size: std::mem::size_of::<GcPtr>(),
        }
    } else {
        Action::ArrayMap {
            element_action: Box::new(resolve_edit(&old_element_type, &new_element_type, 0)),
            old_offset,
        }
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
