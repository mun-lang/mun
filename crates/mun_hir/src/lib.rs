//! HIR provides high-level, object-oriented access to Mun code. It is constructed by first parsing
//! Mun code with the mun_syntax crate and then it is lowered into HIR constructs, names are
//! resolved, and type checking is performed. HIR is the input for both the compiler as well as the
//! language server.

#![allow(dead_code)]

#[macro_use]
mod arena;
mod code_model;
mod db;
pub mod diagnostics;
mod display;
mod expr;
mod ids;
mod input;
pub mod line_index;
mod model;
mod name;
mod name_resolution;
mod path;
mod raw;
mod resolve;
mod source_id;
mod ty;
mod type_ref;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub use salsa;

pub use crate::{
    db::{
        DefDatabase, DefDatabaseStorage, HirDatabase, HirDatabaseStorage, RelativePathBuf,
        SourceDatabase, SourceDatabaseStorage,
    },
    display::HirDisplay,
    expr::{
        resolver_for_expr, ArithOp, BinaryOp, Body, CmpOp, Expr, ExprId, ExprScopes, Literal,
        LogicOp, Ordering, Pat, PatId, Statement,
    },
    ids::ItemLoc,
    input::{FileId, PackageInput},
    name::Name,
    name_resolution::PerNs,
    path::{Path, PathKind},
    raw::RawItems,
    resolve::{Resolution, Resolver},
    ty::{ApplicationTy, InferenceResult, Ty, TypeCtor},
};

use crate::{
    arena::{Arena, ArenaId, RawId},
    name::AsName,
    source_id::{AstIdMap, FileAstId},
};

pub use self::code_model::{FnData, Function, Module, ModuleDef, Visibility};
