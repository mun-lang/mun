# extern crate mun_runtime;
# use mun_runtime::{Runtime, StructRef};
# use std::{cell::RefCell, env, rc::Rc};
#
# fn main() {
#     let lib_path = env::args().nth(1).expect("Expected path to a Mun library.");
#
#     let runtime =
#         Runtime::builder(lib_path)
#             .finish()
#             .expect("Failed to spawn Runtime");
#
    let mut xy: StructRef = runtime.invoke("vector2_new", (-1.0f32, 1.0f32)).unwrap();
    let x: f32 = xy.get("x").unwrap();
    xy.set("x", x * x).unwrap();
    let y = xy.replace("y", -1.0f32).unwrap();
# }
