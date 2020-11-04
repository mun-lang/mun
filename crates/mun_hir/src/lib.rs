//! HIR provides high-level, object-oriented access to Mun code. It is constructed by first parsing
//! Mun code with the mun_syntax crate and then it is lowered into HIR constructs, names are
//! resolved, and type checking is performed. HIR is the input for both the compiler as well as the
//! language server.

#![allow(dead_code)]

#[macro_use]
mod macros;
#[macro_use]
mod arena;
mod code_model;
mod db;
pub mod diagnostics;
mod display;
mod expr;
pub mod fixture;
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

mod item_scope;
#[cfg(test)]
mod mock;
mod package_defs;
#[cfg(test)]
mod tests;
mod visibility;

pub use salsa;

pub use relative_path::{RelativePath, RelativePathBuf};

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
    ids::{ItemLoc, ModuleId, PackageId},
    in_file::InFile,
    input::{FileId, SourceRoot, SourceRootId},
    name::Name,
    name_resolution::PerNs,
    path::{Path, PathKind},
    primitive_type::{FloatBitness, IntBitness, Signedness},
    resolve::{resolver_for_expr, resolver_for_scope, Resolver, TypeNs, ValueNs},
    ty::{
        lower::CallableDef, ApplicationTy, FloatTy, InferenceResult, IntTy, ResolveBitness, Ty,
        TypeCtor,
    },
    visibility::{HasVisibility, Visibility},
};

use crate::{name::AsName, source_id::AstIdMap};

pub use self::code_model::{
    Function, FunctionData, Module, ModuleDef, Package, Struct, StructMemoryKind, TypeAlias,
};
