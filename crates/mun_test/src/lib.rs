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

extern crate mun_runtime;

#[cfg(test)]
mod tests {
    use mun_runtime::{MunRuntime, Symbol};
    use std::path::Path;
    use std::time::Duration;

    #[test]
    fn mun_fn_call() {
        let mut runtime =
            MunRuntime::new(Duration::from_secs(1)).expect("Failed to initialize Mun runtime.");

        // FIXME: Remove the necessity for this relative path
        let input_path = Path::new("../../crates/mun_test/src/");
        let output_path = Path::new("../../target/debug/test.dll");

        let lib = runtime
            .add_module(&input_path, &output_path, false)
            .expect("Failed to load shared library.");

        let add_fn: Symbol<unsafe extern "C" fn(f32, f32) -> f32> = runtime
            .get_symbol(&input_path, "add")
            .expect("Could not find 'add' function symbol.");

        assert_eq!(unsafe { add_fn(2.0, 2.0) }, 4.0);
    }
}
