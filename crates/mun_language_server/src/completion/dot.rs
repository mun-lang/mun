use mun_db::Upcast;

use super::{CompletionContext, Completions, DotAccess};

/// Complete dot accesses, i.e. fields. Adds `CompletionItems` to `result`.
pub(super) fn complete_dot(
    result: &mut Completions,
    ctx: &CompletionContext<'_>,
    dot_access: &DotAccess,
) {
    let receiver_ty = match dot_access {
        DotAccess {
            receiver_ty: Some(receiver_ty),
            ..
        } => receiver_ty,
        _ => return,
    };

    // Get all the fields of the expression
    if let Some(strukt) = receiver_ty.as_struct() {
        for field in strukt.fields(ctx.db.upcast()) {
            result.add_field(ctx, field);
        }
    };
}

#[cfg(test)]
mod tests {
    use crate::completion::{test_utils::completion_string, CompletionKind};

    #[test]
    fn test_struct_fields() {
        insta::assert_snapshot!(completion_string(
            r#"
        struct FooBar {
            foo: i32,
            bar: i32,
        };
        
        fn foo() {
            let bar = FooBar { foo: 0, bar: 0 };
            bar.$0
        }
        "#,
            Some(CompletionKind::Reference)
        ));
    }

    #[test]
    fn test_tuple_struct() {
        insta::assert_snapshot!(completion_string(
            r#"
        struct FooBar(i32, i32)
        
        fn foo() {
            let bar = FooBar(0,0);
            bar.$0
        }
        "#,
            Some(CompletionKind::Reference)
        ));
    }

    #[test]
    fn test_nested_struct() {
        insta::assert_snapshot!(completion_string(
            r#"
        struct Foo { baz: i32 }
        struct Bar(Foo)
        
        fn foo() {
            let bar = Bar(Foo{baz:0});
            bar.0.$0
        }
        "#,
            Some(CompletionKind::Reference)
        ));
    }

    #[test]
    fn test_incomplete_struct() {
        insta::assert_snapshot!(completion_string(
            r#"
        struct Foo { bar: i32 }
        
        fn foo() {
            let bar = Foo;
            bar.$0
        }
        "#,
            Some(CompletionKind::Reference)
        ));
    }
}
