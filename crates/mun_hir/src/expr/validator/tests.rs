#[cfg(test)]
use crate::utils::tests::*;

#[test]
fn test_uninitialized_access() {
    insta::assert_snapshot!(
        diagnostics(r#"
    fn foo() {
        let a:i64;
        let b = a + 3;
    }
    "#), @"38..39: use of possibly-uninitialized variable"
    );
}

#[test]
fn test_uninitialized_access_if() {
    insta::assert_snapshot!(diagnostics(
        r#"
    fn foo() {
        let a:i64;
        if true { a = 3; } else { a = 4; }
        let b = a + 4;  // correct, `a` is initialized either way
    }

    fn bar() {
        let a:i64;
        if true { a = 3; }
        let b = a + 4;  // `a` is possibly-unitialized
    }

    fn baz() {
        let a:i64;
        if true { return } else { a = 4 };
        let b = a + 4;  // correct, `a` is initialized either way
    }

    fn foz() {
        let a:i64;
        if true { a = 4 } else { return };
        let b = a + 4;  // correct, `a` is initialized either way
    }

    fn boz() {
        let a:i64;
        return;
        let b = a + 4;  // `a` is not initialized but this is dead code anyway
    }
    "#,
    ), @"191..192: use of possibly-uninitialized variable");
}

#[test]
fn test_uninitialized_access_while() {
    insta::assert_snapshot!(diagnostics(
        r#"
    fn foo(b:i64) {
        let a:i64;
        while b < 4 { b += 1; a = b; a += 1; }
        let c = a + 4;  // `a` is possibly-unitialized
    }
    "#,
    ), @"86..87: use of possibly-uninitialized variable");
}

#[test]
fn test_free_type_alias_without_type_ref() {
    insta::assert_snapshot!(diagnostics(
        r#"
    type Foo; // `Foo` must have a target type
    "#,
    ), @"0..9: free type alias without type ref");
}

#[test]
fn test_private_leak_function_return() {
    insta::assert_snapshot!(diagnostics(
        r#"
    struct Foo(usize);

    pub fn bar() -> Foo { // Foo is not public
        Foo(0)
    }

    pub fn baz(a: usize, b: usize) -> Foo {
        Foo(2)
    }

    pub struct FooBar(usize);

    pub fn FooBaz() -> FooBar {
        FooBar(0)
    }

    fn BarBaz() -> FooBar {
        FooBar(1)
    }
    "#,
    ), @r###"
    36..39: can't leak private type
    111..114: can't leak private type
    "###);
}

#[test]
fn test_private_leak_function_args() {
    insta::assert_snapshot!(diagnostics(
        r#"
    struct Foo(usize);

    pub fn bar(a: Foo, b: isize) -> usize{ // Foo is not public
        0
    }

    pub fn baz(a: isize, b: Foo) -> isize {
        -1
    }

    pub struct FooBar(usize);

    pub fn FooBaz(a: FooBar) -> FooBar {
        a
    }

    fn BarBaz(a: isize, b: FooBar) -> isize {
        a
    }
    "#,
    ), @r###"
    34..37: can't leak private type
    113..116: can't leak private type
    "###);
}

#[test]
fn test_private_leak_function_scoped() {
    insta::assert_snapshot!(diagnostics(
        r#"
    // Illegal, Bar has a smaller scope than this use statement
    pub(super) struct Bar;

    // Illegal, Bar has a smaller scope than this function
    pub fn baz() -> Bar {
        Bar
    }
    "#,
    ), @"155..158: can't leak private type");
}

// No errors, check https://github.com/mun-lang/mun/issues/339
#[test]
fn test_private_leak_alias() {
    insta::assert_snapshot!(diagnostics(
        r#"
    type Bar = usize;

    pub fn baz() -> Bar {
        0
    }
    "#,
    ), @"");
}
