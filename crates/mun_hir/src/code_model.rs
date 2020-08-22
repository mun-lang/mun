pub(crate) mod src;

use self::src::HasSource;
use crate::adt::{StructData, StructFieldId, TypeAliasData};
use crate::builtin_type::BuiltinType;
use crate::code_model::diagnostics::ModuleDefinitionDiagnostic;
use crate::diagnostics::DiagnosticSink;
use crate::expr::validator::{ExprValidator, TypeAliasValidator};
use crate::expr::{Body, BodySourceMap};
use crate::ids::AstItemDef;
use crate::ids::LocationCtx;
use crate::name_resolution::Namespace;
use crate::raw::{DefKind, RawFileItem};
use crate::resolve::{Resolution, Resolver};
use crate::ty::{lower::LowerBatchResult, InferenceResult};
use crate::type_ref::{TypeRefBuilder, TypeRefId, TypeRefMap, TypeRefSourceMap};
use crate::{
    ids::{FunctionId, StructId, TypeAliasId},
    AsName, DefDatabase, FileId, HirDatabase, Name, Ty,
};
use mun_syntax::ast::{ExternOwner, NameOwner, TypeAscriptionOwner, VisibilityOwner};
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
    pub fn declarations(self, db: &dyn HirDatabase) -> Vec<ModuleDef> {
        db.module_data(self.file_id).definitions.clone()
    }

    fn resolver(self, _db: &dyn DefDatabase) -> Resolver {
        Resolver::default().push_module_scope(self.file_id)
    }

    pub fn diagnostics(self, db: &dyn HirDatabase, sink: &mut DiagnosticSink) {
        for diag in db.module_data(self.file_id).diagnostics.iter() {
            diag.add_to(db.upcast(), self, sink);
        }
        for decl in self.declarations(db) {
            match decl {
                ModuleDef::Function(f) => f.diagnostics(db, sink),
                ModuleDef::Struct(s) => s.diagnostics(db, sink),
                ModuleDef::TypeAlias(t) => t.diagnostics(db, sink),
                ModuleDef::BuiltinType(_) => (),
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
    pub(crate) fn module_data_query(db: &dyn DefDatabase, file_id: FileId) -> Arc<ModuleData> {
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
                        DefKind::Struct(ast_id) => {
                            data.definitions.push(ModuleDef::Struct(Struct {
                                id: StructId::from_ast_id(loc_ctx, ast_id),
                            }))
                        }
                        DefKind::TypeAlias(ast_id) => {
                            data.definitions.push(ModuleDef::TypeAlias(TypeAlias {
                                id: TypeAliasId::from_ast_id(loc_ctx, ast_id),
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
    Struct(Struct),
    TypeAlias(TypeAlias),
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

impl From<Struct> for ModuleDef {
    fn from(t: Struct) -> Self {
        ModuleDef::Struct(t)
    }
}

/// The definitions that have a body.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DefWithBody {
    Function(Function),
}
impl_froms!(DefWithBody: Function);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Public,
    Private,
}

impl DefWithBody {
    pub fn infer(self, db: &dyn HirDatabase) -> Arc<InferenceResult> {
        db.infer(self)
    }

    pub fn body(self, db: &dyn HirDatabase) -> Arc<Body> {
        db.body(self)
    }

    pub fn body_source_map(self, db: &dyn HirDatabase) -> Arc<BodySourceMap> {
        db.body_with_source_map(self).1
    }

    /// Builds a `Resolver` for code inside this item. A `Resolver` enables name resolution.
    pub(crate) fn resolver(self, db: &dyn HirDatabase) -> Resolver {
        match self {
            DefWithBody::Function(f) => f.resolver(db),
        }
    }
}

impl Visibility {
    pub fn is_public(self) -> bool {
        self == Visibility::Public
    }

    pub fn is_private(self) -> bool {
        self == Visibility::Private
    }
}

/// Definitions that have a struct.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DefWithStruct {
    Struct(Struct),
}
impl_froms!(DefWithStruct: Struct);

impl DefWithStruct {
    pub fn fields(self, db: &dyn HirDatabase) -> Vec<StructField> {
        match self {
            DefWithStruct::Struct(s) => s.fields(db),
        }
    }

    pub fn field(self, db: &dyn HirDatabase, name: &Name) -> Option<StructField> {
        match self {
            DefWithStruct::Struct(s) => s.field(db, name),
        }
    }

    pub fn module(self, db: &dyn HirDatabase) -> Module {
        match self {
            DefWithStruct::Struct(s) => s.module(db.upcast()),
        }
    }

    pub fn data(self, db: &dyn HirDatabase) -> Arc<StructData> {
        match self {
            DefWithStruct::Struct(s) => s.data(db.upcast()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
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
    is_extern: bool,
}

impl FnData {
    pub(crate) fn fn_data_query(db: &dyn DefDatabase, func: Function) -> Arc<FnData> {
        let src = func.source(db);
        let mut type_ref_builder = TypeRefBuilder::default();
        let name = src
            .value
            .name()
            .map(|n| n.as_name())
            .unwrap_or_else(Name::missing);

        let visibility = src
            .value
            .visibility()
            .map(|_v| Visibility::Public)
            .unwrap_or(Visibility::Private);

        let mut params = Vec::new();
        if let Some(param_list) = src.value.param_list() {
            for param in param_list.params() {
                let type_ref = type_ref_builder.alloc_from_node_opt(param.ascribed_type().as_ref());
                params.push(type_ref);
            }
        }

        let ret_type = if let Some(type_ref) = src.value.ret_type().and_then(|rt| rt.type_ref()) {
            type_ref_builder.alloc_from_node(&type_ref)
        } else {
            type_ref_builder.unit()
        };

        let (type_ref_map, type_ref_source_map) = type_ref_builder.finish();

        let is_extern = src.value.is_extern();

        Arc::new(FnData {
            name,
            params,
            visibility,
            ret_type,
            type_ref_map,
            type_ref_source_map,
            is_extern,
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
    pub fn module(self, db: &dyn DefDatabase) -> Module {
        Module {
            file_id: self.id.file_id(db),
        }
    }

    pub fn name(self, db: &dyn HirDatabase) -> Name {
        self.data(db).name.clone()
    }

    pub fn visibility(self, db: &dyn HirDatabase) -> Visibility {
        self.data(db).visibility()
    }

    pub fn data(self, db: &dyn HirDatabase) -> Arc<FnData> {
        db.fn_data(self)
    }

    pub fn body(self, db: &dyn HirDatabase) -> Arc<Body> {
        db.body(self.into())
    }

    pub fn ty(self, db: &dyn HirDatabase) -> Ty {
        // TODO: Add detection of cyclick types
        db.type_for_def(self.into(), Namespace::Values).0
    }

    pub fn infer(self, db: &dyn HirDatabase) -> Arc<InferenceResult> {
        db.infer(self.into())
    }

    pub fn is_extern(self, db: &dyn HirDatabase) -> bool {
        db.fn_data(self).is_extern
    }

    pub(crate) fn body_source_map(self, db: &dyn HirDatabase) -> Arc<BodySourceMap> {
        db.body_with_source_map(self.into()).1
    }

    pub(crate) fn resolver(self, db: &dyn HirDatabase) -> Resolver {
        // take the outer scope...
        self.module(db.upcast()).resolver(db.upcast())
    }

    pub fn diagnostics(self, db: &dyn HirDatabase, sink: &mut DiagnosticSink) {
        let body = self.body(db);
        body.add_diagnostics(db, self.into(), sink);
        let infer = self.infer(db);
        infer.add_diagnostics(db, self, sink);
        let validator = ExprValidator::new(self, db);
        validator.validate_body(sink);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Struct {
    pub(crate) id: StructId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StructField {
    pub(crate) parent: Struct,
    pub(crate) id: StructFieldId,
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

    pub fn id(self) -> StructFieldId {
        self.id
    }
}

impl Struct {
    pub fn module(self, db: &dyn DefDatabase) -> Module {
        Module {
            file_id: self.id.file_id(db),
        }
    }

    pub fn data(self, db: &dyn DefDatabase) -> Arc<StructData> {
        db.struct_data(self.id)
    }

    pub fn name(self, db: &dyn DefDatabase) -> Name {
        self.data(db).name.clone()
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

    pub(crate) fn resolver(self, db: &dyn HirDatabase) -> Resolver {
        // take the outer scope...
        self.module(db.upcast()).resolver(db.upcast())
    }

    pub fn diagnostics(self, db: &dyn HirDatabase, sink: &mut DiagnosticSink) {
        let data = self.data(db.upcast());
        let lower = self.lower(db);
        lower.add_diagnostics(
            db,
            self.module(db.upcast()).file_id,
            data.type_ref_source_map(),
            sink,
        );
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeAlias {
    pub(crate) id: TypeAliasId,
}

impl TypeAlias {
    pub fn module(self, db: &dyn DefDatabase) -> Module {
        Module {
            file_id: self.id.file_id(db),
        }
    }

    pub fn data(self, db: &dyn DefDatabase) -> Arc<TypeAliasData> {
        db.type_alias_data(self.id)
    }

    pub fn name(self, db: &dyn DefDatabase) -> Name {
        self.data(db).name.clone()
    }

    pub fn type_ref(self, db: &dyn HirDatabase) -> TypeRefId {
        self.data(db.upcast()).type_ref_id
    }

    pub fn lower(self, db: &dyn HirDatabase) -> Arc<LowerBatchResult> {
        db.lower_type_alias(self)
    }

    pub(crate) fn resolver(self, db: &dyn HirDatabase) -> Resolver {
        // take the outer scope...
        self.module(db.upcast()).resolver(db.upcast())
    }

    pub fn diagnostics(self, db: &dyn HirDatabase, sink: &mut DiagnosticSink) {
        let data = self.data(db.upcast());
        let lower = self.lower(db);
        lower.add_diagnostics(
            db,
            self.module(db.upcast()).file_id,
            data.type_ref_source_map(),
            sink,
        );

        let validator = TypeAliasValidator::new(self, db);
        validator.validate_target_type_existence(sink);
    }
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

    fn syntax_ptr_from_def(db: &dyn DefDatabase, owner: Module, kind: DefKind) -> SyntaxNodePtr {
        match kind {
            DefKind::Function(id) => {
                SyntaxNodePtr::new(id.with_file_id(owner.file_id).to_node(db).syntax())
            }
            DefKind::Struct(id) => {
                SyntaxNodePtr::new(id.with_file_id(owner.file_id).to_node(db).syntax())
            }
            DefKind::TypeAlias(id) => {
                SyntaxNodePtr::new(id.with_file_id(owner.file_id).to_node(db).syntax())
            }
        }
    }

    impl ModuleDefinitionDiagnostic {
        pub(super) fn add_to(
            &self,
            db: &dyn DefDatabase,
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
