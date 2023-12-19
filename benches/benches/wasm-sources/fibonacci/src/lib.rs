extern crate wasm_bindgen;

use wasm_bindgen::prelude::*;

fn fibonacci(n: isize) -> isize {
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

#[wasm_bindgen]
pub fn main(n: isize) -> isize {
    fibonacci(n)
}
