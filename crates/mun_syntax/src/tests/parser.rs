use crate::SourceFile;

fn snapshot_test(text: &str) {
    let text = text.trim().replace("\n    ", "\n");
    let file = SourceFile::parse(&text);
    insta::assert_snapshot!(insta::_macro_support::AutoName, file.debug_dump(), &text);
}

#[test]
fn empty() {
    snapshot_test(r#""#);
}

#[test]
fn function() {
    snapshot_test(
        r#"
    // Source file comment

    // Comment that belongs to the function
    fn a() {}
    fn b(value:number) {}
    pub fn d() {}
    pub fn c()->never {}
    fn b(value:number)->number {}"#,
    );
}

#[test]
fn block() {
    snapshot_test(
        r#"
    fn foo() {
        let a;
        let b:i32;
        let c:string;
    }"#,
    );
}

#[test]
fn literals() {
    snapshot_test(
        r#"
    fn foo() {
        let a = true;
        let b = false;
        let c = 1;
        let d = 1.12;
        let e = "Hello, world!"
    }
    "#,
    );
}

#[test]
fn struct_def() {
    snapshot_test(
        r#"
    struct Foo      // error: expected a ';', or a '{'
    struct Foo;
    struct Foo;;    // error: expected a declaration
    struct Foo {}
    struct Foo {};
    struct Foo {,}; // error: expected a field declaration
    struct Foo {
        a: float,
    }
    struct Foo {
        a: float,
        b: int,
    };
    struct Foo()
    struct Foo();
    struct Foo(,);  // error: expected a type
    struct Foo(float)
    struct Foo(float,);
    struct Foo(float, int)
    "#,
    )
}

#[test]
fn unary_expr() {
    snapshot_test(
        r#"
    fn foo() {
        let a = --3;
        let b = !!true;
    }
    "#,
    )
}

#[test]
fn binary_expr() {
    snapshot_test(
        r#"
    fn foo() {
        let a = 3+4*5
        let b = 3*4+10/2
    }
    "#,
    )
}

#[test]
fn expression_statement() {
    snapshot_test(
        r#"
    fn foo() {
        let a = "hello"
        let b = "world"
        let c
        b = "Hello, world!"
        !-5+2*(a+b);
        -3
    }
    "#,
    )
}

#[test]
fn function_calls() {
    snapshot_test(
        r#"
    fn bar(i:number) { }
    fn foo(i:number) {
      bar(i+1)
    }
    "#,
    )
}

#[test]
fn patterns() {
    snapshot_test(
        r#"
    fn main(_:number) {
       let a = 0;
       let _ = a;
    }
    "#,
    )
}

#[test]
fn arithmetic_operands() {
    snapshot_test(
        r#"
    fn main() {
        let _ = a + b;
        let _ = a - b;
        let _ = a * b;
        let _ = a / b;
        let _ = a % b;
        let _ = a << b;
        let _ = a >> b;
        let _ = a & b;
        let _ = a | b;
        let _ = a ^ b;
    }
    "#,
    )
}

#[test]
fn assignment_operands() {
    snapshot_test(
        r#"
    fn main() {
        let a = b;
        a += b;
        a -= b;
        a *= b;
        a /= b;
        a %= b;
        a <<= b;
        a >>= b;
        a &= b;
        a |= b;
        a ^= b;
    }
    "#,
    )
}

#[test]
fn compare_operands() {
    snapshot_test(
        r#"
    fn main() {
        let _ = a == b;
        let _ = a == b;
        let _ = a != b;
        let _ = a < b;
        let _ = a > b;
        let _ = a <= b;
        let _ = a >= b;
    }
    "#,
    )
}

#[test]
fn logic_operands() {
    snapshot_test(
        r#"
    fn main() {
        let _ = a || b;
        let _ = a && b;
    }
    "#,
    )
}

#[test]
fn if_expr() {
    snapshot_test(
        r#"
    fn bar() {
        if true {};
        if true {} else {};
        if true {} else if false {} else {};
        if {true} {} else {}
    }
    "#,
    );
}

#[test]
fn block_expr() {
    snapshot_test(
        r#"
    fn bar() {
        {3}
    }
    "#,
    );
}

#[test]
fn return_expr() {
    snapshot_test(
        r#"
    fn foo() {
        return;
        return 50;
    }
    "#,
    )
}

#[test]
fn loop_expr() {
    snapshot_test(
        r#"
    fn foo() {
        loop {}
    }"#,
    )
}

#[test]
fn break_expr() {
    snapshot_test(
        r#"
    fn foo() {
        break;
        if break { 3; }
        if break 4 { 3; }
    }
    "#,
    )
}

#[test]
fn while_expr() {
    snapshot_test(
        r#"
    fn foo() {
        while true {};
        while { true } {};
    }
    "#,
    )
}

#[test]
fn struct_lit() {
    snapshot_test(
        r#"
    fn foo() {
        U;
        S {};
        S { x, y: 32, };
        S { x: 32, y: 64 };
        TupleStruct { 0: 1 };
        T(1.23);
        T(1.23, 4,)
    }
    "#,
    )
}

#[test]
fn struct_field_index() {
    snapshot_test(
        r#"
    fn main() {
        foo.a
        foo.a.b
        foo.0
        foo.0.1
        foo.10
        foo.01  // index: .0
        foo.0 1 // index: .0 
        foo.a.0
    }
    "#,
    )
}

#[test]
fn memory_type_specifier() {
    snapshot_test(
        r#"
    struct Foo {};
    struct(gc) Baz {};
    struct(value) Baz {};
    struct() Err1 {};    // error: expected memory type specifier
    struct(foo) Err2 {}; // error: expected memory type specifier
    "#,
    )
}

#[test]
fn visibility() {
    snapshot_test(
        r#"
    pub struct Foo {};
    pub(package) struct(gc) Baz {};
    pub(super) fn foo() {}
    pub(package) fn bar() {}
    pub fn baz() {}
    "#,
    )
}

#[test]
fn extern_fn() {
    snapshot_test(
        r#"
    pub extern fn foo();
    "#,
    )
}
