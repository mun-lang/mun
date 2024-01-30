use mun_syntax::ast;

use crate::{
    code_model::{Field, Function, Struct, TypeAlias},
    ids::{AssocItemLoc, Lookup},
    in_file::InFile,
    item_tree::{ItemTreeId, ItemTreeNode},
    DefDatabase, ItemLoc,
};

/// A trait implemented for items that can be related back to their source. The
/// [`HasSource::source`] method returns the source location of its instance.
pub trait HasSource {
    type Ast;

    /// Returns the source location of this instance.
    fn source(&self, db: &dyn DefDatabase) -> InFile<Self::Ast>;
}

impl<N: ItemTreeNode> HasSource for ItemTreeId<N> {
    type Ast = N::Source;

    fn source(&self, db: &dyn DefDatabase) -> InFile<Self::Ast> {
        let tree = db.item_tree(self.file_id);
        let ast_id_map = db.ast_id_map(self.file_id);
        let root = db.parse(self.file_id);
        let node = &tree[self.value];

        InFile::new(
            self.file_id,
            ast_id_map.get(node.ast_id()).to_node(&root.syntax_node()),
        )
    }
}

impl<N: ItemTreeNode> HasSource for ItemLoc<N> {
    type Ast = N::Source;

    fn source(&self, db: &dyn DefDatabase) -> InFile<Self::Ast> {
        self.id.source(db)
    }
}

impl<N: ItemTreeNode> HasSource for AssocItemLoc<N> {
    type Ast = N::Source;

    fn source(&self, db: &dyn DefDatabase) -> InFile<Self::Ast> {
        self.id.source(db)
    }
}

impl HasSource for Function {
    type Ast = ast::FunctionDef;
    fn source(&self, db: &dyn DefDatabase) -> InFile<Self::Ast> {
        self.id.lookup(db).source(db)
    }
}

impl HasSource for Struct {
    type Ast = ast::StructDef;
    fn source(&self, db: &dyn DefDatabase) -> InFile<Self::Ast> {
        self.id.lookup(db).source(db)
    }
}

impl HasSource for Field {
    type Ast = ast::RecordFieldDef;

    fn source(&self, db: &dyn DefDatabase) -> InFile<Self::Ast> {
        let src = self.parent.source(db);
        let file_id = src.file_id;
        let field_sources = if let ast::StructKind::Record(r) = src.value.kind() {
            r.fields().collect()
        } else {
            Vec::new()
        };

        let ast = field_sources
            .into_iter()
            .zip(self.parent.data(db).fields.iter())
            .find(|(_syntax, (id, _))| *id == self.id)
            .unwrap()
            .0;

        InFile::new(file_id, ast)
    }
}

impl HasSource for TypeAlias {
    type Ast = ast::TypeAliasDef;
    fn source(&self, db: &dyn DefDatabase) -> InFile<Self::Ast> {
        self.id.lookup(db).source(db)
    }
}
