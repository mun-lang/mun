use std::sync::Arc;
use c_codegen::{function, identifier};
use mun_codegen::{CodeGenDatabase, ModuleGroupId};
use mun_hir::Upcast;

#[salsa::query_group(CCodegenDatabaseStorage)]
pub trait CCodegenDatabase: CodeGenDatabase + Upcast<dyn CodeGenDatabase> {
    #[salsa::invoke(crate::code_gen::build_c_files)]
    fn transpile_to_c(&self, module_group: ModuleGroupId) -> String;

    /// Returns the identifier for the given function. The identifier returned uniquely identifies the function.
    #[salsa::invoke(crate::signatures::function_identifier)]
    fn function_identifier(&self, function: mun_hir::Function) -> identifier::Identifier;
}
