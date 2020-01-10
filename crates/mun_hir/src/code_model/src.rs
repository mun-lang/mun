use crate::code_model::{Function, Struct, StructField};
use crate::ids::AstItemDef;
use crate::in_file::InFile;
use crate::DefDatabase;
use mun_syntax::ast;

pub trait HasSource {
    type Ast;
    fn source(self, db: &impl DefDatabase) -> InFile<Self::Ast>;
}

impl HasSource for Function {
    type Ast = ast::FunctionDef;
    fn source(self, db: &impl DefDatabase) -> InFile<ast::FunctionDef> {
        self.id.source(db)
    }
}

impl HasSource for Struct {
    type Ast = ast::StructDef;
    fn source(self, db: &impl DefDatabase) -> InFile<ast::StructDef> {
        self.id.source(db)
    }
}

impl HasSource for StructField {
    type Ast = ast::RecordFieldDef;

    fn source(self, db: &impl DefDatabase) -> InFile<ast::RecordFieldDef> {
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
