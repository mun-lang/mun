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
            resolution: Some(resolution),
            ..
        } => {
            let ty = match resolution {
                PathResolution::Def(ModuleDef::Struct(st)) => Some(st.ty(ctx.db)),
                PathResolution::SelfType(imp) => Some(imp.self_ty(ctx.db)),
                _ => None,
            };

            if let Some(ty) = ty {
                MethodResolutionCtx::new(ctx.db, ty)
                    .with_association(AssociationMode::WithoutSelf)
                    .collect(|item, _visible| {
                        match item {
                            AssocItemId::FunctionId(f) => result.add_function(ctx, f.into(), None),
                        };
                        None::<()>
                    });
            }
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
    use crate::completion::{test_utils::completion_string};

    #[test]
    fn test_local_scope() {
        insta::assert_snapshot!(completion_string(
            r#"
        fn foo() {
            let bar = 0;
            let foo_bar = 0;
            f$0
        }
        "#
        ), @r###"
        lc foo_bar i32
        lc bar     i32
        fn foo     -> ()
        "###);
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
        "#
        ), @"fn new -> Foo");
    }

    #[test]
    fn test_parameter() {
        insta::assert_snapshot!(completion_string(
            r#"
        fn bar() {
            let a = 0;
            foo(f$0)
        }
        "#
        ), @r###"
        lc a   i32
        fn bar -> ()
        "###);
    }

    #[test]
    fn test_associated_self() {
        insta::assert_snapshot!(completion_string(
            r#"
            struct Foo;

        impl Foo {
            fn foo() {
                Self::$0
            }
        }
        "#,
        ), @"fn foo -> ()");
    }

    #[test]
    fn test_complete_self() {
        insta::assert_snapshot!(completion_string(
            r#"
            struct Foo;

        impl Foo {
            fn foo(self) {
                $0
            }
        }
        "#,
        ), @r###"
        lc self Foo
        sp Self
        st Foo
        "###);
    }
}
