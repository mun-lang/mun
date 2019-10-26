use crate::code_model::Function;
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
    fn source(self, db: &(impl DefDatabase)) -> Source<ast::FunctionDef> {
        self.id.source(db)
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
