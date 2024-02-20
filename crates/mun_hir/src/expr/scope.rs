use std::sync::Arc;

use rustc_hash::FxHashMap;

use crate::{
    arena::{Arena, Idx},
    expr::{Body, Expr, ExprId, Pat, PatId, Statement},
    ids::DefWithBodyId,
    DefDatabase, Name,
};

/// The ID of a scope in an `ExprScopes`
pub type LocalScopeId = Idx<ScopeData>;

#[derive(Debug, PartialEq, Eq)]
pub struct ExprScopes {
    scopes: Arena<ScopeData>,
    scope_by_expr: FxHashMap<ExprId, LocalScopeId>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ScopeEntry {
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
pub struct ScopeData {
    parent: Option<LocalScopeId>,
    entries: Vec<ScopeEntry>,
}

impl ExprScopes {
    pub(crate) fn expr_scopes_query(db: &dyn DefDatabase, def: DefWithBodyId) -> Arc<ExprScopes> {
        let body = db.body(def);
        Arc::new(ExprScopes::new(&body))
    }

    fn new(body: &Body) -> ExprScopes {
        let mut scopes = ExprScopes {
            scopes: Arena::default(),
            scope_by_expr: FxHashMap::default(),
        };
        let root = scopes.root_scope();
        scopes.add_params_bindings(body, root, body.params().iter().map(|p| &p.0));
        compute_expr_scopes(body.body_expr(), body, &mut scopes, root);
        scopes
    }

    pub(crate) fn entries(&self, scope: LocalScopeId) -> &[ScopeEntry] {
        &self.scopes[scope].entries
    }

    pub(crate) fn scope_chain(
        &'_ self,
        scope: Option<LocalScopeId>,
    ) -> impl Iterator<Item = LocalScopeId> + '_ {
        std::iter::successors(scope, move |&scope| self.scopes[scope].parent)
    }

    pub(crate) fn scope_for(&self, expr: ExprId) -> Option<LocalScopeId> {
        self.scope_by_expr.get(&expr).copied()
    }

    pub(crate) fn scope_by_expr(&self) -> &FxHashMap<ExprId, LocalScopeId> {
        &self.scope_by_expr
    }

    fn root_scope(&mut self) -> LocalScopeId {
        self.scopes.alloc(ScopeData {
            parent: None,
            entries: vec![],
        })
    }

    fn new_scope(&mut self, parent: LocalScopeId) -> LocalScopeId {
        self.scopes.alloc(ScopeData {
            parent: Some(parent),
            entries: vec![],
        })
    }

    fn add_bindings(&mut self, body: &Body, scope: LocalScopeId, pat: PatId) {
        match &body[pat] {
            Pat::Bind { name, .. } => {
                // bind can have a sub pattern, but it's actually not allowed
                // to bind to things in there
                let entry = ScopeEntry {
                    name: name.clone(),
                    pat,
                };
                self.scopes[scope].entries.push(entry);
            }
            p => p.walk_child_pats(|pat| self.add_bindings(body, scope, pat)),
        }
    }

    fn add_params_bindings<'a>(
        &mut self,
        body: &Body,
        scope: LocalScopeId,
        params: impl Iterator<Item = &'a PatId>,
    ) {
        params.for_each(|pat| self.add_bindings(body, scope, *pat));
    }

    fn set_scope(&mut self, node: ExprId, scope: LocalScopeId) {
        self.scope_by_expr.insert(node, scope);
    }
}

fn compute_block_scopes(
    statements: &[Statement],
    tail: Option<ExprId>,
    body: &Body,
    scopes: &mut ExprScopes,
    mut scope: LocalScopeId,
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

fn compute_expr_scopes(expr: ExprId, body: &Body, scopes: &mut ExprScopes, scope: LocalScopeId) {
    scopes.set_scope(expr, scope);
    match &body[expr] {
        Expr::Block { statements, tail } => {
            compute_block_scopes(statements, *tail, body, scopes, scope);
        }
        e => e.walk_child_exprs(|e| compute_expr_scopes(e, body, scopes, scope)),
    };
}
