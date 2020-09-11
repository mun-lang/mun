use super::Module;
use crate::{
    arena::{Arena, Idx},
    ids::{Lookup, StructId},
    name::AsName,
    name_resolution::Namespace,
    ty::lower::LowerBatchResult,
    type_ref::{LocalTypeRefId, TypeRefBuilder, TypeRefMap, TypeRefSourceMap},
    DefDatabase, DiagnosticSink, FileId, HasVisibility, HirDatabase, Name, Ty, Visibility,
};
use mun_syntax::{
    ast,
    ast::{NameOwner, TypeAscriptionOwner},
};
use std::{fmt, sync::Arc};

use crate::resolve::HasResolver;
use crate::visibility::RawVisibility;
pub use ast::StructMemoryKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Struct {
    pub(crate) id: StructId,
}

impl From<StructId> for Struct {
    fn from(id: StructId) -> Self {
        Struct { id }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StructField {
    pub(crate) parent: Struct,
    pub(crate) id: LocalStructFieldId,
}

impl StructField {
    pub fn ty(self, db: &dyn HirDatabase) -> Ty {
        let data = self.parent.data(db.upcast());
        let type_ref_id = data.fields[self.id].type_ref;
        let lower = self.parent.lower(db);
        lower[type_ref_id].clone()
    }

    pub fn name(self, db: &dyn HirDatabase) -> Name {
        self.parent.data(db.upcast()).fields[self.id].name.clone()
    }

    pub fn id(self) -> LocalStructFieldId {
        self.id
    }
}

impl Struct {
    pub fn module(self, db: &dyn HirDatabase) -> Module {
        Module {
            id: self.id.lookup(db.upcast()).module,
        }
    }

    pub fn file_id(self, db: &dyn HirDatabase) -> FileId {
        self.id.lookup(db.upcast()).id.file_id
    }

    pub fn data(self, db: &dyn DefDatabase) -> Arc<StructData> {
        db.struct_data(self.id)
    }

    pub fn name(self, db: &dyn HirDatabase) -> Name {
        self.data(db.upcast()).name.clone()
    }

    pub fn fields(self, db: &dyn HirDatabase) -> Vec<StructField> {
        self.data(db.upcast())
            .fields
            .iter()
            .map(|(id, _)| StructField { parent: self, id })
            .collect()
    }

    pub fn field(self, db: &dyn HirDatabase, name: &Name) -> Option<StructField> {
        self.data(db.upcast())
            .fields
            .iter()
            .find(|(_, data)| data.name == *name)
            .map(|(id, _)| StructField { parent: self, id })
    }

    pub fn ty(self, db: &dyn HirDatabase) -> Ty {
        // TODO: Add detection of cyclick types
        db.type_for_def(self.into(), Namespace::Types).0
    }

    pub fn lower(self, db: &dyn HirDatabase) -> Arc<LowerBatchResult> {
        db.lower_struct(self)
    }

    pub fn diagnostics(self, db: &dyn HirDatabase, sink: &mut DiagnosticSink) {
        let data = self.data(db.upcast());
        let lower = self.lower(db);
        lower.add_diagnostics(db, self.file_id(db), data.type_ref_source_map(), sink);
    }
}

/// A single field of a record
/// ```mun
/// struct Foo {
///     a: int, // <- this
/// }
/// ```
/// or
/// ```mun
/// struct Foo(
///     int, // <- this
/// )
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructFieldData {
    pub name: Name,
    pub type_ref: LocalTypeRefId,
}

/// A struct's fields' data (record, tuple, or unit struct)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StructKind {
    Record,
    Tuple,
    Unit,
}

impl fmt::Display for StructKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            StructKind::Record => write!(f, "record"),
            StructKind::Tuple => write!(f, "tuple"),
            StructKind::Unit => write!(f, "unit struct"),
        }
    }
}

/// An identifier for a struct's or tuple's field
pub type LocalStructFieldId = Idx<StructFieldData>;

#[derive(Debug, PartialEq, Eq)]
pub struct StructData {
    pub name: Name,
    pub visibility: RawVisibility,
    pub fields: Arena<StructFieldData>,
    pub kind: StructKind,
    pub memory_kind: StructMemoryKind,
    type_ref_map: TypeRefMap,
    type_ref_source_map: TypeRefSourceMap,
}

impl StructData {
    pub(crate) fn struct_data_query(db: &dyn DefDatabase, id: StructId) -> Arc<StructData> {
        let loc = id.lookup(db);
        let item_tree = db.item_tree(loc.id.file_id);
        let strukt = &item_tree[loc.id.value];
        let src = item_tree.source(db, loc.id.value);

        let memory_kind = src
            .memory_type_specifier()
            .map(|s| s.kind())
            .unwrap_or_default();

        let mut type_ref_builder = TypeRefBuilder::default();
        let (fields, kind) = match src.kind() {
            ast::StructKind::Record(r) => {
                let fields = r
                    .fields()
                    .map(|fd| StructFieldData {
                        name: fd.name().map(|n| n.as_name()).unwrap_or_else(Name::missing),
                        type_ref: type_ref_builder.alloc_from_node_opt(fd.ascribed_type().as_ref()),
                    })
                    .collect();
                (fields, StructKind::Record)
            }
            ast::StructKind::Tuple(t) => {
                let fields = t
                    .fields()
                    .enumerate()
                    .map(|(index, fd)| StructFieldData {
                        name: Name::new_tuple_field(index),
                        type_ref: type_ref_builder.alloc_from_node_opt(fd.type_ref().as_ref()),
                    })
                    .collect();
                (fields, StructKind::Tuple)
            }
            ast::StructKind::Unit => (Arena::default(), StructKind::Unit),
        };

        let visibility = item_tree[strukt.visibility].clone();

        let (type_ref_map, type_ref_source_map) = type_ref_builder.finish();
        Arc::new(StructData {
            name: strukt.name.clone(),
            visibility,
            fields,
            kind,
            memory_kind,
            type_ref_map,
            type_ref_source_map,
        })
    }

    pub fn type_ref_source_map(&self) -> &TypeRefSourceMap {
        &self.type_ref_source_map
    }

    pub fn type_ref_map(&self) -> &TypeRefMap {
        &self.type_ref_map
    }
}

impl HasVisibility for Struct {
    fn visibility(&self, db: &dyn HirDatabase) -> Visibility {
        self.data(db.upcast())
            .visibility
            .resolve(db.upcast(), &self.id.resolver(db.upcast()))
    }
}
