use crate::db::SourceDatabase;
use crate::expr::validator::ExprValidator;
use crate::{diagnostics::DiagnosticSink, ids::LocationCtx, mock::MockDatabase, Function};
use mun_syntax::{ast, AstNode};
use std::fmt::Write;

#[test]
fn test_uninitialized_access() {
    diagnostics_snapshot(
        r#"
    fn foo() {
        let a:int;
        let b = a + 3;
    }
    "#,
    )
}

#[test]
fn test_uninitialized_access_if() {
    diagnostics_snapshot(
        r#"
    fn foo() {
        let a:int;
        if true { a = 3; } else { a = 4; }
        let b = a + 4;  // correct, `a` is initialized either way
    }

    fn bar() {
        let a:int;
        if true { a = 3; }
        let b = a + 4;  // `a` is possibly-unitialized
    }

    fn baz() {
        let a:int;
        if true { return } else { a = 4 };
        let b = a + 4;  // correct, `a` is initialized either way
    }

    fn foz() {
        let a:int;
        if true { a = 4 } else { return };
        let b = a + 4;  // correct, `a` is initialized either way
    }

    fn boz() {
        let a:int;
        return;
        let b = a + 4;  // `a` is not initialized but this is dead code anyway
    }
    "#,
    )
}

#[test]
fn test_uninitialized_access_while() {
    diagnostics_snapshot(
        r#"
    fn foo(b:int) {
        let a:int;
        while b < 4 { b += 1; a = b; a += 1; }
        let c = a + 4;  // `a` is possibly-unitialized
    }
    "#,
    )
}

fn diagnostics(content: &str) -> String {
    let (db, file_id) = MockDatabase::with_single_file(content);
    let source_file = db.parse(file_id).ok().unwrap();

    let mut diags = String::new();

    let mut diag_sink = DiagnosticSink::new(|diag| {
        write!(diags, "{}: {}\n", diag.highlight_range(), diag.message()).unwrap();
    });

    let ctx = LocationCtx::new(&db, file_id);
    for node in source_file.syntax().descendants() {
        if let Some(def) = ast::FunctionDef::cast(node.clone()) {
            let fun = Function {
                id: ctx.to_def(&def),
            };
            ExprValidator::new(fun, &db, &mut diag_sink).validate_body();
        }
    }
    drop(diag_sink);
    diags
}

fn diagnostics_snapshot(text: &str) {
    let text = text.trim().replace("\n    ", "\n");
    insta::assert_snapshot!(insta::_macro_support::AutoName, diagnostics(&text), &text);
}
