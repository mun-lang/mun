#[no_mangle]
pub extern "C" fn load() {}

#[no_mangle]
pub fn load() {}

#[no_mangle]
pub fn unload() {}

#[no_mangle]
pub fn init() {}

#[no_mangle]
pub fn deinit() {}

#[no_mangle]
pub fn add(a: f32, b: f32) -> f32 {
    a + b
}
