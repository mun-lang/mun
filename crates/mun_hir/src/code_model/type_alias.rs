use std::sync::Arc;

use crate::{
    ids::Lookup,
    ids::TypeAliasId,
    type_ref::{LocalTypeRefId, TypeRefMap, TypeRefSourceMap},
    visibility::RawVisibility,
    DefDatabase, DiagnosticSink, FileId, HasVisibility, HirDatabase, Name, Ty, TyKind, Visibility,
};

use super::Module;
use crate::expr::validator::TypeAliasValidator;
use crate::resolve::HasResolver;
use crate::ty::lower::LowerTyMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeAlias {
    pub(crate) id: TypeAliasId,
}

impl From<TypeAliasId> for TypeAlias {
    fn from(id: TypeAliasId) -> Self {
        TypeAlias { id }
    }
}

impl TypeAlias {
    pub fn module(self, db: &dyn HirDatabase) -> Module {
        Module {
            id: self.id.lookup(db.upcast()).module,
        }
    }

    pub fn file_id(self, db: &dyn HirDatabase) -> FileId {
        self.id.lookup(db.upcast()).id.file_id
    }

    pub fn data(self, db: &dyn DefDatabase) -> Arc<TypeAliasData> {
        db.type_alias_data(self.id)
    }

    pub fn name(self, db: &dyn HirDatabase) -> Name {
        self.data(db.upcast()).name.clone()
    }

    pub fn type_ref(self, db: &dyn HirDatabase) -> LocalTypeRefId {
        self.data(db.upcast()).type_ref_id
    }

    pub fn lower(self, db: &dyn HirDatabase) -> Arc<LowerTyMap> {
        db.lower_type_alias(self)
    }

    pub fn target_type(self, db: &dyn HirDatabase) -> Ty {
        let data = self.data(db.upcast());
        let mut ty = Ty::from_hir(
            db,
            &self.id.resolver(db.upcast()),
            data.type_ref_map(),
            data.type_ref_id,
        )
        .0;

        while let &TyKind::TypeAlias(alias) = ty.interned() {
            let data = alias.data(db.upcast());
            ty = Ty::from_hir(
                db,
                &alias.id.resolver(db.upcast()),
                data.type_ref_map(),
                data.type_ref_id,
            )
            .0;
        }

        ty
    }

    pub fn diagnostics(self, db: &dyn HirDatabase, sink: &mut DiagnosticSink) {
        let data = self.data(db.upcast());
        let lower = self.lower(db);
        lower.add_diagnostics(db, self.file_id(db), data.type_ref_source_map(), sink);

        let validator = TypeAliasValidator::new(self, db);
        validator.validate_target_type_existence(sink);
        validator.validate_target_type_privacy(sink);
        validator.validate_acyclic(sink);
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct TypeAliasData {
    pub name: Name,
    pub visibility: RawVisibility,
    pub type_ref_id: LocalTypeRefId,
    type_ref_map: TypeRefMap,
    type_ref_source_map: TypeRefSourceMap,
}
impl TypeAliasData {
    pub(crate) fn type_alias_data_query(
        db: &dyn DefDatabase,
        id: TypeAliasId,
    ) -> Arc<TypeAliasData> {
        let loc = id.lookup(db);
        let item_tree = db.item_tree(loc.id.file_id);
        let alias = &item_tree[loc.id.value];
        let src = item_tree.source(db, loc.id.value);
        let mut type_ref_builder = TypeRefMap::builder();
        let type_ref_opt = src.type_ref();
        let type_ref_id = type_ref_builder.alloc_from_node_opt(type_ref_opt.as_ref());
        let (type_ref_map, type_ref_source_map) = type_ref_builder.finish();
        Arc::new(TypeAliasData {
            name: alias.name.clone(),
            visibility: item_tree[alias.visibility].clone(),
            type_ref_id,
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

impl HasVisibility for TypeAlias {
    fn visibility(&self, db: &dyn HirDatabase) -> Visibility {
        self.data(db.upcast())
            .visibility
            .resolve(db.upcast(), &self.id.resolver(db.upcast()))
    }
}
