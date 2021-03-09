use crate::{
    diagnostics::DiagnosticSink,
    expr::validator::{ExprValidator, TypeAliasValidator},
    mock::MockDatabase,
    with_fixture::WithFixture,
    ModuleDef, Package,
};
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

#[test]
fn test_free_type_alias_without_type_ref() {
    diagnostics_snapshot(
        r#"
    type Foo; // `Foo` must have a target type
    "#,
    )
}

fn diagnostics(content: &str) -> String {
    let (db, _file_id) = MockDatabase::with_single_file(content);

    let mut diags = String::new();

    let mut diag_sink = DiagnosticSink::new(|diag| {
        write!(diags, "{:?}: {}\n", diag.highlight_range(), diag.message()).unwrap();
    });

    for item in Package::all(&db)
        .iter()
        .flat_map(|pkg| pkg.modules(&db))
        .flat_map(|module| module.declarations(&db))
    {
        match item {
            ModuleDef::Function(item) => {
                ExprValidator::new(item, &db).validate_body(&mut diag_sink);
            }
            ModuleDef::TypeAlias(item) => {
                TypeAliasValidator::new(item, &db).validate_target_type_existence(&mut diag_sink);
            }
            _ => {}
        }
    }

    drop(diag_sink);
    diags
}

fn diagnostics_snapshot(text: &str) {
    let text = text.trim().replace("\n    ", "\n");
    insta::assert_snapshot!(insta::_macro_support::AutoName, diagnostics(&text), &text);
}
