pub(crate) mod src;

use self::src::HasSource;
use crate::diagnostics::DiagnosticSink;
use crate::expr::{Body, BodySourceMap};
use crate::ids::AstItemDef;
use crate::ids::LocationCtx;
use crate::name_resolution::Namespace;
use crate::raw::{DefKind, RawFileItem};
use crate::resolve::{Resolution, Resolver};
use crate::ty::InferenceResult;
use crate::type_ref::{TypeRefBuilder, TypeRefId, TypeRefMap, TypeRefSourceMap};
use crate::{ids::FunctionId, AsName, DefDatabase, FileId, HirDatabase, Name, Ty};
use mun_syntax::ast::{NameOwner, TypeAscriptionOwner, VisibilityOwner};
use rustc_hash::FxHashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Module {
    pub(crate) file_id: FileId,
}

impl From<FileId> for Module {
    fn from(file_id: FileId) -> Self {
        Module { file_id }
    }
}

impl Module {
    pub fn file_id(self) -> FileId {
        self.file_id
    }

    /// Returns all the definitions declared in this module.
    pub fn declarations(self, db: &impl HirDatabase) -> Vec<ModuleDef> {
        db.module_data(self.file_id).definitions.clone()
    }

    fn resolver(self, _db: &impl DefDatabase) -> Resolver {
        Resolver::default().push_module_scope(self.file_id)
    }

    pub fn diagnostics(self, db: &impl HirDatabase, sink: &mut DiagnosticSink) {
        for diag in db.module_data(self.file_id).diagnostics.iter() {
            diag.add_to(db, self, sink);
        }
        for decl in self.declarations(db) {
            #[allow(clippy::single_match)]
            match decl {
                ModuleDef::Function(f) => f.diagnostics(db, sink),
                _ => (),
            }
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Default)]
pub struct ModuleData {
    definitions: Vec<ModuleDef>,
    diagnostics: Vec<ModuleDefinitionDiagnostic>,
}

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct ModuleScope {
    items: FxHashMap<Name, Resolution>,
}

impl ModuleData {
    pub(crate) fn module_data_query(db: &impl DefDatabase, file_id: FileId) -> Arc<ModuleData> {
        let items = db.raw_items(file_id);
        let mut data = ModuleData::default();
        let loc_ctx = LocationCtx::new(db, file_id);
        let mut definition_by_name = FxHashMap::default();
        for item in items.items().iter() {
            match item {
                RawFileItem::Definition(def) => {
                    if let Some(prev_definition) = definition_by_name.get(&items[*def].name) {
                        data.diagnostics.push(
                            diagnostics::ModuleDefinitionDiagnostic::DuplicateName {
                                name: items[*def].name.clone(),
                                definition: *def,
                                first_definition: *prev_definition,
                            },
                        )
                    } else {
                        definition_by_name.insert(items[*def].name.clone(), *def);
                    }
                    match items[*def].kind {
                        DefKind::Function(ast_id) => {
                            data.definitions.push(ModuleDef::Function(Function {
                                id: FunctionId::from_ast_id(loc_ctx, ast_id),
                            }))
                        }
                    }
                }
            };
        }
        Arc::new(data)
    }

    pub fn definitions(&self) -> &[ModuleDef] {
        &self.definitions
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModuleDef {
    Function(Function),
    BuiltinType(BuiltinType),
}

impl From<Function> for ModuleDef {
    fn from(t: Function) -> Self {
        ModuleDef::Function(t)
    }
}

impl From<BuiltinType> for ModuleDef {
    fn from(t: BuiltinType) -> Self {
        ModuleDef::BuiltinType(t)
    }
}

/// The definitions that have a body.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DefWithBody {
    Function(Function),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Public,
    Private,
}

impl DefWithBody {
    pub fn infer(self, db: &impl HirDatabase) -> Arc<InferenceResult> {
        db.infer(self)
    }

    pub fn body(self, db: &impl HirDatabase) -> Arc<Body> {
        db.body_hir(self)
    }

    pub fn body_source_map(self, db: &impl HirDatabase) -> Arc<BodySourceMap> {
        db.body_with_source_map(self).1
    }

    /// Builds a `Resolver` for code inside this item. A `Resolver` enables name resolution.
    pub(crate) fn resolver(self, db: &impl HirDatabase) -> Resolver {
        match self {
            DefWithBody::Function(f) => f.resolver(db),
        }
    }
}

impl From<Function> for DefWithBody {
    fn from(f: Function) -> Self {
        DefWithBody::Function(f)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Function {
    pub(crate) id: FunctionId,
}

#[derive(Debug, PartialEq, Eq)]
pub struct FnData {
    name: Name,
    params: Vec<TypeRefId>,
    visibility: Visibility,
    ret_type: TypeRefId,
    type_ref_map: TypeRefMap,
    type_ref_source_map: TypeRefSourceMap,
}

impl FnData {
    pub(crate) fn fn_data_query(db: &impl DefDatabase, func: Function) -> Arc<FnData> {
        let src = func.source(db);
        let mut type_ref_builder = TypeRefBuilder::default();
        let name = src
            .ast
            .name()
            .map(|n| n.as_name())
            .unwrap_or_else(Name::missing);

        let visibility = src
            .ast
            .visibility()
            .map(|_v| Visibility::Public)
            .unwrap_or(Visibility::Private);

        let mut params = Vec::new();
        if let Some(param_list) = src.ast.param_list() {
            for param in param_list.params() {
                let type_ref = type_ref_builder.alloc_from_node_opt(param.ascribed_type().as_ref());
                params.push(type_ref);
            }
        }

        let ret_type = if let Some(type_ref) = src.ast.ret_type().and_then(|rt| rt.type_ref()) {
            type_ref_builder.alloc_from_node(&type_ref)
        } else {
            type_ref_builder.unit()
        };

        let (type_ref_map, type_ref_source_map) = type_ref_builder.finish();

        Arc::new(FnData {
            name,
            params,
            visibility,
            ret_type,
            type_ref_map,
            type_ref_source_map,
        })
    }

    pub fn name(&self) -> &Name {
        &self.name
    }

    pub fn params(&self) -> &[TypeRefId] {
        &self.params
    }

    pub fn visibility(&self) -> Visibility {
        self.visibility
    }

    pub fn ret_type(&self) -> &TypeRefId {
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
    pub fn module(self, db: &impl DefDatabase) -> Module {
        Module {
            file_id: self.id.file_id(db),
        }
    }

    pub fn name(self, db: &impl HirDatabase) -> Name {
        self.data(db).name.clone()
    }

    pub fn visibility(self, db: &impl HirDatabase) -> Visibility {
        self.data(db).visibility()
    }

    pub fn data(self, db: &impl HirDatabase) -> Arc<FnData> {
        db.fn_data(self)
    }

    pub fn body(self, db: &impl HirDatabase) -> Arc<Body> {
        db.body_hir(self.into())
    }

    pub fn ty(self, db: &impl HirDatabase) -> Ty {
        db.type_for_def(self.into(), Namespace::Values)
    }

    pub fn infer(self, db: &impl HirDatabase) -> Arc<InferenceResult> {
        db.infer(self.into())
    }

    pub(crate) fn body_source_map(self, db: &impl HirDatabase) -> Arc<BodySourceMap> {
        db.body_with_source_map(self.into()).1
    }

    pub(crate) fn resolver(self, db: &impl HirDatabase) -> Resolver {
        // take the outer scope...
        self.module(db).resolver(db)
    }

    pub fn diagnostics(self, db: &impl HirDatabase, sink: &mut DiagnosticSink) {
        let infer = self.infer(db);
        infer.add_diagnostics(db, self, sink);
        //        let mut validator = ExprValidator::new(self, infer, sink);
        //        validator.validate_body(db);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuiltinType {
    Float,
    Int,
    Boolean,
}

use crate::code_model::diagnostics::ModuleDefinitionDiagnostic;
use crate::name::*;

impl BuiltinType {
    #[rustfmt::skip]
    pub(crate) const ALL: &'static [(Name, BuiltinType)] = &[
        (FLOAT, BuiltinType::Float),
        (INT, BuiltinType::Int),
        (BOOLEAN, BuiltinType::Boolean),
    ];
}

mod diagnostics {
    use super::Module;
    use crate::diagnostics::{DiagnosticSink, DuplicateDefinition};
    use crate::raw::{DefId, DefKind};
    use crate::{DefDatabase, Name};
    use mun_syntax::{AstNode, SyntaxNodePtr};

    #[derive(Debug, PartialEq, Eq, Clone, Hash)]
    pub(super) enum ModuleDefinitionDiagnostic {
        DuplicateName {
            name: Name,
            definition: DefId,
            first_definition: DefId,
        },
    }

    fn syntax_ptr_from_def(db: &impl DefDatabase, owner: Module, kind: DefKind) -> SyntaxNodePtr {
        match kind {
            DefKind::Function(id) => {
                SyntaxNodePtr::new(id.with_file_id(owner.file_id).to_node(db).syntax())
            }
        }
    }

    impl ModuleDefinitionDiagnostic {
        pub(super) fn add_to(
            &self,
            db: &impl DefDatabase,
            owner: Module,
            sink: &mut DiagnosticSink,
        ) {
            match self {
                ModuleDefinitionDiagnostic::DuplicateName {
                    name,
                    definition,
                    first_definition,
                } => {
                    let raw_items = db.raw_items(owner.file_id);
                    sink.push(DuplicateDefinition {
                        file: owner.file_id,
                        name: name.to_string(),
                        definition: syntax_ptr_from_def(db, owner, raw_items[*definition].kind),
                        first_definition: syntax_ptr_from_def(
                            db,
                            owner,
                            raw_items[*first_definition].kind,
                        ),
                    })
                }
            }
        }
    }
}
