use crate::code_model::DefWithBody;
use crate::expr::{Expr, Pat, PatId, Statement};
use crate::{
    arena::{Arena, RawId},
    expr::{Body, ExprId},
    HirDatabase, Name,
};
use rustc_hash::FxHashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScopeId(RawId);
impl_arena_id!(ScopeId);

#[derive(Debug, PartialEq, Eq)]
pub struct ExprScopes {
    body: Arc<Body>,
    scopes: Arena<ScopeId, ScopeData>,
    scope_by_expr: FxHashMap<ExprId, ScopeId>,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ScopeEntry {
    name: Name,
    pat: PatId,
}

impl ScopeEntry {
    pub(crate) fn name(&self) -> &Name {
        &self.name
    }

    pub(crate) fn pat(&self) -> PatId {
        self.pat
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ScopeData {
    parent: Option<ScopeId>,
    entries: Vec<ScopeEntry>,
}

impl ExprScopes {
    pub(crate) fn expr_scopes_query(db: &impl HirDatabase, def: DefWithBody) -> Arc<ExprScopes> {
        let body = db.body_hir(def);
        let res = ExprScopes::new(body);
        Arc::new(res)
    }

    fn new(body: Arc<Body>) -> ExprScopes {
        let mut scopes = ExprScopes {
            body: body.clone(),
            scopes: Arena::default(),
            scope_by_expr: FxHashMap::default(),
        };
        let root = scopes.root_scope();
        scopes.add_params_bindings(root, body.params().iter().map(|p| &p.0));
        compute_expr_scopes(body.body_expr(), &body, &mut scopes, root);
        scopes
    }

    pub(crate) fn entries(&self, scope: ScopeId) -> &[ScopeEntry] {
        &self.scopes[scope].entries
    }

    pub(crate) fn scope_chain<'a>(
        &'a self,
        scope: Option<ScopeId>,
    ) -> impl Iterator<Item = ScopeId> + 'a {
        std::iter::successors(scope, move |&scope| self.scopes[scope].parent)
    }

    pub(crate) fn scope_for(&self, expr: ExprId) -> Option<ScopeId> {
        self.scope_by_expr.get(&expr).copied()
    }

    pub(crate) fn scope_by_expr(&self) -> &FxHashMap<ExprId, ScopeId> {
        &self.scope_by_expr
    }

    fn root_scope(&mut self) -> ScopeId {
        self.scopes.alloc(ScopeData {
            parent: None,
            entries: vec![],
        })
    }

    fn new_scope(&mut self, parent: ScopeId) -> ScopeId {
        self.scopes.alloc(ScopeData {
            parent: Some(parent),
            entries: vec![],
        })
    }

    fn add_bindings(&mut self, body: &Body, scope: ScopeId, pat: PatId) {
        match &body[pat] {
            Pat::Bind { name, .. } => {
                // bind can have a sub pattern, but it's actually not allowed
                // to bind to things in there
                let entry = ScopeEntry {
                    name: name.clone(),
                    pat,
                };
                self.scopes[scope].entries.push(entry)
            }
            p => p.walk_child_pats(|pat| self.add_bindings(body, scope, pat)),
        }
    }

    fn add_params_bindings<'a>(&mut self, scope: ScopeId, params: impl Iterator<Item = &'a PatId>) {
        let body = Arc::clone(&self.body);
        params.for_each(|pat| self.add_bindings(&body, scope, *pat));
    }

    fn set_scope(&mut self, node: ExprId, scope: ScopeId) {
        self.scope_by_expr.insert(node, scope);
    }
}

fn compute_block_scopes(
    statements: &[Statement],
    tail: Option<ExprId>,
    body: &Body,
    scopes: &mut ExprScopes,
    mut scope: ScopeId,
) {
    for stmt in statements {
        match stmt {
            Statement::Let {
                pat, initializer, ..
            } => {
                if let Some(expr) = initializer {
                    scopes.set_scope(*expr, scope);
                    compute_expr_scopes(*expr, body, scopes, scope);
                }
                scope = scopes.new_scope(scope);
                scopes.add_bindings(body, scope, *pat);
            }
            Statement::Expr(expr) => {
                scopes.set_scope(*expr, scope);
                compute_expr_scopes(*expr, body, scopes, scope);
            }
        }
    }
    if let Some(expr) = tail {
        compute_expr_scopes(expr, body, scopes, scope);
    }
}

fn compute_expr_scopes(expr: ExprId, body: &Body, scopes: &mut ExprScopes, scope: ScopeId) {
    scopes.set_scope(expr, scope);
    match &body[expr] {
        Expr::Block { statements, tail } => {
            compute_block_scopes(&statements, *tail, body, scopes, scope);
        }
        e => e.walk_child_exprs(|e| compute_expr_scopes(e, body, scopes, scope)),
    };
}
