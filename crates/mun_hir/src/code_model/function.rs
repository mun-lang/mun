use std::{iter::once, sync::Arc};

use mun_syntax::{ast, ast::TypeAscriptionOwner};

use super::Module;
use crate::{
    expr::{validator::ExprValidator, BodySourceMap},
    has_module::HasModule,
    ids::{FunctionId, Lookup},
    name_resolution::Namespace,
    resolve::HasResolver,
    type_ref::{LocalTypeRefId, TypeRefMap, TypeRefSourceMap},
    visibility::RawVisibility,
    Body, DefDatabase, DiagnosticSink, FileId, HasSource, HasVisibility, HirDatabase, InFile,
    InferenceResult, Name, Ty, Visibility,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct Function {
    pub(crate) id: FunctionId,
}

impl From<FunctionId> for Function {
    fn from(id: FunctionId) -> Self {
        Function { id }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct FunctionData {
    name: Name,
    params: Vec<LocalTypeRefId>,
    visibility: RawVisibility,
    ret_type: LocalTypeRefId,
    type_ref_map: TypeRefMap,
    type_ref_source_map: TypeRefSourceMap,
    is_extern: bool,
}

impl FunctionData {
    pub(crate) fn fn_data_query(db: &dyn DefDatabase, func: FunctionId) -> Arc<FunctionData> {
        let loc = func.lookup(db);
        let item_tree = db.item_tree(loc.id.file_id);
        let func = &item_tree[loc.id.value];
        let src = item_tree.source(db, loc.id.value);

        let mut type_ref_builder = TypeRefMap::builder();

        let mut params = Vec::new();
        if let Some(param_list) = src.param_list() {
            for param in param_list.params() {
                let type_ref = type_ref_builder.alloc_from_node_opt(param.ascribed_type().as_ref());
                params.push(type_ref);
            }
        }

        let ret_type = if let Some(type_ref) = src.ret_type().and_then(|rt| rt.type_ref()) {
            type_ref_builder.alloc_from_node(&type_ref)
        } else {
            type_ref_builder.unit()
        };

        let (type_ref_map, type_ref_source_map) = type_ref_builder.finish();

        Arc::new(FunctionData {
            name: func.name.clone(),
            params,
            ret_type,
            type_ref_map,
            type_ref_source_map,
            is_extern: func.is_extern,
            visibility: item_tree[func.visibility].clone(),
        })
    }

    pub fn name(&self) -> &Name {
        &self.name
    }

    pub fn params(&self) -> &[LocalTypeRefId] {
        &self.params
    }

    pub fn visibility(&self) -> &RawVisibility {
        &self.visibility
    }

    pub fn ret_type(&self) -> &LocalTypeRefId {
        &self.ret_type
    }

    pub fn type_ref_source_map(&self) -> &TypeRefSourceMap {
        &self.type_ref_source_map
    }

    pub fn type_ref_map(&self) -> &TypeRefMap {
        &self.type_ref_map
    }
}

impl Function {
    pub fn module(self, db: &dyn HirDatabase) -> Module {
        self.id.module(db.upcast()).into()
    }

    /// Returns the full name of the function including all module specifiers
    /// (e.g: `foo::bar`).
    pub fn full_name(self, db: &dyn HirDatabase) -> String {
        itertools::Itertools::intersperse(
            self.module(db)
                .path_to_root(db)
                .into_iter()
                .filter_map(|module| module.name(db))
                .chain(once(self.name(db)))
                .map(|name| name.to_string()),
            String::from("::"),
        )
        .collect()
    }

    pub fn file_id(self, db: &dyn HirDatabase) -> FileId {
        self.id.lookup(db.upcast()).id.file_id
    }

    pub fn name(self, db: &dyn HirDatabase) -> Name {
        self.data(db.upcast()).name.clone()
    }

    pub fn data(self, db: &dyn DefDatabase) -> Arc<FunctionData> {
        db.fn_data(self.id)
    }

    pub fn body(self, db: &dyn HirDatabase) -> Arc<Body> {
        db.body(self.id.into())
    }

    pub fn ty(self, db: &dyn HirDatabase) -> Ty {
        db.type_for_def(self.into(), Namespace::Values)
    }

    /// Returns the parameters of the function.
    pub fn params(self, db: &dyn HirDatabase) -> Vec<Param> {
        db.callable_sig(self.into())
            .params()
            .iter()
            .enumerate()
            .map(|(idx, ty)| Param {
                func: self,
                ty: ty.clone(),
                idx,
            })
            .collect()
    }

    pub fn ret_type(self, db: &dyn HirDatabase) -> Ty {
        let resolver = self.id.resolver(db.upcast());
        let data = self.data(db.upcast());
        Ty::from_hir(db, &resolver, &data.type_ref_map, data.ret_type).0
    }

    pub fn infer(self, db: &dyn HirDatabase) -> Arc<InferenceResult> {
        db.infer(self.id.into())
    }

    pub fn is_extern(self, db: &dyn HirDatabase) -> bool {
        db.fn_data(self.id).is_extern
    }

    pub(crate) fn body_source_map(self, db: &dyn HirDatabase) -> Arc<BodySourceMap> {
        db.body_with_source_map(self.id.into()).1
    }

    pub fn diagnostics(self, db: &dyn HirDatabase, sink: &mut DiagnosticSink<'_>) {
        let body = self.body(db);
        body.add_diagnostics(db, self.into(), sink);
        let infer = self.infer(db);
        infer.add_diagnostics(db, self, sink);
        let validator = ExprValidator::new(self, db);
        validator.validate_body(sink);
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Param {
    func: Function,
    /// The index in parameter list, including self parameter.
    idx: usize,
    ty: Ty,
}

impl Param {
    /// Returns the function to which this parameter belongs
    pub fn parent_fn(&self) -> Function {
        self.func
    }

    /// Returns the index of this parameter in the parameter list (including
    /// self)
    pub fn index(&self) -> usize {
        self.idx
    }

    /// Returns the type of this parameter.
    pub fn ty(&self) -> &Ty {
        &self.ty
    }

    /// Returns the source of the parameter.
    pub fn source(&self, db: &dyn HirDatabase) -> Option<InFile<ast::Param>> {
        let InFile { file_id, value } = self.func.source(db.upcast());
        let params = value.param_list()?;
        params
            .params()
            .nth(self.idx)
            .map(|value| InFile { file_id, value })
    }
}

impl HasVisibility for Function {
    fn visibility(&self, db: &dyn HirDatabase) -> Visibility {
        self.data(db.upcast())
            .visibility
            .resolve(db.upcast(), &self.id.resolver(db.upcast()))
    }
}
