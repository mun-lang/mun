use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use mun_runtime::{invoke_fn, StructRef};

mod util;

/// A benchmark test that runs fibonacci(n) for a number of samples and compares the performance
/// for calling the implementation between different languages.
pub fn fibonacci_benchmark(c: &mut Criterion) {
    // Perform setup (not part of the benchmark)
    let runtime = util::runtime_from_file("fibonacci.mun");
    let lua = util::lua_from_file("fibonacci.lua");
    let wasm = util::wasmer_from_file("fibonacci.wasm");

    let mut group = c.benchmark_group("fibonacci");

    // Iterate over a number of samples
    for i in [100i64, 200i64, 500i64, 1000i64, 4000i64, 8000i64].iter() {
        // Run Mun fibonacci
        group.bench_with_input(BenchmarkId::new("mun", i), i, |b, i| {
            b.iter(|| {
                let _: i64 = invoke_fn!(runtime, "main", *i).unwrap();
            })
        });

        // Run Rust fibonacci
        group.bench_with_input(BenchmarkId::new("rust", i), i, |b, i| {
            b.iter(|| fibonacci_main(*i))
        });

        // Run LuaJIT fibonacci
        group.bench_with_input(BenchmarkId::new("luajit", i), i, |b, i| {
            b.iter(|| {
                let f: mlua::Function = lua.globals().get("main").unwrap();
                let _: i64 = f.call(*i).unwrap();
            })
        });

        // Run Wasm fibonacci
        group.bench_with_input(BenchmarkId::new("wasm", i), i, |b, i| {
            b.iter(|| {
                wasm.call("main", &[(*i as i32).into()]).unwrap();
            })
        });
    }

    group.finish();

    fn fibonacci(n: i64) -> i64 {
        let mut a = 0;
        let mut b = 1;
        let mut i = 1;
        loop {
            if i > n {
                return a;
            }
            let sum = a + b;
            a = b;
            b = sum;
            i += 1;
        }
    }

    fn fibonacci_main(n: i64) -> i64 {
        fibonacci(n)
    }
}

/// A benchmark method to measure the relative overhead of calling a function from Rust for several
/// languages.
pub fn empty_benchmark(c: &mut Criterion) {
    // Perform setup (not part of the benchmark)
    let runtime = util::runtime_from_file("empty.mun");
    let lua = util::lua_from_file("empty.lua");
    let wasm = util::wasmer_from_file("empty.wasm");

    let mut group = c.benchmark_group("empty");

    group.bench_function("mun", |b| {
        b.iter(|| {
            let _: i64 = invoke_fn!(runtime, "empty", black_box(20i64)).unwrap();
        })
    });
    group.bench_function("rust", |b| b.iter(|| empty(black_box(20))));
    group.bench_function("luajit", |b| {
        b.iter(|| {
            let f: mlua::Function = lua.globals().get("empty").unwrap();
            let _: i64 = f.call(black_box(20)).unwrap();
        })
    });
    group.bench_function("wasm", |b| {
        b.iter(|| wasm.call("empty", &[black_box(20i64).into()]))
    });

    group.finish();

    pub fn empty(n: i64) -> i64 {
        n
    }
}

/// A benchmark method to measure the relative overhead of calling a function from Rust for several
/// languages.
pub fn struct_get_struct_benchmark(c: &mut Criterion) {
    // Perform setup (not part of the benchmark)
    let runtime = util::runtime_from_file("struct.mun");
    let mun_gc_parent: StructRef = invoke_fn!(runtime, "make_gc_parent").unwrap();
    let mun_value_parent: StructRef = invoke_fn!(runtime, "make_value_parent").unwrap();

    let rust_parent = RustParent {
        child: RustChild(-2.0, -1.0, 1.0, 2.0),
    };

    let mut group = c.benchmark_group("struct_get_struct");

    // Iterate over a number of samples
    for i in [100i64, 200i64, 500i64, 1000i64].iter() {
        // When marshalling, both `struct(gc)` and `struct(value)` are assigned on the heap, so we
        // only need to compare two cases:
        // - a `struct(gc)` child
        // - a `struct(value)` child

        // Marshal Mun `struct(gc)`
        group.bench_with_input(BenchmarkId::new("mun (gc)", i), i, |b, i| {
            b.iter(|| {
                for _ in 0..*i {
                    // TODO: Optimise `RefCell::borrow` cost for sequential marshalling
                    let _child = black_box(mun_gc_parent.get::<StructRef>("child").unwrap());
                    // TODO: Optimise `Drop` cost for temporary structs
                }
            })
        });

        // Marshal Mun `struct(value)`
        group.bench_with_input(BenchmarkId::new("mun (value)", i), i, |b, i| {
            b.iter(|| {
                for _ in 0..*i {
                    // TODO: Optimise allocation of `struct(value)` by using a memory arena
                    let _child = black_box(mun_value_parent.get::<StructRef>("child").unwrap());
                }
            })
        });

        // Run Rust fibonacci
        group.bench_with_input(BenchmarkId::new("rust", i), i, |b, i| {
            b.iter(|| {
                for _ in 0..*i {
                    let _child = black_box(&rust_parent.child);
                }
            })
        });
    }

    group.finish();

    struct RustChild(f32, f32, f32, f32);

    struct RustParent {
        child: RustChild,
    }
}

criterion_group!(
    benches,
    fibonacci_benchmark,
    empty_benchmark,
    struct_get_struct_benchmark
);
criterion_main!(benches);
