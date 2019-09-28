use crate::ast::{self, child_opt, children, AstChildren, AstNode};

pub trait ModuleItemOwner: AstNode {
    fn items(&self) -> AstChildren<ast::ModuleItem> {
        children(self)
    }
}

pub trait FunctionDefOwner: AstNode {
    fn functions(&self) -> AstChildren<ast::FunctionDef> {
        children(self)
    }
}

pub trait NameOwner: AstNode {
    fn name(&self) -> Option<ast::Name> {
        child_opt(self)
    }
}

pub trait TypeAscriptionOwner: AstNode {
    fn ascribed_type(&self) -> Option<ast::TypeRef> {
        child_opt(self)
    }
}

pub trait VisibilityOwner: AstNode {
    fn visibility(&self) -> Option<ast::Visibility> {
        child_opt(self)
    }
}

pub trait DocCommentsOwner: AstNode {}

pub trait ArgListOwner: AstNode {
    fn arg_list(&self) -> Option<ast::ArgList> {
        child_opt(self)
    }
}
