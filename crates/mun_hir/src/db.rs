#![allow(clippy::type_repetition_in_bounds)]

use std::sync::Arc;

use la_arena::ArenaMap;
use mun_hir_input::{FileId, PackageId, SourceDatabase};
use mun_syntax::{ast, Parse, SourceFile};
use mun_target::{abi, spec::Target};

use crate::{
    code_model::{r#struct::LocalFieldId, FunctionData, ImplData, StructData, TypeAliasData},
    expr::BodySourceMap,
    ids,
    ids::{DefWithBodyId, FunctionId, ImplId, VariantId},
    item_tree::{self, ItemTree},
    method_resolution::InherentImpls,
    name_resolution::Namespace,
    package_defs::PackageDefs,
    ty::{lower::LowerTyMap, CallableDef, FnSig, InferenceResult, Ty, TypableDef},
    visibility, AstIdMap, Body, ExprScopes, Struct, TypeAlias, Visibility,
};

/// The `AstDatabase` provides queries that transform text from the
/// `SourceDatabase` into an Abstract Syntax Tree (AST).
#[salsa::query_group(AstDatabaseStorage)]
pub trait AstDatabase: SourceDatabase {
    /// Parses the file into the syntax tree.
    #[salsa::invoke(parse_query)]
    fn parse(&self, file_id: FileId) -> Parse<ast::SourceFile>;

    /// Returns the top level AST items of a file
    #[salsa::invoke(crate::source_id::AstIdMap::ast_id_map_query)]
    fn ast_id_map(&self, file_id: FileId) -> Arc<AstIdMap>;
}

/// The `InternDatabase` maps certain datastructures to ids. These ids refer to
/// instances of concepts like a `Function`, `Struct` or `TypeAlias` in a
/// semi-stable way.
#[salsa::query_group(InternDatabaseStorage)]
pub trait InternDatabase: SourceDatabase {
    #[salsa::interned]
    fn intern_function(&self, loc: ids::FunctionLoc) -> ids::FunctionId;
    #[salsa::interned]
    fn intern_struct(&self, loc: ids::StructLoc) -> ids::StructId;
    #[salsa::interned]
    fn intern_type_alias(&self, loc: ids::TypeAliasLoc) -> ids::TypeAliasId;
    #[salsa::interned]
    fn intern_impl(self, loc: ids::ImplLoc) -> ids::ImplId;
}

#[salsa::query_group(DefDatabaseStorage)]
pub trait DefDatabase: InternDatabase + AstDatabase {
    /// Returns the `ItemTree` for a specific file. An `ItemTree` represents all
    /// the top level declarations within a file.
    #[salsa::invoke(item_tree::ItemTree::item_tree_query)]
    fn item_tree(&self, file_id: FileId) -> Arc<ItemTree>;

    #[salsa::invoke(StructData::struct_data_query)]
    fn struct_data(&self, id: ids::StructId) -> Arc<StructData>;

    #[salsa::invoke(TypeAliasData::type_alias_data_query)]
    fn type_alias_data(&self, id: ids::TypeAliasId) -> Arc<TypeAliasData>;

    #[salsa::invoke(crate::FunctionData::fn_data_query)]
    fn fn_data(&self, func: FunctionId) -> Arc<FunctionData>;

    #[salsa::invoke(visibility::function_visibility_query)]
    fn function_visibility(&self, def: FunctionId) -> Visibility;

    #[salsa::invoke(visibility::field_visibilities_query)]
    fn field_visibilities(&self, variant_id: VariantId) -> Arc<ArenaMap<LocalFieldId, Visibility>>;

    /// Returns the `PackageDefs` for the specified `PackageId`. The
    /// `PackageDefs` contains all resolved items defined for every module
    /// in the package.
    #[salsa::invoke(crate::package_defs::PackageDefs::package_def_map_query)]
    fn package_defs(&self, package_id: PackageId) -> Arc<PackageDefs>;

    #[salsa::invoke(Body::body_query)]
    fn body(&self, def: DefWithBodyId) -> Arc<Body>;

    #[salsa::invoke(Body::body_with_source_map_query)]
    fn body_with_source_map(&self, def: DefWithBodyId) -> (Arc<Body>, Arc<BodySourceMap>);

    #[salsa::invoke(ExprScopes::expr_scopes_query)]
    fn expr_scopes(&self, def: DefWithBodyId) -> Arc<ExprScopes>;

    #[salsa::invoke(ImplData::impl_data_query)]
    fn impl_data(&self, def: ImplId) -> Arc<ImplData>;
}

#[salsa::query_group(HirDatabaseStorage)]
pub trait HirDatabase: DefDatabase {
    /// Returns the target for code generation.
    #[salsa::input]
    fn target(&self) -> Target;

    /// Returns the `TargetDataLayout` for the current target
    #[salsa::invoke(target_data_layout)]
    fn target_data_layout(&self) -> Arc<abi::TargetDataLayout>;

    #[salsa::invoke(crate::ty::infer_query)]
    fn infer(&self, def: DefWithBodyId) -> Arc<InferenceResult>;

    #[salsa::invoke(crate::ty::lower::lower_struct_query)]
    fn lower_struct(&self, def: Struct) -> Arc<LowerTyMap>;

    #[salsa::invoke(crate::ty::lower::lower_type_alias_query)]
    fn lower_type_alias(&self, def: TypeAlias) -> Arc<LowerTyMap>;

    #[salsa::invoke(crate::ty::callable_item_sig)]
    fn callable_sig(&self, def: CallableDef) -> FnSig;

    #[salsa::invoke(crate::ty::lower::lower_impl_query)]
    fn lower_impl(&self, def: ImplId) -> Arc<LowerTyMap>;

    #[salsa::invoke(crate::ty::type_for_def)]
    fn type_for_def(&self, def: TypableDef, ns: Namespace) -> Ty;

    #[salsa::invoke(crate::ty::type_for_impl_self)]
    fn type_for_impl_self(&self, def: ImplId) -> Ty;

    #[salsa::invoke(InherentImpls::inherent_impls_in_package_query)]
    fn inherent_impls_in_package(&self, package: PackageId) -> Arc<InherentImpls>;
}

fn parse_query(db: &dyn AstDatabase, file_id: FileId) -> Parse<SourceFile> {
    let text = db.file_text(file_id);
    SourceFile::parse(&text)
}

fn target_data_layout(db: &dyn HirDatabase) -> Arc<abi::TargetDataLayout> {
    let target = db.target();
    let data_layout = abi::TargetDataLayout::parse(&target)
        .expect("unable to create TargetDataLayout from target");
    Arc::new(data_layout)
}
