# extern crate mun_runtime;
use mun_runtime::{Runtime, StructRef};
use std::{cell::RefCell, env, rc::Rc};

fn main() {
    let lib_path = env::args().nth(1).expect("Expected path to a Mun library.");

    // Safety: We assume that the library that is loaded is a valid munlib
    let builder = Runtime::builder(lib_path);
    let mut runtime = unsafe { builder.finish() }
        .expect("Failed to spawn Runtime");

    let a: StructRef = runtime.invoke("vector2_new", (-1.0f32, 1.0f32)).unwrap();
    let b: StructRef = runtime.invoke("vector2_new", (1.0f32, -1.0f32)).unwrap();
    let added: StructRef = runtime.invoke("vector2_add", (a, b)).unwrap();
}
