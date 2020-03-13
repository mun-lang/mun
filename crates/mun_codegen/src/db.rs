#![allow(clippy::type_repetition_in_bounds)]

use crate::{ir::module::ModuleIR, type_info::TypeInfo, CodeGenParams, Context};
use inkwell::{
    module::Module,
    targets::TargetData,
    types::{AnyTypeEnum, StructType},
    OptimizationLevel,
};
use mun_target::spec::Target;
use std::sync::Arc;

/// The `IrDatabase` enables caching of intermediate in the process of LLVM IR generation. It uses
/// [salsa](https://github.com/salsa-rs/salsa) for this purpose.
#[salsa::query_group(IrDatabaseStorage)]
pub trait IrDatabase: hir::HirDatabase {
    /// Get the LLVM context that should be used for all generation steps.
    #[salsa::input]
    fn context(&self) -> Arc<Context>;

    /// Returns the LLVM module that should be used for all generation steps.
    #[salsa::input]
    fn module(&self) -> Arc<Module>;

    /// Gets the optimization level for generation.
    #[salsa::input]
    fn optimization_lvl(&self) -> OptimizationLevel;

    /// Returns the target for code generation.
    #[salsa::input]
    fn target(&self) -> Target;

    /// Returns the target machine's data layout for code generation.
    #[salsa::input]
    fn target_data(&self) -> Arc<TargetData>;

    /// Given a type and code generation parameters, return the corresponding IR type.
    #[salsa::invoke(crate::ir::ty::ir_query)]
    fn type_ir(&self, ty: hir::Ty, params: CodeGenParams) -> AnyTypeEnum;

    /// Given a struct, return the corresponding IR type.
    #[salsa::invoke(crate::ir::ty::struct_ty_query)]
    fn struct_ty(&self, s: hir::Struct) -> StructType;

    /// Given a `hir::FileId` generate code for the module.
    #[salsa::invoke(crate::ir::module::ir_query)]
    fn module_ir(&self, file: hir::FileId) -> Arc<ModuleIR>;

    /// Given a type, return the runtime `TypeInfo` that can be used to reflect the type.
    #[salsa::invoke(crate::ir::ty::type_info_query)]
    fn type_info(&self, ty: hir::Ty) -> TypeInfo;
}
