use crate::in_file::InFile;
use crate::source_id::{AstId, FileAstId};
use crate::{DefDatabase, FileId};
use mun_syntax::{ast, AstNode};
use std::hash::{Hash, Hasher};

macro_rules! impl_intern_key {
    ($name:ident) => {
        impl salsa::InternKey for $name {
            fn from_intern_id(v: salsa::InternId) -> Self {
                $name(v)
            }
            fn as_intern_id(&self) -> salsa::InternId {
                self.0
            }
        }
    };
}

#[derive(Debug)]
pub struct ItemLoc<N: AstNode> {
    ast_id: AstId<N>,
}

impl<N: AstNode> PartialEq for ItemLoc<N> {
    fn eq(&self, other: &Self) -> bool {
        self.ast_id == other.ast_id
    }
}
impl<N: AstNode> Eq for ItemLoc<N> {}
impl<N: AstNode> Hash for ItemLoc<N> {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        self.ast_id.hash(hasher);
    }
}

impl<N: AstNode> Clone for ItemLoc<N> {
    fn clone(&self) -> ItemLoc<N> {
        ItemLoc {
            ast_id: self.ast_id,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct LocationCtx<DB> {
    db: DB,
    file_id: FileId,
}

impl<'a> LocationCtx<&'a dyn DefDatabase> {
    pub(crate) fn new(
        db: &'a dyn DefDatabase,
        file_id: FileId,
    ) -> LocationCtx<&'a dyn DefDatabase> {
        LocationCtx { db, file_id }
    }

    pub(crate) fn to_def<N, DEF>(self, ast: &N) -> DEF
    where
        N: AstNode,
        DEF: AstItemDef<N>,
    {
        DEF::from_ast(self, ast)
    }
}

pub(crate) trait AstItemDef<N: AstNode>: salsa::InternKey + Clone {
    fn intern(db: &dyn DefDatabase, loc: ItemLoc<N>) -> Self;
    fn lookup_intern(self, db: &dyn DefDatabase) -> ItemLoc<N>;

    fn from_ast(ctx: LocationCtx<&dyn DefDatabase>, ast: &N) -> Self {
        let items = ctx.db.ast_id_map(ctx.file_id);
        let item_id = items.ast_id(ast);
        Self::from_ast_id(ctx, item_id)
    }

    fn from_ast_id(ctx: LocationCtx<&dyn DefDatabase>, ast_id: FileAstId<N>) -> Self {
        let loc = ItemLoc {
            ast_id: ast_id.with_file_id(ctx.file_id),
        };
        Self::intern(ctx.db, loc)
    }

    fn source(self, db: &dyn DefDatabase) -> InFile<N> {
        let loc = self.lookup_intern(db);
        let ast = loc.ast_id.to_node(db);
        InFile::new(loc.ast_id.file_id, ast)
    }

    fn file_id(self, db: &dyn DefDatabase) -> FileId {
        self.lookup_intern(db).ast_id.file_id
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct FunctionId(salsa::InternId);
impl_intern_key!(FunctionId);

impl AstItemDef<ast::FunctionDef> for FunctionId {
    fn intern(db: &dyn DefDatabase, loc: ItemLoc<ast::FunctionDef>) -> Self {
        db.intern_function(loc)
    }
    fn lookup_intern(self, db: &dyn DefDatabase) -> ItemLoc<ast::FunctionDef> {
        db.lookup_intern_function(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StructId(salsa::InternId);
impl_intern_key!(StructId);

impl AstItemDef<ast::StructDef> for StructId {
    fn intern(db: &dyn DefDatabase, loc: ItemLoc<ast::StructDef>) -> Self {
        db.intern_struct(loc)
    }

    fn lookup_intern(self, db: &dyn DefDatabase) -> ItemLoc<ast::StructDef> {
        db.lookup_intern_struct(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeAliasId(salsa::InternId);
impl_intern_key!(TypeAliasId);

impl AstItemDef<ast::TypeAliasDef> for TypeAliasId {
    fn intern(db: &dyn DefDatabase, loc: ItemLoc<ast::TypeAliasDef>) -> Self {
        db.intern_type_alias(loc)
    }
    fn lookup_intern(self, db: &dyn DefDatabase) -> ItemLoc<ast::TypeAliasDef> {
        db.lookup_intern_type_alias(self)
    }
}
