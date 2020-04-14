use crate::{mock::MockDatabase, IrDatabase, ModuleBuilder};
use hir::{diagnostics::DiagnosticSink, line_index::LineIndex, Module, SourceDatabase};
use inkwell::OptimizationLevel;
use mun_target::spec::Target;
use std::cell::RefCell;
use std::sync::Arc;

#[test]
fn issue_128() {
    test_snapshot(
        r#"
    // resources/script.mun
    extern fn thing(n: int);
    extern fn print(n: int) -> int;

    pub fn main() {
        // 1st
        print(1);
        thing(5);

        // 2nd
        print(2);
        thing(78);
    }
    "#,
    )
}

#[test]
fn issue_133() {
    test_snapshot(
        r#"
    fn do_the_things(n: int) -> int {
        n + 7
    }
    
    pub fn main() {
        do_the_things(3);
    }
    "#,
    );
}

#[test]
fn function() {
    test_snapshot(
        r#"
    pub fn main() {
    }
    "#,
    );
}

#[test]
fn return_type() {
    test_snapshot(
        r#"
    pub fn main() -> int {
      0
    }
    "#,
    );
}

#[test]
fn function_arguments() {
    test_snapshot(
        r#"
    pub fn main(a:int) -> int {
      a
    }
    "#,
    );
}

#[test]
fn binary_expressions() {
    test_snapshot(
        r#"
    pub fn add(a:int, b:int) -> int {
      a+b
    }

    pub fn subtract(a:int, b:int) -> int {
      a-b
    }

    pub fn multiply(a:int, b:int) -> int {
      a*b
    }
    "#,
    );
}

#[test]
fn let_statement() {
    test_snapshot(
        r#"
    pub fn main(a:int) -> int {
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
    pub fn main() {
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
    pub fn add(a:int, b:int) -> int {
      let result = a
      result += b
      result
    }

    pub fn subtract(a:int, b:int) -> int {
      let result = a
      result -= b
      result
    }

    pub fn multiply(a:int, b:int) -> int {
      let result = a
      result *= b
      result
    }

    pub fn divide(a:int, b:int) -> int {
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
    pub fn add_three(a:int) -> int {
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
    fn add_impl(a:int, b:int) -> int {
        a+b
    }

    fn add(a:int, b:int) -> int {
      add_impl(a,b)
    }

    pub fn test() -> int {
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
    pub fn equals(a:int, b:int) -> bool                { a == b }
    pub fn not_equals(a:int, b:int) -> bool            { a != b }
    pub fn less(a:int, b:int) -> bool                  { a < b }
    pub fn less_equal(a:int, b:int) -> bool            { a <= b }
    pub fn greater(a:int, b:int) -> bool               { a > b }
    pub fn greater_equal(a:int, b:int) -> bool         { a >= b }
    pub fn equalsf(a:float, b:float) -> bool           { a == b }
    pub fn not_equalsf(a:float, b:float) -> bool       { a != b }
    pub fn lessf(a:float, b:float) -> bool             { a < b }
    pub fn less_equalf(a:float, b:float) -> bool       { a <= b }
    pub fn greaterf(a:float, b:float) -> bool          { a > b }
    pub fn greater_equalf(a:float, b:float) -> bool    { a >= b }
    "#,
    );
}

#[test]
fn if_statement() {
    test_snapshot(
        r#"
    pub fn foo(a:int) -> int {
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
    pub fn foo(a:int) {
        let c = bar()
    }
    "#,
    )
}

#[test]
fn fibonacci() {
    test_snapshot(
        r#"
    pub fn fibonacci(n:int) -> int {
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
    pub fn fibonacci(n:int) -> int {
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
    pub fn foo(a:int) -> int {
        let a = a+1;
        {
            let a = a+2;
        }
        a+3
    }

    pub fn bar(a:int) -> int {
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
    pub fn main() -> int {
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
    pub fn main(a:int) -> int {
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
    pub fn main(a:int) -> int {
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
    pub fn test_true() -> bool {
        true
    }

    pub fn test_false() -> bool {
        false
    }"#,
    );
}

#[test]
fn loop_expr() {
    test_snapshot(
        r#"
    pub fn foo() {
        loop {}
    }
    "#,
    )
}

#[test]
fn loop_break_expr() {
    test_snapshot(
        r#"
    pub fn foo(n:int) -> int {
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
    pub fn foo(n:int) {
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

#[test]
fn struct_test() {
    test_snapshot_unoptimized(
        r#"
    struct(value) Bar(float, int, bool, Foo);
    struct(value) Foo { a: int };
    struct(value) Baz;
    pub fn foo() {
        let a: Foo = Foo { a: 5 };
        let b: Bar = Bar(1.23, a.a, true, a);
        let c: Baz = Baz;
    }
    "#,
    )
}

#[test]
fn field_expr() {
    test_snapshot(
        r#"
    struct(value) Bar(float, Foo);
    struct(value) Foo { a: int };

    fn bar_1(bar: Bar) -> Foo {
        bar.1
    }

    fn foo_a(foo: Foo) -> int {
        foo.a
    }

    pub fn bar_1_foo_a(bar: Bar) -> int {
        foo_a(bar_1(bar))
    }

    pub fn main() -> int {
        let a: Foo = Foo { a: 5 };
        let b: Bar = Bar(1.23, a);
        let aa_lhs = a.a + 2;
        let aa_rhs = 2 + a.a;
        aa_lhs + aa_rhs
    }
    "#,
    )
}

#[test]
fn field_crash() {
    test_snapshot_unoptimized(
        r#"
    struct(gc) Foo { a: int };

    pub fn main(c:int) -> int {
        let b = Foo { a: c + 5 }
        b.a
    }
    "#,
    )
}

#[test]
fn gc_struct() {
    test_snapshot_unoptimized(
        r#"
    struct(gc) Foo { a: int, b: int };

    pub fn foo() {
        let a = Foo { a: 3, b: 4 };
        a.b += 3;
        let b = a;
    }
    "#,
    )
}

#[test]
fn primitive_types() {
    test_snapshot(
        r#"
   pub fn add(a: u8, b: u8) -> u8 { a+b }
   pub fn less(a: u16, b: u16) -> bool { a<b }
   pub fn greater(a: u32, b: u32) -> bool { a>b }
   pub fn equal(a: u64, b: u64) -> bool { a==b }
   pub fn nequal(a: u128, b: u128) -> bool { a!=b }
   pub fn greater_equal(a: usize, b: usize) -> bool { a>=b }
   pub fn less_equal(a: uint, b: uint) -> bool { a<=b }

   pub fn iadd(a: i8, b: i8) -> i8 { a+b }
   pub fn iless(a: i16, b: i16) -> bool { a<b }
   pub fn igreater(a: i32, b: i32) -> bool { a>b }
   pub fn iequal(a: i64, b: i64) -> bool { a==b }
   pub fn inequal(a: i128, b: i128) -> bool { a!=b }
   pub fn igreater_equal(a: isize, b: isize) -> bool { a>=b }
   pub fn iless_equal(a: int, b: int) -> bool { a<=b }
    "#,
    )
}

#[test]
fn extern_fn() {
    test_snapshot(
        r#"
    extern fn add(a:int, b:int) -> int;
    pub fn main() {
        add(3,4);
    }
    "#,
    )
}

#[test]
fn private_fn_only() {
    test_snapshot(
        r#"
    fn private_main() {
        let a = 1;
    }
    "#,
    );
}

#[test]
fn incremental_compilation() {
    let (mut db, file_id) = MockDatabase::with_single_file(
        r#"
        struct Foo(int);

        pub fn foo(foo: Foo) -> int {
            foo.0
        }
        "#,
    );
    db.set_optimization_lvl(OptimizationLevel::Default);
    db.set_target(Target::host_target().unwrap());

    {
        let events = db.log_executed(|| {
            db.file_ir(file_id);
        });
        assert!(
            format!("{:?}", events).contains("group_ir"),
            "{:#?}",
            events
        );
        assert!(format!("{:?}", events).contains("file_ir"), "{:#?}", events);
    }

    db.set_optimization_lvl(OptimizationLevel::Aggressive);

    {
        let events = db.log_executed(|| {
            db.file_ir(file_id);
        });
        println!("events: {:?}", events);
        assert!(
            !format!("{:?}", events).contains("group_ir"),
            "{:#?}",
            events
        );
        assert!(format!("{:?}", events).contains("file_ir"), "{:#?}", events);
    }

    // TODO: Try to disconnect `group_ir` and `file_ir`
    // TODO: Add support for multiple files in a group
}

fn test_snapshot(text: &str) {
    test_snapshot_with_optimization(text, OptimizationLevel::Default);
}

fn test_snapshot_unoptimized(text: &str) {
    test_snapshot_with_optimization(text, OptimizationLevel::None);
}

fn test_snapshot_with_optimization(text: &str, opt: OptimizationLevel) {
    let text = text.trim().replace("\n    ", "\n");

    let (mut db, file_id) = MockDatabase::with_single_file(&text);
    db.set_optimization_lvl(opt);
    db.set_target(Target::host_target().unwrap());

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

    let module_builder =
        ModuleBuilder::new(&db, file_id).expect("Failed to initialize module builder");

    // The thread is named after the test case, so we can use it to name our snapshots.
    let thread_name = std::thread::current()
        .name()
        .expect("The current thread does not have a name.")
        .replace("test::", "");

    let group_ir_value = if !messages.is_empty() {
        "".to_owned()
    } else {
        format!(
            "{}",
            db.group_ir(file_id)
                .llvm_module
                .print_to_string()
                .to_string()
        )
    };

    let file_ir_value = if !messages.is_empty() {
        messages.join("\n")
    } else {
        format!(
            "{}",
            db.file_ir(file_id)
                .llvm_module
                .print_to_string()
                .to_string()
        )
    };

    // To ensure that we test symbol generation
    if messages.is_empty() {
        let _obj_file = module_builder.build().expect("Failed to build object file");
    }

    let value = format!(
        r"; == FILE IR =====================================
{}

; == GROUP IR ====================================
{}",
        file_ir_value, group_ir_value
    );
    insta::assert_snapshot!(thread_name, value, &text);
}
