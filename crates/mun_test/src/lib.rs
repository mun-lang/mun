#[no_mangle]
pub extern "C" fn load() {}

#[no_mangle]
pub extern "C" fn unload() {}

#[no_mangle]
pub extern "C" fn init() {}

#[no_mangle]
pub extern "C" fn deinit() {}

#[no_mangle]
pub extern "C" fn add(a: f32, b: f32) -> f32 {
    a + b
}

#[no_mangle]
pub extern "C" fn subtract(a: f32, b: f32) -> f32 {
    a - b
}
