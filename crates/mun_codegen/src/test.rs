use crate::{mock::MockDatabase, IrDatabase};
use mun_hir::diagnostics::DiagnosticSink;
use mun_hir::line_index::LineIndex;
use mun_hir::Module;
use mun_hir::SourceDatabase;
use std::cell::RefCell;
use std::sync::Arc;

#[test]
fn function() {
    test_snapshot(
        r#"
    fn main() {
    }
    "#,
    );
}

#[test]
fn return_type() {
    test_snapshot(
        r#"
    fn main():int {
      0
    }
    "#,
    );
}

#[test]
fn function_arguments() {
    test_snapshot(
        r#"
    fn main(a:int):int {
      a
    }
    "#,
    );
}

#[test]
fn binary_expressions() {
    test_snapshot(
        r#"
    fn add(a:int, b:int):int {
      a+b
    }

    fn subtract(a:int, b:int):int {
      a-b
    }

    fn multiply(a:int, b:int):int {
      a*b
    }
    "#,
    );
}

#[test]
fn let_statement() {
    test_snapshot(
        r#"
    fn main(a:int):int {
      let b = a+1
      b
    }
    "#,
    );
}

#[test]
fn invalid_binary_ops() {
    test_snapshot(
        r#"
    fn main() {
      let a = 3+3.0;
      let b = 3.0+3;
    }
    "#,
    );
}

#[test]
fn update_operators() {
    test_snapshot(
        r#"
    fn add(a:int, b:int):int {
      let result = a
      result += b
      result
    }

    fn subtract(a:int, b:int):int {
      let result = a
      result -= b
      result
    }

    fn multiply(a:int, b:int):int {
      let result = a
      result *= b
      result
    }

    fn divide(a:int, b:int):int {
      let result = a
      result /= b
      result
    }
    "#,
    );
}

#[test]
fn update_parameter() {
    test_snapshot(
        r#"
    fn add_three(a:int):int {
      a += 3;
      a
    }
    "#,
    );
}

#[test]
fn function_calls() {
    test_snapshot(
        r#"
    fn add_impl(a:int, b:int):int {
        a+b
    }

    fn add(a:int, b:int):int {
      add_impl(a,b)
    }

    fn test():int {
      add(4,5)
      add_impl(4,5)
      add(4,5)
    }
    "#,
    );
}

#[test]
fn equality_operands() {
    test_snapshot(
        r#"
    fn equals(a:int, b:int):bool                { a == b }
    fn not_equals(a:int, b:int):bool            { a != b }
    fn less(a:int, b:int):bool                  { a < b }
    fn less_equal(a:int, b:int):bool            { a <= b }
    fn greater(a:int, b:int):bool               { a > b }
    fn greater_equal(a:int, b:int):bool         { a >= b }
    fn equalsf(a:float, b:float):bool           { a == b }
    fn not_equalsf(a:float, b:float):bool       { a != b }
    fn lessf(a:float, b:float):bool             { a < b }
    fn less_equalf(a:float, b:float):bool       { a <= b }
    fn greaterf(a:float, b:float):bool          { a > b }
    fn greater_equalf(a:float, b:float):bool    { a >= b }
    "#,
    );
}

#[test]
fn if_statement() {
    test_snapshot(
        r#"
    fn foo(a:int):int {
        let b = if a > 3 {
            let c = if a > 4 {
                a+1
            } else {
                a+3
            }
            c
        } else {
            a-1
        }
        b
    }
    "#,
    )
}

#[test]
fn void_return() {
    test_snapshot(
        r#"
    fn bar() {
        let a = 3;
    }
    fn foo(a:int) {
        let c = bar()
    }
    "#,
    )
}

#[test]
fn fibonacci() {
    test_snapshot(
        r#"
    fn fibonacci(n:int):int {
        if n <= 1 {
            n
        } else {
            fibonacci(n-1) + fibonacci(n-2)
        }
    }
    "#,
    )
}

#[test]
fn fibonacci_loop() {
    test_snapshot(
        r#"
    fn fibonacci(n:int):int {
        let a = 0;
        let b = 1;
        let i = 1;
        loop {
            if i > n {
                return a
            }
            let sum = a + b;
            a = b;
            b = sum;
            i += 1;
        }
    }
    "#,
    )
}

#[test]
fn shadowing() {
    test_snapshot(
        r#"
    fn foo(a:int):int {
        let a = a+1;
        {
            let a = a+2;
        }
        a+3
    }

    fn bar(a:int):int {
        let a = a+1;
        let a = {
            let a = a+2;
            a
        }
        a+3
    }
    "#,
    );
}

#[test]
fn return_expr() {
    test_snapshot(
        r#"
    fn main():int {
        return 5;
        let a = 3; // Nothing regarding this statement should be in the IR
    }
    "#,
    );
}

#[test]
fn conditional_return_expr() {
    test_snapshot(
        r#"
    fn main(a:int):int {
        if a > 4 {
            return a;
        }
        a - 1
    }
    "#,
    );
}

#[test]
fn never_conditional_return_expr() {
    test_snapshot(
        r#"
    fn main(a:int):int {
        if a > 4 {
            return a;
        } else {
            return a - 1;
        }
    }
    "#,
    );
}

#[test]
fn true_is_true() {
    test_snapshot(
        r#"
    fn test_true():bool {
        true
    }

    fn test_false():bool {
        false
    }"#,
    );
}

#[test]
fn loop_expr() {
    test_snapshot(
        r#"
    fn foo() {
        loop {}
    }
    "#,
    )
}

#[test]
fn loop_break_expr() {
    test_snapshot(
        r#"
    fn foo(n:int):int {
        loop {
            if n > 5 {
                break n;
            }
            if n > 10 {
                break 10;
            }
            n += 1;
        }
    }
    "#,
    )
}

#[test]
fn while_expr() {
    test_snapshot(
        r#"
    fn foo(n:int) {
        while n<3 {
            n += 1;
        };

        // This will be completely optimized out
        while n<4 {
            break;
        };
    }
    "#,
    )
}

fn test_snapshot(text: &str) {
    let text = text.trim().replace("\n    ", "\n");

    let (db, file_id) = MockDatabase::with_single_file(&text);

    let line_index: Arc<LineIndex> = db.line_index(file_id);
    let messages = RefCell::new(Vec::new());
    let mut sink = DiagnosticSink::new(|diag| {
        let line_col = line_index.line_col(diag.highlight_range().start());
        messages.borrow_mut().push(format!(
            "error {}:{}: {}",
            line_col.line + 1,
            line_col.col + 1,
            diag.message()
        ));
    });
    Module::from(file_id).diagnostics(&db, &mut sink);
    drop(sink);
    let messages = messages.into_inner();

    let name = if !messages.is_empty() {
        messages.join("\n")
    } else {
        format!(
            "{}",
            db.module_ir(file_id)
                .llvm_module
                .print_to_string()
                .to_string()
        )
    };
    insta::assert_snapshot!(insta::_macro_support::AutoName, name, &text);
}
