use crate::code_model::{Function, Struct, StructField};
use crate::ids::AstItemDef;
use crate::{DefDatabase, FileId, SourceDatabase};
use mun_syntax::{ast, AstNode, SyntaxNode};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Source<T> {
    pub file_id: FileId,
    pub ast: T,
}

pub trait HasSource {
    type Ast;
    fn source(self, db: &impl DefDatabase) -> Source<Self::Ast>;
}

impl HasSource for Function {
    type Ast = ast::FunctionDef;
    fn source(self, db: &impl DefDatabase) -> Source<ast::FunctionDef> {
        self.id.source(db)
    }
}

impl HasSource for Struct {
    type Ast = ast::StructDef;
    fn source(self, db: &impl DefDatabase) -> Source<ast::StructDef> {
        self.id.source(db)
    }
}

impl HasSource for StructField {
    type Ast = ast::RecordFieldDef;

    fn source(self, db: &impl DefDatabase) -> Source<ast::RecordFieldDef> {
        let src = self.parent.source(db);
        let file_id = src.file_id;
        let field_sources = if let ast::StructKind::Record(r) = src.ast.kind() {
            r.fields().collect()
        } else {
            Vec::new()
        };

        let ast = field_sources
            .into_iter()
            .zip(self.parent.data(db).fields.as_ref().unwrap().iter())
            .find(|(_syntax, (id, _))| *id == self.id)
            .unwrap()
            .0;

        Source { file_id, ast }
    }
}

impl<T> Source<T> {
    pub(crate) fn map<F: FnOnce(T) -> U, U>(self, f: F) -> Source<U> {
        Source {
            file_id: self.file_id,
            ast: f(self.ast),
        }
    }

    pub(crate) fn file_syntax(&self, db: &impl SourceDatabase) -> SyntaxNode {
        db.parse(self.file_id).tree().syntax().clone()
    }
}
