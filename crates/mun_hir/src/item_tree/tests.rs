use std::fmt;

use mun_db::Upcast;
use mun_hir_input::WithFixture;

use crate::{mock::MockDatabase, DefDatabase, DiagnosticSink};

fn print_item_tree(text: &str) -> Result<String, fmt::Error> {
    let (db, file_id) = MockDatabase::with_single_file(text);
    let item_tree = db.item_tree(file_id);
    let mut result_str = super::pretty::print_item_tree(db.upcast(), &item_tree)?;
    let mut sink = DiagnosticSink::new(|diag| {
        result_str.push_str(&format!(
            "\n{:?}: {}",
            diag.highlight_range(),
            diag.message()
        ));
    });

    item_tree
        .diagnostics
        .iter()
        .for_each(|diag| diag.add_to(&db, &item_tree, &mut sink));

    drop(sink);
    Ok(result_str)
}

#[test]
fn top_level_items() {
    insta::assert_snapshot!(print_item_tree(
        r#"
    fn foo(a:i32, b:u8, c:String) -> i32 {}
    pub fn bar(a:i32, b:u8, c:String) ->  {}
    pub(super) fn bar(a:i32, b:u8, c:String) ->  {}
    pub(package) fn baz(a:i32, b:, c:String) ->  {}
    extern fn eval(a:String) -> bool;

    struct Foo {
        a: i32,
        b: u8,
        c: String,
    }
    struct Foo2 {
        a: i32,
        b: ,
        c: String,
    }
    struct Bar (i32, u32, String)
    struct Baz;

    type FooBar = Foo;
    type FooBar = package::Foo;
    "#
    )
    .unwrap());
}

#[test]
fn test_use() {
    insta::assert_snapshot!(print_item_tree(
        r#"
    pub use foo;
    use super::bar;
    use super::*;
    use foo::{bar as _, baz::hello as world};
        "#
    )
    .unwrap());
}

#[test]
fn test_impls() {
    insta::assert_snapshot!(print_item_tree(
        r#"
    impl Bar {
        fn foo(a:i32, b:u8, c:String) -> i32 {}
        pub fn bar(a:i32, b:u8, c:String) ->  {}
    }
    "#
    )
    .unwrap());
}

#[test]
fn test_duplicate_import() {
    insta::assert_snapshot!(print_item_tree(
        r#"
    use foo::Bar;
    use baz::Bar;

    struct Bar {}
    "#
    )
    .unwrap());
}
