#[doc(hidden)]
#[macro_export(local_inner_macros)]
macro_rules! count_args {
    () => { 0 };
    ($name:ident) => { 1 };
    ($first:ident, $($rest:ident),*) => {
        1 + count_args!($($rest),*)
    }
}

/// Tries to downcast the `fn_ptr` of `FunctionInfo` to the specified function type.
///
/// Returns an error message upon failure.
#[macro_export]
macro_rules! downcast_fn {
    ($FunctionInfo:expr, fn($($T:ident),*) -> $Output:ident) => {{
        let num_args = $crate::count_args!($($T),*);

        let arg_types = $FunctionInfo.signature.arg_types();
        if arg_types.len() != num_args {
            return Err(format!(
                "Invalid number of arguments. Expected: {}. Found: {}.",
                arg_types.len(),
                num_args
            ));
        }

        let mut idx = 0;
        $(
            if arg_types[idx].guid != $T::type_guid() {
                return Err(format!(
                    "Invalid argument type at index {}. Expected: {}. Found: {}.",
                    idx,
                    arg_types[idx].name(),
                    $T::type_name()
                ));
            }
            idx += 1;
        )*

        if let Some(return_type) = $FunctionInfo.signature.return_type() {
            if return_type.guid != Output::type_guid() {
                return Err(format!(
                    "Invalid return type. Expected: {}. Found: {}",
                    return_type.name(),
                    Output::type_name(),
                ));
            }
        } else if <()>::type_guid() != Output::type_guid() {
            return Err(format!(
                "Invalid return type. Expected: {}. Found: {}",
                <()>::type_name(),
                Output::type_name(),
            ));
        }

        Ok(unsafe { core::mem::transmute($FunctionInfo.fn_ptr) })
    }}
}
