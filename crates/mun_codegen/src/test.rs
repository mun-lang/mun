use std::cell::RefCell;

use inkwell::{context::Context, OptimizationLevel};
use mun_db::Upcast;
use mun_hir::{diagnostics::DiagnosticSink, HirDatabase};
use mun_hir_input::{SourceDatabase, WithFixture};
use mun_target::spec::Target;

use crate::{
    code_gen::{AssemblyBuilder, CodeGenContext},
    ir::{file::gen_file_ir, file_group::gen_file_group_ir},
    mock::MockDatabase,
    CodeGenDatabase,
};

#[test]
fn array_index_assign() {
    test_snapshot_unoptimized(
        "array_index_assign",
        r"
    pub fn main() {
        let a = [1,2,3,4,]
        a[1] = 100
    }
    ",
    );
}

#[test]
fn array_index() {
    test_snapshot(
        "array_index",
        r"
    pub fn main() -> i8 {
        let a = [1,2,3,4,]
        a[3]
    }
    ",
    );
}

#[test]
fn array_literal() {
    test_snapshot_unoptimized(
        "array_literal",
        r"
    pub fn main() {
        let a = [1,2,3,4,]
    }
    ",
    );
}

#[test]
fn multi_file() {
    test_snapshot(
        "multi_file",
        r"
    //- /mod.mun
    pub fn main() -> i32 {
        foo::get_value()
    }

    //- /foo.mun
    pub(super) fn get_value() -> i32 {
        3
    }
    ",
    );
}

#[test]
fn issue_262() {
    test_snapshot(
        "issue_262",
        r"
    fn foo() -> i32 {
        let bar = {
            let b = 3;
            return b + 3;
        };

        // This code will never be executed
        let a = 3 + 4;
        a
    }",
    );
}

#[test]
fn issue_225() {
    test_snapshot(
        "issue_225",
        r#"
    struct Num {
        value: i64,
    }

    pub fn foo(b: i64) {
        Num { value: b }.value;
    }

    pub fn bar(b: i64) {
        { let a = Num { value: b }; a}.value;
    }
        "#,
    );
}

#[test]
fn issue_228_never_if() {
    test_snapshot(
        "issue_228_never_if",
        r#"
    pub  fn fact(n: usize) -> usize {
   	    if n == 0 {return 1} else {return n * (n-1)}
   	    return 2;
    }
    "#,
    );
}

#[test]
fn issue_228() {
    test_snapshot(
        "issue_228",
        r#"
    pub  fn fact(n: usize) -> usize {
   	    if n == 0 {return 1} else {n * (n-1)}
    }
    "#,
    );
}

#[test]
fn issue_128() {
    test_snapshot(
        "issue_128",
        r#"
    // resources/script.mun
    extern fn thing(n: i32);
    extern fn print(n: i32) -> i32;

    pub fn main() {
        // 1st
        print(1);
        thing(5);

        // 2nd
        print(2);
        thing(78);
    }
    "#,
    );
}

#[test]
fn issue_133() {
    test_snapshot(
        "issue_133",
        r#"
    fn do_the_things(n: i32) -> i32 {
        n + 7
    }
    
    pub fn main() {
        do_the_things(3);
    }
    "#,
    );
}

#[test]
fn literal_types() {
    test_snapshot_unoptimized(
        "literal_types",
        r"
    pub fn main(){
        let a = 123;
        let a = 123u8;
        let a = 123u16;
        let a = 123u32;
        let a = 123u64;
        let a = 123u128;
        let a = 1_000_000_u32;
        let a = 123i8;
        let a = 123i16;
        let a = 123i32;
        let a = 123i64;
        let a = 123123123123123123123123123123123i128;
        let a = 1_000_000_i32;
        let a = 1_000_123.0e-2;
        let a = 1_000_123.0e-2f32;
        let a = 1_000_123.0e-2f64;
    }

    pub fn add(a:u32) -> u32 {
        a + 12u32
    }",
    );
}

#[test]
fn function() {
    test_snapshot(
        "function",
        r#"
    pub fn main() {
    }
    "#,
    );
}

#[test]
fn return_type() {
    test_snapshot(
        "return_type",
        r#"
    pub fn main() -> i32 {
      0
    }
    "#,
    );
}

#[test]
fn function_arguments() {
    test_snapshot(
        "function_arguments",
        r#"
    pub fn main(a:i32) -> i32 {
      a
    }
    "#,
    );
}

#[test]
fn assignment_op_bool() {
    test_snapshot(
        "assignment_op_bool",
        r#"
    pub fn assign(a: bool, b: bool) -> bool {
        a = b;
        a
    }
    // TODO: Add errors
    // a += b;
    // a *= b;
    // a -= b;
    // a /= b;
    // a %= b;
    "#,
    );
}

#[test]
fn logic_op_bool() {
    test_snapshot(
        "logic_op_bool",
        r#"
    pub fn and(a: bool, b: bool) -> bool {
        a && b
    }
    pub fn or(a: bool, b: bool) -> bool {
        a || b
    }    
    "#,
    );
}

#[test]
fn assignment_op_struct() {
    test_snapshot(
        "assignment_op_struct",
        r#"
    pub struct(value) Value(i32, i32);
    pub struct(gc) Heap(f64, f64);

    pub fn assign_value(a: Value, b: Value) -> Value {
        a = b;
        a
    }

    pub fn assign_heap(a: Heap, b: Heap) -> Heap {
        a = b;
        a
    }
    // TODO: Add errors
    // a += b;
    // a *= b;
    // a -= b;
    // a /= b;
    // a %= b;
    "#,
    );
}

macro_rules! test_number_operator_types {
    ($(
        $ty:ident
     ),+) => {
        $(
            paste::item! {
                #[test]
                fn [<assignment_op_ $ty>]() {
                    test_snapshot(
                        &format!("assignment_op_{ty}", ty = stringify!($ty)),
                        &format!(r#"
    pub fn assign(a: {ty}, b: {ty}) -> {ty} {{
        a = b;
        a
    }}
    pub fn assign_add(a: {ty}, b: {ty}) -> {ty} {{
        a += b;
        a
    }}
    pub fn assign_subtract(a: {ty}, b: {ty}) -> {ty} {{
        a -= b;
        a
    }}
    pub fn assign_multiply(a: {ty}, b: {ty}) -> {ty} {{
        a *= b;
        a
    }}
    pub fn assign_divide(a: {ty}, b: {ty}) -> {ty} {{
        a /= b;
        a
    }}
    pub fn assign_remainder(a: {ty}, b: {ty}) -> {ty} {{
        a %= b;
        a
    }}
                        "#, ty = stringify!($ty),
                    ));
                }

                #[test]
                fn [<arithmetic_op_ $ty>]() {
                    test_snapshot(
                        &format!("arithmetic_op_{ty}", ty = stringify!($ty)),
                        &format!(r#"
    pub fn add(a: {ty}, b: {ty}) -> {ty} {{ a + b }}
    pub fn subtract(a: {ty}, b: {ty}) -> {ty} {{ a - b }}
    pub fn multiply(a: {ty}, b: {ty}) -> {ty} {{ a * b }}
    pub fn divide(a: {ty}, b: {ty}) -> {ty} {{ a / b }}
    pub fn remainder(a: {ty}, b: {ty}) -> {ty} {{ a % b }}
                        "#, ty = stringify!($ty),
                    ));
                }
            }
        )+
    };
}

test_number_operator_types!(f32, f64, i8, i16, i32, i64, i128, u8, u16, u32, u64, u128);

macro_rules! test_compare_operator_types {
    ($(
        $ty:ident
     ),+) => {
        $(
            paste::item! {
                #[test]
                fn [<compare_op_ $ty>]() {
                    test_snapshot(
                        &format!("compare_op_{ty}", ty = stringify!($ty)),
                        &format!(r#"
    pub fn equals(a: {ty}, b: {ty}) -> bool {{ a == b }}
    pub fn not_equal(a: {ty}, b: {ty}) -> bool {{ a != b}}
    pub fn less(a: {ty}, b: {ty}) -> bool {{ a < b }}
    pub fn less_equal(a: {ty}, b: {ty}) -> bool {{ a <= b }}
    pub fn greater(a: {ty}, b: {ty}) -> bool {{ a > b }}
    pub fn greater_equal(a: {ty}, b: {ty}) -> bool {{ a >= b }}
                        "#, ty = stringify!($ty),
                    ));
                }
            }
        )+
    };
}

test_compare_operator_types!(bool, f32, f64, i8, i16, i32, i64, i128, u8, u16, u32, u64, u128);

macro_rules! test_negate_operator_types  {
    ($(
        $ty:ident
     ),+) => {
        $(
            paste::item! {
                #[test]
                fn [<negate_op_ $ty>]() {
                    test_snapshot(
                        &format!("negate_op_{ty}", ty = stringify!($ty)),
                        &format!(r#"
    pub fn negate(a: {ty}) -> {ty} {{ -a }}
                        "#, ty = stringify!($ty),
                    ));
                }
            }
        )+
    };
}

test_negate_operator_types!(f32, f64, i8, i16, i32, i64, i128);

macro_rules! test_bit_operator_types  {
    ($(
        $ty:ident
     ),+) => {
        $(
            paste::item! {
                #[test]
                fn [<assign_bit_op_ $ty>]() {
                    test_snapshot(
                        &format!("assign_bit_op_{ty}", ty = stringify!($ty)),
                        &format!(r#"
    pub fn assign_bitand(a: {ty}, b: {ty}) -> {ty} {{
        a &= b;
        a
    }}
    pub fn assign_bitor(a: {ty}, b: {ty}) -> {ty} {{
        a |= b;
        a
    }}
    pub fn assign_bitxor(a: {ty}, b: {ty}) -> {ty} {{
        a ^= b;
        a
    }}
                        "#, ty = stringify!($ty),
                    ));
                }

                #[test]
                fn [<bit_op_ $ty>]() {
                    test_snapshot(
                        &format!("bit_op_{ty}", ty = stringify!($ty)),
                    &format!(r#"
    pub fn not(a: {ty}) -> {ty} {{ !a }}
    pub fn bitand(a: {ty}, b: {ty}) -> {ty} {{ a & b }}
    pub fn bitor(a: {ty}, b: {ty}) -> {ty} {{ a | b }}
    pub fn bitxor(a: {ty}, b: {ty}) -> {ty} {{ a ^ b }}
                        "#, ty = stringify!($ty),
                    ));
                }
            }
        )+
    };
}

test_bit_operator_types!(bool, i8, i16, i32, i64, i128, u8, u16, u32, u64, u128);

macro_rules! test_shift_operator_types  {
    ($(
        $ty:ident
     ),+) => {
        $(
            paste::item! {
                #[test]
                fn [<assign_shift_op_ $ty>]() {
                    test_snapshot(
                        &format!("assign_shift_op_{ty}", ty = stringify!($ty)),
                        &format!(r#"
    pub fn assign_leftshift(a: {ty}, b: {ty}) -> {ty} {{
        a <<= b;
        a
    }}
    pub fn assign_rightshift(a: {ty}, b: {ty}) -> {ty} {{
        a >>= b;
        a
    }}
                        "#, ty = stringify!($ty),
                    ));
                }

                #[test]
                fn [<shift_op_ $ty>]() {
                    test_snapshot(
                        &format!("shift_op_{ty}", ty = stringify!($ty)),
                        &format!(r#"
    pub fn leftshift(a: {ty}, b: {ty}) -> {ty} {{ a << b }}
    pub fn rightshift(a: {ty}, b: {ty}) -> {ty} {{ a >> b }}
                        "#, ty = stringify!($ty),
                    ));
                }
            }
        )+
    };
}

test_shift_operator_types!(i8, i16, i32, i64, i128, u8, u16, u32, u64, u128);

#[test]
fn let_statement() {
    test_snapshot(
        "let_statement",
        r#"
    pub fn main(a:i32) -> i32 {
      let b = a+1
      b
    }
    "#,
    );
}

#[test]
fn invalid_binary_ops() {
    test_snapshot(
        "invalid_binary_ops",
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
        "update_operators",
        r#"
    pub fn add(a:i32, b:i32) -> i32 {
      let result = a
      result += b
      result
    }

    pub fn subtract(a:i32, b:i32) -> i32 {
      let result = a
      result -= b
      result
    }

    pub fn multiply(a:i32, b:i32) -> i32 {
      let result = a
      result *= b
      result
    }

    pub fn divide(a:i32, b:i32) -> i32 {
      let result = a
      result /= b
      result
    }

    pub fn remainder(a:i32, b:i32) -> i32 {
      let result = a
      result %= b
      result
    }
    "#,
    );
}

#[test]
fn update_parameter() {
    test_snapshot(
        "update_parameter",
        r#"
    pub fn add_three(a:i32) -> i32 {
      a += 3;
      a
    }
    "#,
    );
}

#[test]
fn function_calls() {
    test_snapshot(
        "function_calls",
        r#"
    fn add_impl(a:i32, b:i32) -> i32 {
        a+b
    }

    fn add(a:i32, b:i32) -> i32 {
      add_impl(a,b)
    }

    pub fn test() -> i32 {
      add(4,5)
      add_impl(4,5)
      add(4,5)
    }
    "#,
    );
}

#[test]
fn if_statement() {
    test_snapshot(
        "if_statement",
        r#"
    pub fn foo(a:i32) -> i32 {
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
    );
}

#[test]
fn void_return() {
    test_snapshot(
        "void_return",
        r#"
    fn bar() {
        let a = 3;
    }
    pub fn foo(a:i32) {
        let c = bar()
    }
    "#,
    );
}

#[test]
fn fibonacci() {
    test_snapshot(
        "fibonacci",
        r#"
    pub fn fibonacci(n:i32) -> i32 {
        if n <= 1 {
            n
        } else {
            fibonacci(n-1) + fibonacci(n-2)
        }
    }
    "#,
    );
}

#[test]
fn fibonacci_loop() {
    test_snapshot(
        "fibonacci_loop",
        r#"
    pub fn fibonacci(n:i32) -> i32 {
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
    );
}

#[test]
fn loop_issue_llvm13() {
    // A bug was surfaced by switching to LLVM13. When using a loop in code an exit
    // block was generated which didnt have a predecessor (because nobody jumped
    // to it), this caused LLVM13 to crash.
    test_snapshot(
        "loop_issue_llvm13",
        r#"
    pub fn issue() -> i32 {
        loop {
        }
    }
    "#,
    );
}

#[test]
fn shadowing() {
    test_snapshot(
        "shadowing",
        r#"
    pub fn foo(a:i32) -> i32 {
        let a = a+1;
        {
            let a = a+2;
        }
        a+3
    }

    pub fn bar(a:i32) -> i32 {
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
        "return_expr",
        r#"
    pub fn main() -> i32 {
        return 5;
        let a = 3; // Nothing regarding this statement should be in the IR
    }
    "#,
    );
}

#[test]
fn conditional_return_expr() {
    test_snapshot(
        "conditional_return_expr",
        r#"
    pub fn main(a:i32) -> i32 {
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
        "never_conditional_return_expr",
        r#"
    pub fn main(a:i32) -> i32 {
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
        "true_is_true",
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
        "loop_expr",
        r#"
    pub fn foo() {
        loop {}
    }
    "#,
    );
}

#[test]
fn loop_break_expr() {
    test_snapshot(
        "loop_break_expr",
        r#"
    pub fn foo(n:i32) -> i32 {
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
    );
}

#[test]
fn while_expr() {
    test_snapshot(
        "while_expr",
        r#"
    pub fn foo(n:i32) {
        while n<3 {
            n += 1;
        };

        // This will be completely optimized out
        while n<4 {
            break;
        };
    }
    "#,
    );
}

#[test]
fn struct_test() {
    test_snapshot_unoptimized(
        "struct_test",
        r#"
    struct(value) Bar(f64, i32, bool, Foo);
    struct(value) Foo { a: i32 };
    struct(value) Baz;
    pub fn foo() {
        let a: Foo = Foo { a: 5 };
        let b: Bar = Bar(1.23, a.a, true, a);
        let c: Baz = Baz;
    }
    "#,
    );
}

#[test]
fn field_expr() {
    test_snapshot(
        "field_expr",
        r#"
    pub struct(value) Bar(f64, Foo);
    pub struct(value) Foo { a: i32 };

    fn bar_1(bar: Bar) -> Foo {
        bar.1
    }

    fn foo_a(foo: Foo) -> i32 {
        foo.a
    }

    pub fn bar_1_foo_a(bar: Bar) -> i32 {
        foo_a(bar_1(bar))
    }

    pub fn main() -> i32 {
        let a: Foo = Foo { a: 5 };
        let b: Bar = Bar(1.23, a);
        let aa_lhs = a.a + 2;
        let aa_rhs = 2 + a.a;
        aa_lhs + aa_rhs
    }
    "#,
    );
}

#[test]
fn field_crash() {
    test_snapshot_unoptimized(
        "field_crash",
        r#"
    struct(gc) Foo { a: i32 };

    pub fn main(c:i32) -> i32 {
        let b = Foo { a: c + 5 }
        b.a
    }
    "#,
    );
}

#[test]
fn gc_struct() {
    test_snapshot_unoptimized(
        "gc_struct",
        r#"
    struct(gc) Foo { a: i32, b: i32 };

    pub fn foo() {
        let a = Foo { a: 3, b: 4 };
        a.b += 3;
        let b = a;
    }
    "#,
    );
}

#[test]
fn extern_fn() {
    test_snapshot(
        "extern_fn",
        r#"
    extern fn add(a:i32, b:i32) -> i32;
    pub fn main() {
        add(3,4);
    }
    "#,
    );
}

#[test]
fn private_fn_only() {
    test_snapshot(
        "private_fn_only",
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
        struct Foo(i32);

        pub fn foo(foo: Foo) -> i32 {
            foo.0
        }
        "#,
    );
    db.set_optimization_level(OptimizationLevel::Default);
    db.set_target(Target::host_target().unwrap());

    let module_group_id = db
        .module_partition()
        .group_for_file(file_id)
        .expect("could not find ModuleGroupId for file");

    {
        let events = db.log_executed(|| {
            db.target_assembly(module_group_id);
        });
        assert!(
            format!("{events:?}").contains("package_defs"),
            "{events:#?}"
        );
        assert!(format!("{events:?}").contains("assembly"), "{events:#?}");
    }

    db.set_optimization_level(OptimizationLevel::Aggressive);

    {
        let events = db.log_executed(|| {
            db.target_assembly(module_group_id);
        });
        println!("events: {events:?}");
        assert!(
            !format!("{events:?}").contains("package_defs"),
            "{events:#?}"
        );
        assert!(format!("{events:?}").contains("assembly"), "{events:#?}");
    }

    // TODO: Try to disconnect `group_ir` and `file_ir`
    // TODO: Add support for multiple files in a group
}

#[test]
fn nested_structs() {
    test_snapshot(
        "nested_structs",
        r#"
    pub struct(gc) GcStruct(f32, f32);
    pub struct(value) ValueStruct(f32, f32);

    pub struct(gc) GcWrapper(GcStruct, ValueStruct)
    pub struct(value) ValueWrapper(GcStruct, ValueStruct);

    pub fn new_gc_struct(a: f32, b: f32) -> GcStruct {
        GcStruct(a, b)
    }

    pub fn new_value_struct(a: f32, b: f32) -> ValueStruct {
        ValueStruct(a, b)
    }

    pub fn new_gc_wrapper(a: GcStruct, b: ValueStruct) -> GcWrapper {
        GcWrapper(a, b)
    }

    pub fn new_value_wrapper(a: GcStruct, b: ValueStruct) -> ValueWrapper {
        ValueWrapper(a, b)
    }
    "#,
    );
}

#[test]
fn nested_private_fn() {
    test_snapshot(
        "nested_private_fn",
        r#"
    fn nested_private_fn() -> i32 {
        1
    }

    fn private_fn() -> i32 {
        nested_private_fn()
    }

    pub fn main() -> i32 {
        private_fn()
    }
    "#,
    );
}

#[test]
fn nested_private_extern_fn() {
    test_snapshot(
        "nested_private_extern_fn",
        r#"
    extern fn extern_fn() -> f32;

    fn private_fn() -> f32 {
        extern_fn()
    }

    pub fn main() -> f32 {
        private_fn()
    }
    "#,
    );
}

#[test]
fn nested_private_recursive_fn() {
    test_snapshot(
        "nested_private_recursive_fn",
        r#"
    fn private_fn() -> f32 {
        private_fn()
    }

    pub fn main() -> f32 {
        private_fn()
    }
    "#,
    );
}

#[test]
fn nested_private_recursive_fn_with_args() {
    test_snapshot(
        "nested_private_recursive_fn_with_args",
        r#"
    extern fn other() -> i32;

    fn private_fn(a: i32) -> f32 {
        private_fn(a)
    }

    pub fn main() -> f32 {
        private_fn(other())
    }
    "#,
    );
}

fn test_snapshot(name: &str, text: &str) {
    test_snapshot_with_optimization(name, text, OptimizationLevel::Default);
}

fn test_snapshot_unoptimized(name: &str, text: &str) {
    test_snapshot_with_optimization(name, text, OptimizationLevel::None);
}

fn test_snapshot_with_optimization(name: &str, text: &str, opt: OptimizationLevel) {
    let mut db = MockDatabase::with_files(text);
    db.set_optimization_level(opt);
    db.set_target(Target::host_target().unwrap());

    // Build and extra diagnostics
    let messages = RefCell::new(Vec::new());
    let mut sink = DiagnosticSink::new(|diag| {
        let file_id = diag.source().file_id;
        let line_index = db.line_index(file_id);
        let source_root_id = db.file_source_root(file_id);
        let source_root = db.source_root(source_root_id);
        let relative_path = source_root.relative_path(file_id);
        let line_col = line_index.line_col(diag.highlight_range().start());
        messages.borrow_mut().push(format!(
            "{} ({}:{}): error: {}",
            relative_path,
            line_col.line + 1,
            line_col.col_utf16 + 1,
            diag.message()
        ));
    });
    for module in mun_hir::Package::all(db.upcast())
        .into_iter()
        .flat_map(|package| package.modules(db.upcast()))
    {
        module.diagnostics(db.upcast(), &mut sink);
    }
    drop(sink);
    let messages = messages.into_inner();

    // Setup code generation
    let llvm_context = Context::create();
    let code_gen = CodeGenContext::new(&llvm_context, db.upcast());
    let module_parition = db.module_partition();

    let value = if messages.is_empty() {
        itertools::Itertools::intersperse(module_parition.iter().map(|(module_group_id, module_group)| {
            let group_ir = gen_file_group_ir(&code_gen, module_group);
            let file_ir = gen_file_ir(&code_gen, &group_ir, module_group);

            let group_ir = group_ir.llvm_module.print_to_string().to_string();
            // println!("=== GROUP IR:\n {} ",&group_ir);
            let file_ir = file_ir.llvm_module.print_to_string().to_string();
            // println!("=== FILE IR:\n {} ",&file_ir);

            // To ensure that we test symbol generation
            let module_builder = AssemblyBuilder::new(&code_gen, &module_parition, module_group_id);
            let _obj_file = module_builder.build().expect("Failed to build object file");

            format!(
                "; == FILE IR ({}) =====================================\n{}\n; == GROUP IR ({}) ====================================\n{}",
                module_group.relative_file_path(),
                file_ir,
                module_group.relative_file_path(),
                group_ir
            )
        }),String::from("\n")).collect::<String>()
    } else {
        itertools::Itertools::intersperse(messages.into_iter(), String::from("\n"))
            .collect::<String>()
    };

    insta::assert_snapshot!(name, value, text);
}
