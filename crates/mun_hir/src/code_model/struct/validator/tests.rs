#[cfg(test)]
use crate::utils::tests::*;

#[test]
fn test_private_leak_struct_fields() {
    insta::assert_snapshot!(diagnostics(
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
    "#),
    @r###"
    180..183: can't leak private type
    392..395: can't leak private type
    "###)
}
