# extern crate mun_runtime;
use mun_runtime::{RuntimeBuilder, StructRef};
use std::{cell::RefCell, env, rc::Rc};

fn main() {
    let lib_path = env::args().nth(1).expect("Expected path to a Mun library.");

    let runtime = RuntimeBuilder::new(lib_path)
        .spawn()
        .expect("Failed to spawn Runtime");

    let runtime_ref = runtime.borrow();
    let a: StructRef = runtime_ref.invoke("vector2_new", (-1.0f32, 1.0f32)).unwrap();
    let b: StructRef = runtime_ref.invoke("vector2_new", (1.0f32, -1.0f32)).unwrap();
    let added: StructRef = runtime_ref.invoke("vector2_add", (a, b)).unwrap();
}
