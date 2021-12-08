use std::ffi::CStr;
use std::{
    ffi::CString,
    os::raw::{c_char, c_int},
};

#[repr(C)]
struct LldInvokeResult {
    success: bool,
    messages: *const c_char,
}

#[repr(C)]
pub enum LldFlavor {
    Elf = 0,
    Wasm = 1,
    Darwin = 2,
    DarwinOld = 3,
    Coff = 4,
}

extern "C" {
    fn mun_lld_link(flavor: LldFlavor, argc: c_int, argv: *const *const c_char) -> LldInvokeResult;
    fn mun_link_free_result(result: *mut LldInvokeResult);
}

pub enum LldError {
    StringConversionError,
}

pub struct LldResult {
    success: bool,
    messages: String,
}

impl LldResult {
    pub fn ok(self) -> Result<(), String> {
        if self.success {
            Ok(())
        } else {
            Err(self.messages)
        }
    }
}

/// Invokes LLD of the given flavor with the specified arguments.
pub fn link(target: LldFlavor, args: &[String]) -> LldResult {
    // Prepare arguments
    let c_args = args
        .iter()
        .map(|arg| CString::new(arg.as_bytes()).unwrap())
        .collect::<Vec<CString>>();
    let args: Vec<*const c_char> = c_args.iter().map(|arg| arg.as_ptr()).collect();

    // Invoke LLD
    let mut lld_result = unsafe { mun_lld_link(target, args.len() as c_int, args.as_ptr()) };

    // Get the messages from the invocation
    let messages = if !lld_result.messages.is_null() {
        unsafe {
            CStr::from_ptr(lld_result.messages)
                .to_string_lossy()
                .to_string()
        }
    } else {
        String::new()
    };

    // Construct the result
    let result = LldResult {
        success: lld_result.success,
        messages,
    };

    // Release the result
    unsafe { mun_link_free_result(&mut lld_result as *mut LldInvokeResult) };
    drop(lld_result);

    result
}
