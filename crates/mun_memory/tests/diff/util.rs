#![allow(dead_code)]
use mun_memory::{
    diff::{myers, Diff, FieldDiff, FieldEditKind},
    TypeDesc, TypeFields, TypeLayout,
};
use std::alloc::Layout;

pub const STRUCT1_NAME: &str = "struct1";
pub const STRUCT1_GUID: abi::Guid = abi::Guid {
    b: [
        0, 10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 110, 120, 130, 140, 150,
    ],
};
pub const STRUCT2_NAME: &str = "struct2";
pub const STRUCT2_GUID: abi::Guid = abi::Guid {
    b: [
        150, 140, 130, 120, 110, 100, 90, 80, 70, 60, 50, 40, 30, 20, 10, 0,
    ],
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructInfo {
    fields: Vec<(String, TypeInfo)>,
}

impl StructInfo {
    pub fn new(fields: &[(&str, &TypeInfo)]) -> Self {
        Self {
            fields: fields
                .iter()
                .map(|(name, ty)| (name.to_string(), (*ty).clone()))
                .collect(),
        }
    }

    pub fn layout(&self) -> Layout {
        // NOTE: This implementation is naive, but it is merely a test
        let size = self.fields.iter().map(|ty| ty.1.layout.size()).sum();
        let align = self
            .fields
            .iter()
            .map(|(_, ty)| ty.layout.align())
            .max()
            .unwrap_or(1);
        unsafe { Layout::from_size_align_unchecked(size, align) }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TypeInfoTail {
    Empty,
    Struct(StructInfo),
}

#[derive(Clone, Debug)]
pub struct TypeInfo {
    pub name: String,
    pub guid: abi::Guid,
    pub group: abi::TypeGroup,
    pub layout: Layout,
    pub tail: TypeInfoTail,
}

impl TypeInfo {
    pub fn new_fundamental<T: abi::HasStaticTypeInfo>() -> Self {
        let type_info = T::type_info();
        Self {
            name: type_info.name().to_string(),
            guid: type_info.guid,
            group: abi::TypeGroup::FundamentalTypes,
            layout: Layout::new::<T>(),
            tail: TypeInfoTail::Empty,
        }
    }

    pub fn new_struct(name: &str, guid: abi::Guid, struct_info: StructInfo) -> Self {
        Self {
            name: name.to_string(),
            guid,
            group: abi::TypeGroup::StructTypes,
            layout: struct_info.layout(),
            tail: TypeInfoTail::Struct(struct_info),
        }
    }
}

// TODO: Change Guid to be a hash of field names and field types. For fundamental types, their
// singular field type (e.g. u8, i16, f32) is used. Order of fields is important!
impl PartialEq for TypeInfo {
    fn eq(&self, other: &Self) -> bool {
        self.guid == other.guid
    }
}

impl Eq for TypeInfo {}

impl TypeDesc for &TypeInfo {
    fn name(&self) -> &str {
        &self.name
    }
    fn guid(&self) -> &abi::Guid {
        &self.guid
    }
    fn group(&self) -> abi::TypeGroup {
        self.group
    }
}

impl TypeLayout for &TypeInfo {
    fn layout(&self) -> Layout {
        self.layout
    }
}

impl<'t> TypeFields<&'t TypeInfo> for &'t TypeInfo {
    fn fields(&self) -> Vec<(&str, Self)> {
        match &self.tail {
            TypeInfoTail::Empty => Vec::new(),
            TypeInfoTail::Struct(s) => s
                .fields
                .iter()
                .map(|(name, ty)| (name.as_str(), ty))
                .collect(),
        }
    }

    fn offsets(&self) -> &[u16] {
        // This is a stub, as we don't do any actual memory mapping
        &[]
    }
}

pub fn apply_myers_diff<'t, T: Copy + Eq>(old: &[T], new: &[T], diff: Vec<myers::Diff>) -> Vec<T> {
    let mut combined: Vec<_> = old.iter().cloned().collect();
    for diff in diff.iter().rev() {
        if let myers::Diff::Delete { index } = diff {
            combined.remove(*index);
        }
    }
    for diff in diff {
        if let myers::Diff::Insert { index } = diff {
            let value = unsafe { new.get_unchecked(index) };
            combined.insert(index, *value);
        }
    }
    combined
}

pub(crate) fn apply_diff<'t>(
    old: &[&TypeInfo],
    new: &[&TypeInfo],
    diff: Vec<Diff>,
) -> Vec<TypeInfo> {
    let mut combined: Vec<TypeInfo> = old.iter().map(|ty| (*ty).clone()).collect();
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
                apply_mapping(old_ty, new_ty, diff);
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

fn apply_mapping<'t>(old: &mut TypeInfo, new: &TypeInfo, mapping: &[FieldDiff]) {
    if let TypeInfoTail::Struct(old_struct) = &mut old.tail {
        if let TypeInfoTail::Struct(new_struct) = &new.tail {
            let mut combined: Vec<_> = old_struct.fields.iter().cloned().collect();
            for diff in mapping.iter().rev() {
                match diff {
                    FieldDiff::Delete { index } => {
                        combined.remove(*index);
                    }

                    FieldDiff::Move { old_index, .. } => {
                        combined.remove(*old_index);
                    }
                    _ => (),
                }
            }

            fn get_new_index(diff: &FieldDiff) -> usize {
                match diff {
                    FieldDiff::Insert { index } => *index,
                    FieldDiff::Move { new_index, .. } => *new_index,
                    _ => std::usize::MAX,
                }
            }

            // Sort elements in ascending order of their insertion indices.
            let mut additions: Vec<(usize, _)> = mapping
                .iter()
                .filter_map(|diff| match diff {
                    FieldDiff::Insert { index } => Some((
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
                combined.insert(index, field);
            }

            fn edit_field(
                kind: &FieldEditKind,
                old_field: &mut (String, TypeInfo),
                new_field: &(String, TypeInfo),
            ) {
                match *kind {
                    FieldEditKind::ConvertType => panic!("Casting is currently not supported"),
                    FieldEditKind::Rename => {
                        old_field.0 = new_field.0.clone();
                    }
                }
            }

            // Handle edits
            for diff in mapping.iter() {
                match diff {
                    FieldDiff::Edit { index, kind } => edit_field(
                        kind,
                        unsafe { combined.get_unchecked_mut(*index) },
                        unsafe { new_struct.fields.get_unchecked(*index) },
                    ),
                    FieldDiff::Move {
                        old_index,
                        new_index,
                        edit: Some(kind),
                    } => edit_field(
                        kind,
                        unsafe { combined.get_unchecked_mut(*old_index) },
                        unsafe { new_struct.fields.get_unchecked(*new_index) },
                    ),
                    _ => (),
                }
            }

            old_struct.fields = combined;
            old.layout = old_struct.layout();
        } else {
            unreachable!()
        }
    } else {
        unreachable!()
    }
}
