//! HIR provides high-level, object-oriented access to Mun code. It is constructed by first parsing
//! Mun code with the mun_syntax crate and then it is lowered into HIR constructs, names are
//! resolved, and type checking is performed. HIR is the input for both the compiler as well as the
//! language server.

#![allow(dead_code)]

#[macro_use]
mod macros;
#[macro_use]
mod arena;
mod adt;
mod builtin_type;
mod code_model;
mod db;
pub mod diagnostics;
mod display;
mod expr;
mod ids;
mod in_file;
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
mod utils;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub use salsa;

pub use relative_path::{RelativePath, RelativePathBuf};

pub use crate::{
    arena::{ArenaId, RawId},
    builtin_type::{FloatBitness, IntBitness, Signedness},
    db::{
        DefDatabase, DefDatabaseStorage, HirDatabase, HirDatabaseStorage, SourceDatabase,
        SourceDatabaseStorage,
    },
    display::HirDisplay,
    expr::{
        resolver_for_expr, ArithOp, BinaryOp, Body, CmpOp, Expr, ExprId, ExprScopes, Literal,
        LogicOp, Ordering, Pat, PatId, RecordLitField, Statement, UnaryOp,
    },
    ids::ItemLoc,
    input::{FileId, SourceRoot, SourceRootId},
    name::Name,
    name_resolution::PerNs,
    path::{Path, PathKind},
    raw::RawItems,
    resolve::{Resolution, Resolver},
    ty::{
        lower::CallableDef, ApplicationTy, FloatTy, InferenceResult, IntTy, ResolveBitness, Ty,
        TypeCtor,
    },
};

use crate::{
    arena::Arena,
    name::AsName,
    source_id::{AstIdMap, FileAstId},
};

pub use self::adt::StructMemoryKind;
pub use self::code_model::{
    FnData, Function, Module, ModuleDef, Struct, Visibility,
    StructField,
};
