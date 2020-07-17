#![allow(clippy::type_repetition_in_bounds)]

use crate::input::{SourceRoot, SourceRootId};
use crate::name_resolution::Namespace;
use crate::ty::lower::LowerBatchResult;
use crate::ty::{CallableDef, FnSig, Ty, TypableDef};
use crate::{
    adt::StructData,
    code_model::{DefWithBody, FnData, Function, ModuleData},
    ids,
    line_index::LineIndex,
    name_resolution::ModuleScope,
    ty::InferenceResult,
    AstIdMap, ExprScopes, FileId, RawItems, Struct,
};
use mun_syntax::{ast, Parse, SourceFile};
use mun_target::abi;
use mun_target::spec::Target;
pub use relative_path::RelativePathBuf;
use std::sync::Arc;

// TODO(bas): In the future maybe move this to a seperate crate (mun_db?)
pub trait Upcast<T: ?Sized> {
    fn upcast(&self) -> &T;
}

/// Database which stores all significant input facts: source code and project model. Everything
/// else in rust-analyzer is derived from these queries.
#[salsa::query_group(SourceDatabaseStorage)]
pub trait SourceDatabase: salsa::Database {
    /// Text of the file.
    #[salsa::input]
    fn file_text(&self, file_id: FileId) -> Arc<String>;

    /// Path to a file, relative to the root of its source root.
    #[salsa::input]
    fn file_relative_path(&self, file_id: FileId) -> RelativePathBuf;

    /// Parses the file into the syntax tree.
    #[salsa::invoke(parse_query)]
    fn parse(&self, file_id: FileId) -> Parse<ast::SourceFile>;

    /// Source root of a file
    #[salsa::input]
    fn file_source_root(&self, file_id: FileId) -> SourceRootId;

    /// Contents of the source root
    #[salsa::input]
    fn source_root(&self, id: SourceRootId) -> Arc<SourceRoot>;

    /// Returns the line index of a file
    #[salsa::invoke(line_index_query)]
    fn line_index(&self, file_id: FileId) -> Arc<LineIndex>;
}

#[salsa::query_group(DefDatabaseStorage)]
pub trait DefDatabase: SourceDatabase + Upcast<dyn SourceDatabase> {
    /// Returns the top level AST items of a file
    #[salsa::invoke(crate::source_id::AstIdMap::ast_id_map_query)]
    fn ast_id_map(&self, file_id: FileId) -> Arc<AstIdMap>;

    /// Returns the raw items of a file
    #[salsa::invoke(RawItems::raw_file_items_query)]
    fn raw_items(&self, file_id: FileId) -> Arc<RawItems>;

    #[salsa::invoke(StructData::struct_data_query)]
    fn struct_data(&self, id: ids::StructId) -> Arc<StructData>;

    #[salsa::invoke(crate::FnData::fn_data_query)]
    fn fn_data(&self, func: Function) -> Arc<FnData>;

    /// Returns the module data of the specified file
    #[salsa::invoke(crate::code_model::ModuleData::module_data_query)]
    fn module_data(&self, file_id: FileId) -> Arc<ModuleData>;

    /// Interns a function definition
    #[salsa::interned]
    fn intern_function(&self, loc: ids::ItemLoc<ast::FunctionDef>) -> ids::FunctionId;

    /// Interns a struct definition
    #[salsa::interned]
    fn intern_struct(&self, loc: ids::ItemLoc<ast::StructDef>) -> ids::StructId;
}

#[salsa::query_group(HirDatabaseStorage)]
pub trait HirDatabase: DefDatabase + Upcast<dyn DefDatabase> {
    /// Returns the target for code generation.
    #[salsa::input]
    fn target(&self) -> Target;

    /// Returns the `TargetDataLayout` for the current target
    #[salsa::invoke(target_data_layout)]
    fn target_data_layout(&self) -> Arc<abi::TargetDataLayout>;

    #[salsa::invoke(ExprScopes::expr_scopes_query)]
    fn expr_scopes(&self, def: DefWithBody) -> Arc<ExprScopes>;

    #[salsa::invoke(crate::name_resolution::module_scope_query)]
    fn module_scope(&self, file_id: FileId) -> Arc<ModuleScope>;

    #[salsa::invoke(crate::ty::infer_query)]
    fn infer(&self, def: DefWithBody) -> Arc<InferenceResult>;

    #[salsa::invoke(crate::ty::lower::lower_struct_query)]
    fn lower_struct(&self, def: Struct) -> Arc<LowerBatchResult>;

    #[salsa::invoke(crate::ty::callable_item_sig)]
    fn callable_sig(&self, def: CallableDef) -> FnSig;

    #[salsa::invoke(crate::ty::type_for_def)]
    fn type_for_def(&self, def: TypableDef, ns: Namespace) -> Ty;

    #[salsa::invoke(crate::expr::body_hir_query)]
    fn body(&self, def: DefWithBody) -> Arc<crate::expr::Body>;

    #[salsa::invoke(crate::expr::body_with_source_map_query)]
    fn body_with_source_map(
        &self,
        def: DefWithBody,
    ) -> (Arc<crate::expr::Body>, Arc<crate::expr::BodySourceMap>);
}

fn parse_query(db: &dyn SourceDatabase, file_id: FileId) -> Parse<SourceFile> {
    let text = db.file_text(file_id);
    SourceFile::parse(&*text)
}

fn line_index_query(db: &dyn SourceDatabase, file_id: FileId) -> Arc<LineIndex> {
    let text = db.file_text(file_id);
    Arc::new(LineIndex::new(text.as_ref()))
}

fn target_data_layout(db: &dyn HirDatabase) -> Arc<abi::TargetDataLayout> {
    let target = db.target();
    let data_layout = abi::TargetDataLayout::parse(&target)
        .expect("unable to create TargetDataLayout from target");
    Arc::new(data_layout)
}
