use crate::with_fixture::WithFixture;
use crate::{item_tree::ItemTree, mock::MockDatabase, DefDatabase, Upcast};
use std::{fmt, sync::Arc};

fn item_tree(text: &str) -> Arc<ItemTree> {
    let (db, file_id) = MockDatabase::with_single_file(text);
    db.item_tree(file_id)
}

fn print_item_tree(text: &str) -> Result<String, fmt::Error> {
    let (db, file_id) = MockDatabase::with_single_file(text);
    let item_tree = db.item_tree(file_id);
    super::pretty::print_item_tree(db.upcast(), &item_tree)
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

    pub use foo;
    use super::bar;
    use super::*;
    use foo::{bar, baz::hello as world};
    "#
    )
    .unwrap());
}
