use mun_runtime::Runtime;

extern "C" fn random() -> i64 {
    let result = std::time::Instant::now().elapsed().subsec_nanos() as i64;
    println!("random: {}", result);
    result
}

fn main() {
    // Safety: We assume that the library that is loaded is a valid munlib
    let builder =
        Runtime::builder("main.munlib").insert_fn("random", random as extern "C" fn() -> i64);
    let mut runtime = unsafe { builder.finish() }.expect("Failed to spawn Runtime");

    let result: bool = runtime.invoke("random_bool", ()).unwrap();
    println!("random_bool: {}", result);
}
