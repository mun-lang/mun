#[doc(hidden)]
#[macro_export(local_inner_macros)]
macro_rules! count_args {
    () => { 0 };
    ($name:ident) => { 1 };
    ($first:ident, $($rest:ident),*) => {
        1 + count_args!($($rest),*)
    }
}

macro_rules! invoke_fn_impl {
    ($(
        fn $FnName:ident($($Arg:tt: $T:ident),*) -> $ErrName:ident;
    )+) => {
        $(
            /// An invocation error that contains the function name, a mutable reference to the
            /// runtime, passed arguments, and the output type. This allows the caller to retry
            /// the function invocation using the `Retriable` trait.
            pub struct $ErrName<'i, 's, $($T: ArgumentReflection + Marshal<'i>,)*> {
                msg: String,
                function_name: &'s str,
                $($Arg: $T,)*
                input: core::marker::PhantomData<&'i ()>,
            }

            impl<'i, 's, $($T: ArgumentReflection + Marshal<'i>,)*> core::fmt::Debug for $ErrName<'i, 's, $($T,)*> {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    write!(f, "{}", &self.msg)
                }
            }

            impl<'i, 's, $($T: ArgumentReflection + Marshal<'i>,)*> core::fmt::Display for $ErrName<'i, 's, $($T,)*> {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    write!(f, "{}", &self.msg)
                }
            }

            impl<'i, 's, $($T: ArgumentReflection + Marshal<'i>,)*> std::error::Error for $ErrName<'i, 's, $($T,)*> {
                fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
                    None
                }
            }

            impl<'i, 's, $($T: ArgumentReflection + Marshal<'i>,)*> $ErrName<'i, 's, $($T,)*> {
                /// Constructs a new invocation error.
                #[allow(clippy::too_many_arguments)]
                pub fn new(err_msg: String, function_name: &'s str, $($Arg: $T),*) -> Self {
                    Self {
                        msg: err_msg,
                        function_name,
                        $($Arg,)*
                        input: core::marker::PhantomData,
                    }
                }

                /// Retries a function invocation once, resulting in a potentially successful
                /// invocation.
                // FIXME: `unwrap_or_else` does not compile for `StructRef`, due to
                // https://doc.rust-lang.org/nomicon/lifetime-mismatch.html#improperly-reduced-borrows
                pub fn retry<'r, 'o, Output>(self, runtime: &'r mut Runtime) -> Result<Output, Self>
                where
                    Output: 'o + ReturnTypeReflection + Marshal<'o>,
                    'r: 'o,
                {
                    // Safety: The output of `retry_impl` is guaranteed to only contain a shared
                    // reference.
                    unsafe { self.retry_impl(runtime) }
                }

                /// Retries the function invocation until it succeeds, resulting in an output.
                // FIXME: `unwrap_or_else` does not compile for `StructRef`, due to
                // https://doc.rust-lang.org/nomicon/lifetime-mismatch.html#improperly-reduced-borrows
                pub fn wait<'r, 'o, Output>(mut self, runtime: &'r mut Runtime) -> Output
                where
                    Output: 'o + ReturnTypeReflection + Marshal<'o>,
                    'r: 'o,
                {
                    // Safety: The output of `retry_impl` is guaranteed to only contain a shared
                    // reference.
                    let runtime = &*runtime;

                    loop {
                        self = match unsafe { self.retry_impl(runtime) } {
                            Ok(output) => return output,
                            Err(e) => e,
                        };
                    }
                }

                /// Inner implementation that retries a function invocation once, resulting in a
                /// potentially successful invocation. This is a workaround for:
                /// https://doc.rust-lang.org/nomicon/lifetime-mismatch.html
                ///
                /// # Safety
                ///
                /// When calling this function, you have to guarantee that `runtime` is mutably
                /// borrowed. The `Output` value can only contain a shared borrow of `runtime`.
                unsafe fn retry_impl<'r, 'o, Output>(self, runtime: &'r Runtime) -> Result<Output, Self>
                where
                    Output: 'o + ReturnTypeReflection + Marshal<'o>,
                    'r: 'o,
                {
                    #[allow(clippy::cast_ref_to_mut)]
                    let runtime = &mut *(runtime as *const Runtime as *mut Runtime);

                    eprintln!("{}", self.msg);
                    while !runtime.update() {
                        // Wait until there has been an update that might fix the error
                    }
                    $crate::Runtime::$FnName(runtime, self.function_name, $(self.$Arg,)*)
                }
            }

            impl Runtime {
                /// Invokes the method `method_name` with arguments `args`, in the library compiled
                /// based on the manifest at `manifest_path`.
                ///
                /// If an error occurs when invoking the method, an error message is logged. The
                /// runtime continues looping until the cause of the error has been resolved.
                #[allow(clippy::too_many_arguments, unused_assignments)]
                pub fn $FnName<'i, 'o, 'r, 's, $($T: ArgumentReflection + Marshal<'i>,)* Output: 'o + ReturnTypeReflection + Marshal<'o>>(
                    runtime: &'r Runtime,
                    function_name: &'s str,
                    $($Arg: $T,)*
                ) -> core::result::Result<Output, $ErrName<'i, 's, $($T,)*>>
                where
                    'r: 'o,
                {
                    match runtime
                        .get_function_definition(function_name)
                        .ok_or_else(|| format!("Failed to obtain function '{}'", function_name))
                        .and_then(|function_info| {
                            // Validate function signature
                            let num_args = $crate::count_args!($($T),*);

                            let arg_types = function_info.prototype.signature.arg_types();
                            if arg_types.len() != num_args {
                                return Err(format!(
                                    "Invalid number of arguments. Expected: {}. Found: {}.",
                                    arg_types.len(),
                                    num_args,
                                ));
                            }

                            #[allow(unused_mut, unused_variables)]
                            let mut idx = 0;
                            $(
                                crate::reflection::equals_argument_type(runtime, &arg_types[idx], &$Arg)
                                    .map_err(|(expected, found)| {
                                        format!(
                                            "Invalid argument type at index {}. Expected: {}. Found: {}.",
                                            idx,
                                            expected,
                                            found,
                                        )
                                    })?;
                                idx += 1;
                            )*

                            if let Some(return_type) = function_info.prototype.signature.return_type() {
                                crate::reflection::equals_return_type::<Output>(return_type)
                            } else if <() as ReturnTypeReflection>::type_guid() != Output::type_guid() {
                                Err((<() as ReturnTypeReflection>::type_name(), Output::type_name()))
                            } else {
                                Ok(())
                            }.map_err(|(expected, found)| {
                                format!(
                                    "Invalid return type. Expected: {}. Found: {}",
                                    expected,
                                    found,
                                )
                            })?;

                            Ok(function_info)
                        }) {
                        Ok(function_info) => {
                            let function: fn($($T::MunType),*) -> Output::MunType = unsafe {
                                core::mem::transmute(function_info.fn_ptr)
                            };
                            let result = function($($Arg.marshal_into()),*);

                            // Marshall the result
                            return Ok(Marshal::marshal_from(result, runtime))
                        }
                        Err(e) => Err($ErrName::new(e, function_name, $($Arg),*))
                    }
                }
            }
        )+
    }
}

/// Invokes a runtime function and returns a [`Result`] that contains either the output value or
/// an error that can be used to retry the function invocation.
///
/// The first argument `invoke_fn` receives is a `Ref<Runtime>` and the second argument is a
/// function string. This must be a `&str`.
///
/// Additional parameters passed to `invoke_fn` are the arguments of the function in the order
/// given.
#[macro_export]
macro_rules! invoke_fn {
    ($Runtime:expr, $FnName:expr) => {
        $crate::Runtime::invoke_fn0(&$Runtime, $FnName)
    };
    ($Runtime:expr, $FnName:expr, $A:expr) => {
        $crate::Runtime::invoke_fn1(&$Runtime, $FnName, $A)
    };
    ($Runtime:expr, $FnName:expr, $A:expr, $B:expr) => {
        $crate::Runtime::invoke_fn2(&$Runtime, $FnName, $A, $B)
    };
    ($Runtime:expr, $FnName:expr, $A:expr, $B:expr, $C:expr) => {
        $crate::Runtime::invoke_fn3(&$Runtime, $FnName, $A, $B, $C)
    };
    ($Runtime:expr, $FnName:expr, $A:expr, $B:expr, $C:expr, $D:expr) => {
        $crate::Runtime::invoke_fn4(&$Runtime, $FnName, $A, $B, $C, $D)
    };
    ($Runtime:expr, $FnName:expr, $A:expr, $B:expr, $C:expr, $D:expr, $E:expr) => {
        $crate::Runtime::invoke_fn5(&$Runtime, $FnName, $A, $B, $C, $D, $E)
    };
    ($Runtime:expr, $FnName:expr, $A:expr, $B:expr, $C:expr, $D:expr, $E:expr, $F:expr) => {
        $crate::Runtime::invoke_fn6(&$Runtime, $FnName, $A, $B, $C, $D, $E, $F)
    };
    ($Runtime:expr, $FnName:expr, $A:expr, $B:expr, $C:expr, $D:expr, $E:expr, $F:expr, $G:expr) => {
        $crate::Runtime::invoke_fn7(&$Runtime, $FnName, $A, $B, $C, $D, $E, $F, $G)
    };
    ($Runtime:expr, $FnName:expr, $A:expr, $B:expr, $C:expr, $D:expr, $E:expr, $F:expr, $G:expr, $H:expr) => {
        $crate::Runtime::invoke_fn8(&$Runtime, $FnName, $A, $B, $C, $D, $E, $F, $G, $H)
    };
    ($Runtime:expr, $FnName:expr, $A:expr, $B:expr, $C:expr, $D:expr, $E:expr, $F:expr, $G:expr, $H:expr, $I:expr) => {
        $crate::Runtime::invoke_fn9(&$Runtime, $FnName, $A, $B, $C, $D, $E, $F, $G, $H, $I)
    };
    ($Runtime:expr, $FnName:expr, $A:expr, $B:expr, $C:expr, $D:expr, $E:expr, $F:expr, $G:expr, $H:expr, $I:expr, $J:expr) => {
        $crate::Runtime::invoke_fn10(&$Runtime, $FnName, $A, $B, $C, $D, $E, $F, $G, $H, $I, $J)
    };
    ($Runtime:expr, $FnName:expr, $A:expr, $B:expr, $C:expr, $D:expr, $E:expr, $F:expr, $G:expr, $H:expr, $I:expr, $J:expr, $K:expr) => {
        $crate::Runtime::invoke_fn11(
            &$Runtime, $FnName, $A, $B, $C, $D, $E, $F, $G, $H, $I, $J, $K,
        )
    };
    ($Runtime:expr, $FnName:expr, $A:expr, $B:expr, $C:expr, $D:expr, $E:expr, $F:expr, $G:expr, $H:expr, $I:expr, $J:expr, $K:expr, $L:expr) => {
        $crate::Runtime::invoke_fn12(
            &$Runtime, $FnName, $A, $B, $C, $D, $E, $F, $G, $H, $I, $J, $K, $L,
        )
    };
    ($Runtime:expr, $FnName:expr, $A:expr, $B:expr, $C:expr, $D:expr, $E:expr, $F:expr, $G:expr, $H:expr, $I:expr, $J:expr, $K:expr, $L:expr, $M:expr) => {
        $crate::Runtime::invoke_fn13(
            &$Runtime, $FnName, $A, $B, $C, $D, $E, $F, $G, $H, $I, $J, $K, $L, $M,
        )
    };
    ($Runtime:expr, $FnName:expr, $A:expr, $B:expr, $C:expr, $D:expr, $E:expr, $F:expr, $G:expr, $H:expr, $I:expr, $J:expr, $K:expr, $L:expr, $M:expr, $N:expr) => {
        $crate::Runtime::invoke_fn14(
            &$Runtime, $FnName, $A, $B, $C, $D, $E, $F, $G, $H, $I, $J, $K, $L, $M, $N,
        )
    };
    ($Runtime:expr, $FnName:expr, $A:expr, $B:expr, $C:expr, $D:expr, $E:expr, $F:expr, $G:expr, $H:expr, $I:expr, $J:expr, $K:expr, $L:expr, $M:expr, $N:expr, $O:expr) => {
        $crate::Runtime::invoke_fn15(
            &$Runtime, $FnName, $A, $B, $C, $D, $E, $F, $G, $H, $I, $J, $K, $L, $M, $N, $O,
        )
    };
}
