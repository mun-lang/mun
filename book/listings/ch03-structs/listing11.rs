# extern crate mun_runtime;
use mun_runtime::{Runtime, StructRef};
use std::{cell::RefCell, env, rc::Rc};

fn main() {
    let lib_path = env::args().nth(1).expect("Expected path to a Mun library.");

    let runtime = Runtime::builder(lib_path)
        .finish()
        .expect("Failed to spawn Runtime");

    let a: StructRef = runtime.invoke("vector2_new", (-1.0f32, 1.0f32)).unwrap();
    let b: StructRef = runtime.invoke("vector2_new", (1.0f32, -1.0f32)).unwrap();
    let added: StructRef = runtime.invoke("vector2_add", (a, b)).unwrap();
}
