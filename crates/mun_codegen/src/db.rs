#![allow(clippy::type_repetition_in_bounds)]

use crate::{
    ir::{file::FileIR, file_group::FileGroupIR},
    type_info::TypeInfo,
    CodeGenParams, Context,
};
use inkwell::{
    targets::TargetData,
    types::{AnyTypeEnum, StructType},
    OptimizationLevel,
};
use std::sync::Arc;

/// The `IrDatabase` enables caching of intermediate in the process of LLVM IR generation. It uses
/// [salsa](https://github.com/salsa-rs/salsa) for this purpose.
#[salsa::query_group(IrDatabaseStorage)]
pub trait IrDatabase: hir::HirDatabase {
    /// Get the LLVM context that should be used for all generation steps.
    #[salsa::input]
    fn context(&self) -> Arc<Context>;

    /// Gets the optimization level for generation.
    #[salsa::input]
    fn optimization_lvl(&self) -> OptimizationLevel;

    /// Returns the target machine's data layout for code generation.
    #[salsa::invoke(crate::code_gen::target_data_query)]
    fn target_data(&self) -> Arc<TargetData>;

    /// Given a type and code generation parameters, return the corresponding IR type.
    #[salsa::invoke(crate::ir::ty::ir_query)]
    fn type_ir(&self, ty: hir::Ty, params: CodeGenParams) -> AnyTypeEnum;

    /// Given a struct, return the corresponding IR type.
    #[salsa::invoke(crate::ir::ty::struct_ty_query)]
    fn struct_ty(&self, s: hir::Struct) -> StructType;

    /// Given a `hir::FileId` generate code that is shared among the group of files.
    /// TODO: Currently, a group always consists of a single file. Need to add support for multiple
    /// files using something like `FileGroupId`.
    #[salsa::invoke(crate::ir::file_group::ir_query)]
    fn group_ir(&self, file: hir::FileId) -> Arc<FileGroupIR>;

    /// Given a `hir::FileId` generate code for the module.
    #[salsa::invoke(crate::ir::file::ir_query)]
    fn file_ir(&self, file: hir::FileId) -> Arc<FileIR>;

    /// Given a type, return the runtime `TypeInfo` that can be used to reflect the type.
    #[salsa::invoke(crate::ir::ty::type_info_query)]
    fn type_info(&self, ty: hir::Ty) -> TypeInfo;
}
