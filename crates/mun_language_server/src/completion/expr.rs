use mun_hir::{
    method_resolution::{AssociationMode, MethodResolutionCtx},
    semantics::PathResolution,
    AssocItemId, ModuleDef,
};

use super::{CompletionContext, Completions, PathCompletionContext, PathExprContext, Qualified};

pub(super) fn complete_expr_path(
    result: &mut Completions,
    ctx: &CompletionContext<'_>,
    PathCompletionContext { qualified, .. }: &PathCompletionContext,
    _expr_ctx: &PathExprContext,
) {
    match qualified {
        Qualified::With {
            resolution: Some(PathResolution::Def(ModuleDef::Struct(s))),
            ..
        } => {
            let ty = s.ty(ctx.db);
            MethodResolutionCtx::new(ctx.db, ty.clone())
                .with_association(AssociationMode::WithoutSelf)
                .collect(|item, _visible| {
                    match item {
                        AssocItemId::FunctionId(f) => result.add_function(ctx, f.into(), None),
                    };
                    None::<()>
                });
        }
        Qualified::No => {
            // Iterate over all items in the current scope and add completions for them
            ctx.scope.visit_all_names(&mut |name, def| {
                result.add_resolution(ctx, name.to_string(), &def);
            });
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use crate::completion::{test_utils::completion_string, CompletionKind};

    #[test]
    fn test_local_scope() {
        insta::assert_snapshot!(completion_string(
            r#"
        fn foo() {
            let bar = 0;
            let foo_bar = 0;
            f$0
        }
        "#,
            Some(CompletionKind::Reference)
        ));
    }

    #[test]
    fn test_associate_function() {
        insta::assert_snapshot!(completion_string(
            r#"
        struct Foo;

        impl Foo {
            fn new() -> Self {
                Self
            }
        }

        fn foo() {
            let bar = Foo::$0;
        }
        "#,
            Some(CompletionKind::Reference)
        ));
    }
}
