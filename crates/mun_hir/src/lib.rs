//! HIR provides high-level, object-oriented access to Mun code. It is
//! constructed by first parsing Mun code with the `mun_syntax` crate and then
//! it is lowered into HIR constructs, names are resolved, and type checking is
//! performed. HIR is the input for both the compiler as well as the
//! language server.

#![allow(dead_code)]

pub use salsa;

pub use self::code_model::{
    Enum, Field, Function, FunctionData, HasSource, Module, ModuleDef, Package, Struct,
    StructMemoryKind, TypeAlias,
};
pub use crate::{
    db::{
        AstDatabase, AstDatabaseStorage, DefDatabase, DefDatabaseStorage, HirDatabase,
        HirDatabaseStorage, InternDatabase, InternDatabaseStorage, SourceDatabase,
        SourceDatabaseStorage, Upcast,
    },
    diagnostics::{Diagnostic, DiagnosticSink},
    display::HirDisplay,
    expr::{
        ArithOp, BinaryOp, Body, CmpOp, Expr, ExprId, ExprScopes, Literal, LogicOp, Ordering, Pat,
        PatId, RecordLitField, Statement, UnaryOp,
    },
    ids::{ItemLoc, ModuleId},
    in_file::InFile,
    input::{FileId, SourceRoot, SourceRootId},
    name::Name,
    name_resolution::PerNs,
    package_set::{PackageId, PackageSet},
    path::{Path, PathKind},
    primitive_type::{FloatBitness, IntBitness, Signedness},
    resolve::{resolver_for_expr, resolver_for_scope, Resolver, TypeNs, ValueNs},
    ty::{
        lower::CallableDef, FloatTy, InferenceResult, IntTy, ResolveBitness, Substitution, Ty,
        TyKind,
    },
    visibility::{HasVisibility, Visibility},
};
use crate::{name::AsName, source_id::AstIdMap};

#[macro_use]
mod macros;
mod code_model;
mod db;
pub mod diagnostics;
mod display;
mod expr;
mod ids;
mod in_file;
mod input;
mod item_tree;
pub mod line_index;
mod module_tree;
mod name;
mod name_resolution;
mod path;
mod primitive_type;
mod resolve;
mod source_id;
mod ty;
mod type_ref;
mod utils;

pub mod fixture;
mod has_module;
mod item_scope;
mod method_resolution;
#[cfg(test)]
mod mock;
mod package_defs;
mod package_set;
mod pretty;
pub mod semantics;
mod source_analyzer;
#[cfg(test)]
mod tests;
mod visibility;
pub mod with_fixture;
