use mun_runtime::Runtime;
use std::{cell::RefCell, env, rc::Rc};

fn main() {
    let lib_path = env::args().nth(1).expect("Expected path to a Mun library.");

    // Safety: We assume that the library that is loaded is a valid munlib
    let builder = Runtime::builder(lib_path);
    let mut runtime = unsafe { builder.finish() }
        .expect("Failed to spawn Runtime");

    loop {
        let arg: i64 = runtime.invoke("arg", ()).unwrap();
        let result: i64 = runtime.invoke("fibonacci", (arg,)).unwrap();
        println!("fibonacci({}) = {}", arg, result);
        
        unsafe { runtime.update() };
    }
}
