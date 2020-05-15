# extern crate mun_runtime;
# use mun_runtime::{invoke_fn, RuntimeBuilder, StructRef};
# use std::{cell::RefCell, env, rc::Rc};
#
# fn main() {
#     let lib_path = env::args().nth(1).expect("Expected path to a Mun library.");
#
#     let mut runtime = 
#         RuntimeBuilder::new(lib_path)
#             .spawn()
#             .expect("Failed to spawn Runtime");
#
    let mut xy: StructRef = invoke_fn!(runtime, "vector2_new", -1.0f64, 1.0f64).unwrap();
    let x: f64 = xy.get("x").unwrap();
    xy.set("x", x * x).unwrap();
    let y = xy.replace("y", -1.0f64).unwrap();
# }
