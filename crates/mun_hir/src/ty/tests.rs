use crate::fixture::WithFixture;
use crate::{
    diagnostics::DiagnosticSink, expr::BodySourceMap, mock::MockDatabase, HirDisplay,
    InferenceResult, ModuleDef, Package,
};
use std::{fmt::Write, sync::Arc};

#[test]
fn use_() {
    infer_snapshot(
        r#"
    //- /bar.mun
    use package::Foo;
    pub struct Bar(Foo);

    //- /mod.mun
    pub use foo::Foo; // Re-export a child's definition

    struct Baz;

    //- /foo.mun
    use package::{bar::Bar, Baz};

    pub struct Foo {
        baz: Baz, // Can use private definitions from any of its ancestors
    }

    pub fn foo_from_bar(bar: Bar) -> Foo {
        bar.0
    }
    "#,
    )
}

#[test]
fn private_access() {
    infer_snapshot(
        r#"
    //- /foo.mun
    struct Foo {};
    struct Bar(i64);
    struct Baz;
    fn foo() {}
    type FooBar = Foo;

    pub struct PubFoo {};
    pub struct PubBar(i64);
    pub struct PubBaz;
    pub fn pub_foo() {}
    pub type PubFooBar = PubFoo;

    pub(super) struct PubSupFoo {};
    pub(super) struct PubSupBar(i64);
    pub(super) struct PubSupBaz;
    pub(super) fn pub_sup_foo() {}
    pub(super) type PubSupFooBar = PubSupFoo;

    pub(package) struct PubPackageFoo {};
    pub(package) struct PubPackageBar(i64);
    pub(package) struct PubPackageBaz;
    pub(package) fn pub_package_foo() {}
    pub(package) type PubPackageFooBar = PubPackageFoo;

    //- /bar.mun
    fn main() {
        let a = package::foo::Foo {}; // private access
        let a = package::foo::Bar(3); // private access
        let a = package::foo::Baz; // private access
        let a = package::foo::FooBar{}}; // private access

        let a = super::foo::Foo {}; // private access
        let a = super::foo::Bar(3); // private access
        let a = super::foo::Baz; // private access
        let a = super::foo::FooBar{}; // private access

        package::foo::foo(); // private access
        super::foo::foo(); // private access

        let a = package::foo::PubFoo {};
        let a = package::foo::PubBar(3);
        let a = package::foo::PubBaz;
        let a = package::foo::PubFooBar{};

        let a = super::foo::PubFoo {};
        let a = super::foo::PubBar(3);
        let a = super::foo::PubBaz;
        let a = super::foo::PubFooBar{};

        package::foo::pub_foo();
        super::foo::pub_foo();

        let a = package::foo::PubSupFoo {};
        let a = package::foo::PubSupBar(3);
        let a = package::foo::PubSupBaz;
        let a = package::foo::PubSupFooBar{};

        let a = super::foo::PubSupFoo {};
        let a = super::foo::PubSupBar(3);
        let a = super::foo::PubSupBaz;
        let a = super::foo::PubSupFooBar{};

        package::foo::pub_sup_foo();
        super::foo::pub_sup_foo();

        let a = package::foo::PubPackageFoo {};
        let a = package::foo::PubPackageBar(3);
        let a = package::foo::PubPackageBaz;
        let a = package::foo::PubPackageFooBar{};

        let a = super::foo::PubPackageFoo {};
        let a = super::foo::PubPackageBar(3);
        let a = super::foo::PubPackageBaz;
        let a = super::foo::PubPackageFooBar{};

        package::foo::pub_package_foo();
        super::foo::pub_package_foo();
    }

    //- /foo/baz.mun
    fn main() {
        let a = package::foo::Foo {};
        let a = package::foo::Bar(3);
        let a = package::foo::Baz;
        let a = package::foo::FooBar{};

        let a = super::Foo {};
        let a = super::Bar(3);
        let a = super::Baz;
        let a = super::FooBar{};

        package::foo::foo();
        super::foo();
    }

    //- /mod.mun
    fn main() {
        let a = package::foo::Foo {}; // private access
        let a = package::foo::Bar(3); // private access
        let a = package::foo::Baz; // private access
        let a = package::foo::FooBar{}; // private access

        let a = foo::Foo {}; // private access
        let a = foo::Bar(3); // private access
        let a = foo::Baz; // private access
        let a = foo::FooBar{}; // private access

        package::foo::foo(); // private access
        foo::foo(); // private access

        let a = package::foo::PubSupFoo {};
        let a = package::foo::PubSupBar(3);
        let a = package::foo::PubSupBaz;
        let a = package::foo::PubSupFooBar{};

        let a = foo::PubSupFoo {};
        let a = foo::PubSupBar(3);
        let a = foo::PubSupBaz;
        let a = foo::PubSupFooBar{};

        package::foo::pub_sup_foo();
        foo::pub_sup_foo();
    }
    "#,
    )
}

#[test]
fn scoped_path() {
    infer_snapshot(
        r"
    //- /mod.mun
    struct Foo;

    fn main() -> self::Foo {
        Foo
    }

    fn bar() -> Foo {
        super::Foo  // undefined value
    }

    fn baz() -> Foo {
        package::Foo
    }

    //- /foo.mun
    struct Foo;

    fn bar() -> Foo {
        super::Foo  // mismatched type
    }

    fn baz() -> package::Foo {
        super::Foo
    }

    fn nested() -> self::Foo {
        package::foo::Foo
    }
    ",
    )
}

#[test]
fn comparison_not_implemented_for_struct() {
    infer_snapshot(
        r"
    struct Foo;

    fn main() -> bool {
        Foo == Foo
    }",
    )
}

#[test]
fn infer_literals() {
    infer_snapshot(
        r"
        fn integer() -> i32 {
            0
        }

        fn large_unsigned_integer() -> u128 {
            0
        }

        fn with_let() -> u16 {
            let b = 4;
            let a = 4;
            a
        }
    ",
    )
}

#[test]
fn infer_suffix_literals() {
    infer_snapshot(
        r"
    fn main(){
        123;
        123u8;
        123u16;
        123u32;
        123u64;
        123u128;
        1_000_000_u32;
        123i8;
        123i16;
        123i32;
        123i64;
        123i128;
        1_000_000_i32;
        1_000_123.0e-2;
        1_000_123.0e-2f32;
        1_000_123.0e-2f64;
        9999999999999999999999999999999999999999999_f64;
    }

    fn add(a:u32) -> u32 {
        a + 12u32
    }

    fn errors() {
        0b22222; // invalid literal
        0b00010_f32; // non-10 base f64
        0o71234_f32; // non-10 base f64
        1234_foo; // invalid suffix
        1234.0_bar; // invalid suffix
        9999999999999999999999999999999999999999999; // too large
        256_u8; // literal out of range for `u8`
        128_i8; // literal out of range for `i8`
        12712371237123_u32; // literal out of range `u32`
        9999999999999999999999999; // literal out of range `i32`
    }
    ",
    )
}

#[test]
fn infer_invalid_struct_type() {
    infer_snapshot(
        r"
    fn main(){
        let a = Foo {b: 3};
    }",
    )
}

#[test]
fn infer_conditional_return() {
    infer_snapshot(
        r#"
    fn foo(a:int)->i32 {
        if a > 4 {
            return 4;
        }
        a
    }

    fn bar(a:i32)->i32 {
        if a > 4 {
            return 4;
        } else {
            return 1;
        }
    }
    "#,
    )
}

#[test]
fn infer_return() {
    infer_snapshot(
        r#"
    fn test()->i32 {
        return; // error: mismatched type
        return 5;
    }
    "#,
    )
}

#[test]
fn infer_basics() {
    infer_snapshot(
        r#"
    fn test(a:i32, b:f64, c:never, d:bool) -> bool {
        a;
        b;
        c;
        d
    }
    "#,
    )
}

#[test]
fn infer_branching() {
    infer_snapshot(
        r#"
    fn test() {
        let a = if true { 3 } else { 4 }
        let b = if true { 3 }               // Missing else branch
        let c = if true { 3; }
        let d = if true { 5 } else if false { 3 } else { 4 }
        let e = if true { 5.0 } else { 5 }  // Mismatched branches
    }
    "#,
    )
}

#[test]
fn void_return() {
    infer_snapshot(
        r#"
    fn bar() {
        let a = 3;
    }
    fn foo(a:i32) {
        let c = bar()
    }
    "#,
    )
}

#[test]
fn place_expressions() {
    infer_snapshot(
        r#"
    fn foo(a:i32) {
        a += 3;
        3 = 5; // error: invalid left hand side of expression
    }
    "#,
    )
}

#[test]
fn update_operators() {
    infer_snapshot(
        r#"
    fn foo(a:i32, b:f64) {
        a += 3;
        a -= 3;
        a *= 3;
        a /= 3;
        a %= 3;
        b += 3.0;
        b -= 3.0;
        b *= 3.0;
        b /= 3.0;
        b %= 3.0;
        a *= 3.0; // mismatched type
        b *= 3; // mismatched type
    }
    "#,
    )
}

#[test]
fn infer_unary_ops() {
    infer_snapshot(
        r#"
    fn foo(a: i32, b: bool) {
        a = -a;
        b = !b;
    }
        "#,
    )
}

#[test]
fn invalid_unary_ops() {
    infer_snapshot(
        r#"
    fn bar(a: f64, b: bool) {
        a = !a; // mismatched type
        b = -b; // mismatched type
    }
        "#,
    )
}

#[test]
fn infer_loop() {
    infer_snapshot(
        r#"
    fn foo() {
        loop {}
    }
    "#,
    )
}

#[test]
fn infer_break() {
    infer_snapshot(
        r#"
    fn foo()->i32 {
        break; // error: not in a loop
        loop { break 3; break 3.0; } // error: mismatched type
        let a:i32 = loop { break 3.0; } // error: mismatched type
        loop { break 3; }
        let a:i32 = loop { break loop { break 3; } }
        loop { break loop { break 3.0; } } // error: mismatched type
    }
    "#,
    )
}

#[test]
fn infer_while() {
    infer_snapshot(
        r#"
    fn foo() {
        let n = 0;
        while n < 3 { n += 1; };
        while n < 3 { n += 1; break; };
        while n < 3 { break 3; };   // error: break with value can only appear in a loop
        while n < 3 { loop { break 3; }; };
    }
    "#,
    )
}

#[test]
fn invalid_binary_ops() {
    infer_snapshot(
        r#"
    fn foo() {
        let b = false;
        let n = 1;
        let _ = b + n; // error: invalid binary operation
    }
    "#,
    )
}

#[test]
fn struct_decl() {
    infer_snapshot(
        r#"
    struct Foo;
    struct(gc) Bar {
        f: f64,
        i: i32,
    }
    struct(value) Baz(f64, i32);


    fn main() {
        let foo: Foo;
        let bar: Bar;
        let baz: Baz;
    }
    "#,
    )
}

#[test]
fn struct_lit() {
    infer_snapshot(
        r#"
    struct Foo;
    struct Bar {
        a: f64,
    }
    struct Baz(f64, i32);

    fn main() {
        let a: Foo = Foo;
        let b: Bar = Bar { a: 1.23, };
        let c = Baz(1.23, 1);

        let a = Foo{}; // error: mismatched struct literal kind. expected `unit struct`, found `record`
        let a = Foo(); // error: mismatched struct literal kind. expected `unit struct`, found `tuple`
        let b = Bar; // error: mismatched struct literal kind. expected `record`, found `unit struct`
        let b = Bar(); // error: mismatched struct literal kind. expected `record`, found `tuple`
        let b = Bar{}; // error: missing record fields: a
        let c = Baz; // error: mismatched struct literal kind. expected `tuple`, found `unit struct`
        let c = Baz{}; // error: mismatched struct literal kind. expected `tuple`, found `record`
        let c = Baz(); // error: this tuple struct literal has 2 fields but 0 fields were supplied
    }
    "#,
    )
}

#[test]
fn struct_field_index() {
    infer_snapshot(
        r#"
    struct Foo {
        a: f64,
        b: i32,
    }
    struct Bar(f64, i32)
    struct Baz;

    fn main() {
        let foo = Foo { a: 1.23, b: 4 };
        foo.a
        foo.b
        foo.c // error: attempted to access a non-existent field in a struct.
        let bar = Bar(1.23, 4);
        bar.0
        bar.1
        bar.2 // error: attempted to access a non-existent field in a struct.
        let baz = Baz;
        baz.a // error: attempted to access a non-existent field in a struct.
        let f = 1.0
        f.0; // error: attempted to access a field on a primitive type.
    }
    "#,
    )
}

#[test]
fn primitives() {
    infer_snapshot(
        r#"
    fn unsigned_primitives(a: u8, b: u16, c: u32, d: u64, e: u128, f: usize, g: u32) -> u8 { a }
    fn signed_primitives(a: i8, b: i16, c: i32, d: i64, e: i128, f: isize, g: i32) -> i8 { a }
    fn float_primitives(a: f32, b: f64, c: f64) -> f32 { a }
    "#,
    )
}

#[test]
fn extern_fn() {
    infer_snapshot(
        r#"
    extern fn foo(a:i32, b:i32) -> i32;
    fn main() {
        foo(3,4);
    }

    extern fn with_body() {}    // extern functions cannot have bodies

    struct S;
    extern fn with_non_primitive(s:S);  // extern functions can only have primitives as parameters
    extern fn with_non_primitive_return() -> S;  // extern functions can only have primitives as parameters
    "#,
    )
}

#[test]
fn infer_type_alias() {
    infer_snapshot(
        r#"
    type Foo = i32;
    type Bar = Foo;
    type Baz = UnknownType;  // error: undefined type

    fn main(a: Foo) {
        let b: Bar = a;
    }
    "#,
    )
}

#[test]
fn recursive_alias() {
    infer_snapshot(
        r#"
    struct Foo {}
    type Foo = Foo;

    type A = B;
    type B = A;

    fn main() {
        let a: Foo;  // error: unknown type
        let b: A;    // error: unknown type
        let c: B;    // error: unknown type
    }
    "#,
    )
}

fn infer_snapshot(text: &str) {
    let text = text.trim().replace("\n    ", "\n");
    insta::assert_snapshot!(insta::_macro_support::AutoName, infer(&text), &text);
}

fn infer(content: &str) -> String {
    let db = MockDatabase::with_files(content);

    let mut acc = String::new();

    let mut infer_def = |infer_result: Arc<InferenceResult>,
                         body_source_map: Arc<BodySourceMap>| {
        let mut types = Vec::new();

        for (pat, ty) in infer_result.type_of_pat.iter() {
            let syntax_ptr = match body_source_map.pat_syntax(pat) {
                Some(sp) => sp.map(|ast| ast.syntax_node_ptr()),
                None => continue,
            };
            types.push((syntax_ptr, ty));
        }

        for (expr, ty) in infer_result.type_of_expr.iter() {
            let syntax_ptr = match body_source_map.expr_syntax(expr) {
                Some(sp) => {
                    sp.map(|ast| ast.either(|it| it.syntax_node_ptr(), |it| it.syntax_node_ptr()))
                }
                None => continue,
            };
            types.push((syntax_ptr, ty));
        }

        // Sort ranges for consistency
        types.sort_by_key(|(src_ptr, _)| {
            (src_ptr.value.range().start(), src_ptr.value.range().end())
        });
        for (src_ptr, ty) in &types {
            let node = src_ptr.value.to_node(&src_ptr.file_syntax(&db));

            let (range, text) = (
                src_ptr.value.range(),
                node.text().to_string().replace("\n", " "),
            );
            write!(
                acc,
                "{} '{}': {}\n",
                range,
                ellipsize(text, 15),
                ty.display(&db)
            )
            .unwrap();
        }
    };

    let mut diags = String::new();

    let mut diag_sink = DiagnosticSink::new(|diag| {
        write!(diags, "{}: {}\n", diag.highlight_range(), diag.message()).unwrap();
    });

    for item in Package::all(&db)
        .iter()
        .flat_map(|pkg| pkg.modules(&db))
        .flat_map(|module| module.declarations(&db))
    {
        match item {
            ModuleDef::Function(fun) => {
                let source_map = fun.body_source_map(&db);
                let infer_result = fun.infer(&db);

                fun.diagnostics(&db, &mut diag_sink);

                infer_def(infer_result, source_map);
            }
            ModuleDef::TypeAlias(item) => {
                item.diagnostics(&db, &mut diag_sink);
            }
            _ => {}
        }
    }

    drop(diag_sink);

    acc.truncate(acc.trim_end().len());
    diags.truncate(diags.trim_end().len());
    [diags, acc].join("\n").trim().to_string()
}

fn ellipsize(mut text: String, max_len: usize) -> String {
    if text.len() <= max_len {
        return text;
    }
    let ellipsis = "...";
    let e_len = ellipsis.len();
    let mut prefix_len = (max_len - e_len) / 2;
    while !text.is_char_boundary(prefix_len) {
        prefix_len += 1;
    }
    let mut suffix_len = max_len - e_len - prefix_len;
    while !text.is_char_boundary(text.len() - suffix_len) {
        suffix_len += 1;
    }
    text.replace_range(prefix_len..text.len() - suffix_len, ellipsis);
    text
}
