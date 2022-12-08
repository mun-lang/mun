use mun_runtime::Runtime;

fn main() {
    // Safety: We assume that the library that is loaded is a valid munlib
    let builder = Runtime::builder("main.munlib");
    let mut runtime = unsafe { builder.finish() }.expect("Failed to spawn Runtime");

    let result: bool = runtime.invoke("random_bool", ()).unwrap();
    println!("random bool: {}", result);
}
