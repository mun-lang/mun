use mun_runtime::Runtime;
use std::{cell::RefCell, env, rc::Rc};

fn main() {
    let lib_path = env::args().nth(1).expect("Expected path to a Mun library.");

    let mut runtime = Runtime::builder(lib_path)
        .finish()
        .expect("Failed to spawn Runtime");

    loop {
        let arg: i64 = runtime.invoke("arg", ()).unwrap();
        let result: i64 = runtime.invoke("fibonacci", (arg,)).unwrap();
        println!("fibonacci({}) = {}", arg, result);
        runtime.update();
    }
}
