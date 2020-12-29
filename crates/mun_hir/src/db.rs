#![allow(clippy::type_repetition_in_bounds)]

use crate::expr::BodySourceMap;
use crate::ids::{DefWithBodyId, FunctionId, PackageId};
use crate::input::{SourceRoot, SourceRootId};
use crate::item_tree::{self, ItemTree};
use crate::module_tree::ModuleTree;
use crate::name_resolution::Namespace;
use crate::package_defs::PackageDefs;
use crate::ty::lower::LowerBatchResult;
use crate::ty::{CallableDef, FnSig, Ty, TypableDef};
use crate::{
    code_model::{FunctionData, StructData, TypeAliasData},
    ids,
    line_index::LineIndex,
    ty::InferenceResult,
    AstIdMap, Body, ExprScopes, FileId, Struct, TypeAlias,
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

/// Database which stores all significant input facts: source code and project model.
#[salsa::query_group(SourceDatabaseStorage)]
pub trait SourceDatabase: salsa::Database {
    /// Text of the file.
    #[salsa::input]
    fn file_text(&self, file_id: FileId) -> Arc<str>;

    /// Path to a file, relative to the root of its source root.
    #[salsa::input]
    fn file_relative_path(&self, file_id: FileId) -> RelativePathBuf;

    /// Source root of a file
    #[salsa::input]
    fn file_source_root(&self, file_id: FileId) -> SourceRootId;

    /// Contents of the source root
    #[salsa::input]
    fn source_root(&self, id: SourceRootId) -> Arc<SourceRoot>;

    /// The source root for a specific package
    #[salsa::input]
    fn package_source_root(&self, package: PackageId) -> SourceRootId;

    /// For a package, returns its hierarchy of modules.
    #[salsa::invoke(ModuleTree::module_tree_query)]
    fn module_tree(&self, package: PackageId) -> Arc<ModuleTree>;

    /// Returns the line index of a file
    #[salsa::invoke(line_index_query)]
    fn line_index(&self, file_id: FileId) -> Arc<LineIndex>;
}

/// The `AstDatabase` provides queries that transform text from the `SourceDatabase` into an
/// Abstract Syntax Tree (AST).
#[salsa::query_group(AstDatabaseStorage)]
pub trait AstDatabase: SourceDatabase {
    /// Parses the file into the syntax tree.
    #[salsa::invoke(parse_query)]
    fn parse(&self, file_id: FileId) -> Parse<ast::SourceFile>;

    /// Returns the top level AST items of a file
    #[salsa::invoke(crate::source_id::AstIdMap::ast_id_map_query)]
    fn ast_id_map(&self, file_id: FileId) -> Arc<AstIdMap>;
}

/// The `InternDatabase` maps certain datastructures to ids. These ids refer to instances of
/// concepts like a `Function`, `Struct` or `TypeAlias` in a semi-stable way.
#[salsa::query_group(InternDatabaseStorage)]
pub trait InternDatabase: SourceDatabase {
    #[salsa::interned]
    fn intern_function(&self, loc: ids::FunctionLoc) -> ids::FunctionId;
    #[salsa::interned]
    fn intern_struct(&self, loc: ids::StructLoc) -> ids::StructId;
    #[salsa::interned]
    fn intern_type_alias(&self, loc: ids::TypeAliasLoc) -> ids::TypeAliasId;
}

#[salsa::query_group(DefDatabaseStorage)]
pub trait DefDatabase: InternDatabase + AstDatabase + Upcast<dyn AstDatabase> {
    /// Returns the `ItemTree` for a specific file. An `ItemTree` represents all the top level
    /// declarations within a file.
    #[salsa::invoke(item_tree::ItemTree::item_tree_query)]
    fn item_tree(&self, file_id: FileId) -> Arc<ItemTree>;

    #[salsa::invoke(StructData::struct_data_query)]
    fn struct_data(&self, id: ids::StructId) -> Arc<StructData>;

    #[salsa::invoke(TypeAliasData::type_alias_data_query)]
    fn type_alias_data(&self, id: ids::TypeAliasId) -> Arc<TypeAliasData>;

    #[salsa::invoke(crate::FunctionData::fn_data_query)]
    fn fn_data(&self, func: FunctionId) -> Arc<FunctionData>;

    /// Returns the `PackageDefs` for the specified `PackageId`. The `PackageDefs` contains all
    /// resolved items defined for every module in the package.
    #[salsa::invoke(crate::package_defs::PackageDefs::package_def_map_query)]
    fn package_defs(&self, package_id: PackageId) -> Arc<PackageDefs>;

    #[salsa::invoke(Body::body_query)]
    fn body(&self, def: DefWithBodyId) -> Arc<Body>;

    #[salsa::invoke(Body::body_with_source_map_query)]
    fn body_with_source_map(&self, def: DefWithBodyId) -> (Arc<Body>, Arc<BodySourceMap>);

    #[salsa::invoke(ExprScopes::expr_scopes_query)]
    fn expr_scopes(&self, def: DefWithBodyId) -> Arc<ExprScopes>;
}

#[salsa::query_group(HirDatabaseStorage)]
pub trait HirDatabase: DefDatabase + Upcast<dyn DefDatabase> {
    /// Returns the target for code generation.
    #[salsa::input]
    fn target(&self) -> Target;

    /// Returns the `TargetDataLayout` for the current target
    #[salsa::invoke(target_data_layout)]
    fn target_data_layout(&self) -> Arc<abi::TargetDataLayout>;

    #[salsa::invoke(crate::ty::infer_query)]
    fn infer(&self, def: DefWithBodyId) -> Arc<InferenceResult>;

    #[salsa::invoke(crate::ty::lower::lower_struct_query)]
    fn lower_struct(&self, def: Struct) -> Arc<LowerBatchResult>;

    #[salsa::invoke(crate::ty::lower::lower_type_alias_query)]
    fn lower_type_alias(&self, def: TypeAlias) -> Arc<LowerBatchResult>;

    #[salsa::invoke(crate::ty::callable_item_sig)]
    fn callable_sig(&self, def: CallableDef) -> FnSig;

    #[salsa::invoke(crate::ty::type_for_def)]
    #[salsa::cycle(crate::ty::type_for_cycle_recover)]
    fn type_for_def(&self, def: TypableDef, ns: Namespace) -> (Ty, bool);
}

fn parse_query(db: &dyn AstDatabase, file_id: FileId) -> Parse<SourceFile> {
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
