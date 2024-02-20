mod function;
mod r#impl;
mod module;
mod package;
pub(crate) mod src;
pub(crate) mod r#struct;
mod type_alias;

use std::sync::Arc;

pub use self::{
    function::{Function, FunctionData},
    module::{Module, ModuleDef},
    package::Package,
    r#impl::ImplData,
    r#struct::{Field, Struct, StructData, StructKind, StructMemoryKind},
    src::HasSource,
    type_alias::{TypeAlias, TypeAliasData},
};
use crate::{expr::BodySourceMap, HirDatabase, Name};

/// The definitions that have a body.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DefWithBody {
    Function(Function),
}
impl_froms!(DefWithBody: Function);

impl DefWithBody {
    pub fn module(self, db: &dyn HirDatabase) -> Module {
        match self {
            DefWithBody::Function(f) => f.module(db),
        }
    }

    pub fn body_source_map(self, db: &dyn HirDatabase) -> Arc<BodySourceMap> {
        match self {
            DefWithBody::Function(f) => f.body_source_map(db),
        }
    }
}

/// Definitions that have a struct.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DefWithStruct {
    Struct(Struct),
}
impl_froms!(DefWithStruct: Struct);

impl DefWithStruct {
    pub fn module(self, db: &dyn HirDatabase) -> Module {
        match self {
            DefWithStruct::Struct(s) => s.module(db),
        }
    }

    pub fn fields(self, db: &dyn HirDatabase) -> Vec<Field> {
        match self {
            DefWithStruct::Struct(s) => s.fields(db),
        }
    }

    pub fn field(self, db: &dyn HirDatabase, name: &Name) -> Option<Field> {
        match self {
            DefWithStruct::Struct(s) => s.field(db, name),
        }
    }

    pub fn data(self, db: &dyn HirDatabase) -> Arc<StructData> {
        match self {
            DefWithStruct::Struct(s) => s.data(db.upcast()),
        }
    }
}
