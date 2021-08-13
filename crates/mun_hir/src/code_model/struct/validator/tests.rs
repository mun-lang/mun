#[cfg(test)]
use crate::utils::tests::*;

#[test]
fn test_private_leak_struct_fields() {
    diagnostics_snapshot(
        r#"
    
    struct Foo(usize);
    pub struct Bar(usize);

    // valid, bar is public
    pub struct Baz {
        foo: Foo,
        pub bar: Bar,
    }

    // invalid, Foo is private
    pub struct FooBar {
        pub foo: Foo,
        pub bar: Bar,
    }

    // valid, FooBaz is private
    struct FooBaz {
        pub foo: Foo,
        pub bar: Bar,
    }

    pub(crate) struct BarBaz;

    // invalid, exporting pub(crate) to pub
    pub struct FooBarBaz {
        pub foo: Foo,
        pub bar: Bar,
    }
    "#,
    )
}

// this function needs to be declared in each file separately
// since insta's AutoName creates files in the directory from which
// the macro is called.
fn diagnostics_snapshot(text: &str) {
    let text = text.trim().replace("\n    ", "\n");
    insta::assert_snapshot!(insta::_macro_support::AutoName, diagnostics(&text), &text);
}
