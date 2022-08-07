use abi::{Guid, StructMemoryKind};

use crate::{
    diff::{diff, Diff, FieldDiff},
    gc::GcPtr,
    type_info::{ArrayInfo, TypeInfo},
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
#[derive(Debug)]
pub struct FieldMapping {
    pub new_ty: Arc<TypeInfo>,
    pub new_offset: usize,
    pub action: Action,
}

/// The `Action` to take when mapping memory from A to B.
#[derive(Debug, Eq, PartialEq)]
pub enum Action {
    ArrayAlloc,
    ArrayFromValue {
        element_action: Box<Action>,
        old_offset: usize,
    },
    ArrayMap {
        element_action: Box<Action>,
        old_offset: usize,
    },
    Cast {
        old_ty: Arc<TypeInfo>,
        old_offset: usize,
    },
    Copy {
        old_offset: usize,
        /// Size in bytes
        size: usize,
    },
    ElementFromArray {
        element_action: Box<Action>,
        old_offset: usize,
    },
    StructAlloc,
    StructMapFromGc {
        old_ty: Arc<TypeInfo>,
        old_offset: usize,
    },
    StructMapFromValue {
        old_ty: Arc<TypeInfo>,
        old_offset: usize,
    },
    StructMapInPlace {
        old_ty: Arc<TypeInfo>,
        old_offset: usize,
    },
    ZeroInitialize,
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
            FieldDiff::Edit { old_index, .. } => old_index.clone(),
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
                    old_offset: usize::from(old_field.offset),
                    size: if old_field.type_info.is_stack_allocated() {
                        old_field.type_info.layout.size()
                    } else {
                        std::mem::size_of::<GcPtr>()
                    },
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
                    .map(|field| usize::from(field.offset))
                    .expect("The old field must exist.");
                println!("EDIT");
                (*new_index, resolve_edit(old_type, new_type, old_offset))
            }),
            FieldDiff::Insert { index, new_type } => Some((
                *index,
                if new_type.is_struct() && !new_type.is_stack_allocated() {
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
                    .map(|field| usize::from(field.offset))
                    .expect("Old field must exist.");

                Some((
                    *new_index,
                    Action::Copy {
                        old_offset,
                        size: if ty.is_stack_allocated() {
                            ty.layout.size()
                        } else {
                            std::mem::size_of::<GcPtr>()
                        },
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
                .map(|field| usize::from(field.offset))
                .expect("The old field must exist.");

            let action = mapping.get_mut(*new_index).unwrap();
            *action = resolve_edit(old_type, new_type, old_offset);
        }
    }

    let new_fields = new_ty.fields();
    Conversion {
        field_mapping: mapping
            .into_iter()
            .enumerate()
            .map(|(new_index, action)| {
                let new_field = new_fields
                    .get(new_index)
                    .expect(format!("New field at index: '{}' must exist.", new_index).as_str());
                FieldMapping {
                    new_ty: new_field.type_info.clone(),
                    new_offset: usize::from(new_field.offset),
                    action: action,
                }
            })
            .collect(),
        new_ty: new_ty.clone(),
    }
}

pub fn resolve_edit(old_ty: &Arc<TypeInfo>, new_ty: &TypeInfo, old_offset: usize) -> Action {
    match &old_ty.data {
        crate::TypeInfoData::Primitive(old_guid) => {
            resolve_primitive_edit(old_ty, new_ty, old_guid, old_offset)
        }
        crate::TypeInfoData::Struct(_) => resolve_struct_edit(old_ty, new_ty, old_offset),
        crate::TypeInfoData::Pointer(_) => resolve_pointer_edit(old_ty, new_ty),
        crate::TypeInfoData::Array(old_array) => resolve_array_edit(old_array, new_ty, old_offset),
    }
}

fn resolve_primitive_edit(
    old_ty: &Arc<TypeInfo>,
    new_ty: &TypeInfo,
    old_guid: &Guid,
    old_offset: usize,
) -> Action {
    match &new_ty.data {
        crate::TypeInfoData::Primitive(new_guid) => {
            resolve_primitive_to_primitive_edit(old_ty, old_guid, old_offset, new_guid)
        }
        crate::TypeInfoData::Struct(s) => {
            if s.memory_kind == StructMemoryKind::Value {
                Action::ZeroInitialize
            } else {
                Action::StructAlloc
            }
        }
        crate::TypeInfoData::Pointer(_) => unreachable!(),
        crate::TypeInfoData::Array(new_array) => {
            resolve_primitive_to_array_edit(old_ty, new_array, old_offset)
        }
    }
}

fn resolve_primitive_to_primitive_edit(
    old_ty: &Arc<TypeInfo>,
    old_guid: &Guid,
    old_offset: usize,
    new_guid: &Guid,
) -> Action {
    if *old_guid == *new_guid {
        Action::Copy {
            old_offset,
            size: old_ty.layout.size(),
        }
    } else {
        Action::Cast {
            old_ty: old_ty.clone(),
            old_offset,
        }
    }
}

fn resolve_primitive_to_array_edit(
    old_ty: &Arc<TypeInfo>,
    new_array: &ArrayInfo,
    old_offset: usize,
) -> Action {
    Action::ArrayFromValue {
        element_action: Box::new(resolve_edit(old_ty, &new_array.element_ty, 0)),
        old_offset,
    }
}

fn resolve_struct_edit(old_ty: &Arc<TypeInfo>, new_ty: &TypeInfo, old_offset: usize) -> Action {
    match &new_ty.data {
        crate::TypeInfoData::Primitive(_) => Action::ZeroInitialize,
        crate::TypeInfoData::Struct(_) => resolve_struct_to_struct_edit(old_ty, new_ty, old_offset),
        crate::TypeInfoData::Pointer(_) => unreachable!(),
        crate::TypeInfoData::Array(new_array) => {
            resolve_struct_to_array_edit(old_ty, new_array, old_offset)
        }
    }
}

fn resolve_struct_to_struct_edit(
    old_ty: &Arc<TypeInfo>,
    new_ty: &TypeInfo,
    old_offset: usize,
) -> Action {
    // Early opt-out for when we are recursively resolving types (e.g. for arrays)
    if **old_ty == *new_ty {
        return Action::Copy {
            old_offset: old_offset,
            size: if old_ty.is_stack_allocated() {
                old_ty.layout.size()
            } else {
                std::mem::size_of::<GcPtr>()
            },
        };
    }

    // ASSUMPTION: When the name is the same, we are dealing with the same struct,
    // but different internals
    let is_same_struct = old_ty.name == new_ty.name;

    if old_ty.is_stack_allocated() && new_ty.is_stack_allocated() {
        // struct(value) -> struct(value)
        if is_same_struct {
            Action::StructMapInPlace {
                old_ty: old_ty.clone(),
                old_offset,
            }
        } else {
            Action::ZeroInitialize
        }
    } else if old_ty.is_stack_allocated() {
        // struct(value) -> struct(gc)
        if is_same_struct {
            Action::StructMapFromValue {
                old_ty: old_ty.clone(),
                old_offset,
            }
        } else {
            Action::StructAlloc
        }
    } else if new_ty.is_stack_allocated() {
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

fn resolve_struct_to_array_edit(
    old_ty: &Arc<TypeInfo>,
    new_array: &ArrayInfo,
    old_offset: usize,
) -> Action {
    Action::ArrayFromValue {
        element_action: Box::new(resolve_edit(old_ty, &new_array.element_ty, 0)),
        old_offset,
    }
}

fn resolve_pointer_edit(_old_ty: &Arc<TypeInfo>, _new_ty: &TypeInfo) -> Action {
    // Not supported in the language - yet
    unreachable!()
}

fn resolve_array_edit(old_array: &ArrayInfo, new_ty: &TypeInfo, old_offset: usize) -> Action {
    match &new_ty.data {
        crate::TypeInfoData::Primitive(_) => {
            resolve_array_to_primitive_edit(old_array, new_ty, old_offset)
        }
        crate::TypeInfoData::Struct(_) => {
            resolve_array_to_struct_edit(old_array, new_ty, old_offset)
        }
        crate::TypeInfoData::Pointer(_) => unreachable!(),
        crate::TypeInfoData::Array(new_array) => {
            resolve_array_to_array_edit(old_array, new_array, old_offset)
        }
    }
}

fn resolve_array_to_primitive_edit(
    old_array: &ArrayInfo,
    new_ty: &TypeInfo,
    old_offset: usize,
) -> Action {
    Action::ElementFromArray {
        old_offset,
        element_action: Box::new(resolve_edit(&old_array.element_ty, new_ty, 0)),
    }
}

fn resolve_array_to_struct_edit(
    old_array: &ArrayInfo,
    new_ty: &TypeInfo,
    old_offset: usize,
) -> Action {
    Action::ElementFromArray {
        old_offset,
        element_action: Box::new(resolve_edit(&old_array.element_ty, new_ty, 0)),
    }
}

fn resolve_array_to_array_edit(
    old_array: &ArrayInfo,
    new_array: &ArrayInfo,
    old_offset: usize,
) -> Action {
    if *old_array.element_ty == *new_array.element_ty {
        println!("THE SAME");
        Action::Copy {
            old_offset,
            size: std::mem::size_of::<GcPtr>(),
        }
    } else {
        println!("ARRAY MAP");
        Action::ArrayMap {
            element_action: Box::new(resolve_edit(
                &old_array.element_ty,
                &new_array.element_ty,
                0,
            )),
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
