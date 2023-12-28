use super::{CompletionContext, Completions};

/// Adds completions to `result` for unqualified path. Unqualified paths are simple names which do
/// not refer to anything outside of the current scope: local function names, variables, etc. E.g.:
/// ```mun
/// fn foo() {
///    let foo_bar = 3;
///    foo_$0
/// }
/// ```
pub(super) fn complete_unqualified_path(result: &mut Completions, ctx: &CompletionContext<'_>) {
    // Only complete trivial paths (e.g. foo, not ::foo)
    if !ctx.is_trivial_path {
        return;
    }

    // Iterate over all items in the current scope and add completions for them
    ctx.scope.visit_all_names(&mut |name, def| {
        result.add_resolution(ctx, name.to_string(), &def);
    });
}

#[cfg(test)]
mod tests {
    use crate::{completion::test_utils::completion_string, completion::CompletionKind};

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
}
