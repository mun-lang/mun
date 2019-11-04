use crate::SourceFile;

fn ok_snapshot_test(text: &str) {
    let text = text.trim().replace("\n    ", "\n");
    let file = SourceFile::parse(&text);
    let errors = file.errors();
    assert_eq!(
        &*errors,
        &[] as &[crate::SyntaxError],
        "There should be no errors\nAST:\n{}",
        file.debug_dump()
    );
    insta::assert_snapshot!(insta::_macro_support::AutoName, file.debug_dump(), &text);
}

#[test]
fn empty() {
    ok_snapshot_test(r#""#);
}

#[test]
fn function() {
    ok_snapshot_test(
        r#"
    // Source file comment

    // Comment that belongs to the function
    fn a() {}
    fn b(value:number) {}
    export fn c():never {}
    fn b(value:number):number {}"#,
    );
}

#[test]
fn block() {
    ok_snapshot_test(
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
    ok_snapshot_test(
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
fn unary_expr() {
    ok_snapshot_test(
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
    ok_snapshot_test(
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
    ok_snapshot_test(
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
    ok_snapshot_test(
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
    ok_snapshot_test(
        r#"
    fn main(_:number) {
       let a = 0;
       let _ = a;
    }
    "#,
    )
}

#[test]
fn compare_operands() {
    ok_snapshot_test(
        r#"
    fn main() {
        let _ = a==b;
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
fn if_expr() {
    ok_snapshot_test(
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
    ok_snapshot_test(
        r#"
    fn bar() {
        {3}
    }
    "#,
    );
}

#[test]
fn return_expr() {
    ok_snapshot_test(
        r#"
    fn foo() {
        return;
        return 50;
    }
    "#,
    )
}
