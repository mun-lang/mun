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
            pub struct $ErrName<'r, 's, $($T: ArgumentReflection,)* Output:ReturnTypeReflection> {
                msg: String,
                runtime: &'r mut Runtime,
                function_name: &'s str,
                $($Arg: $T,)*
                output: core::marker::PhantomData<Output>,
            }

            impl<'r, 's, $($T: ArgumentReflection,)* Output: ReturnTypeReflection> core::fmt::Debug for $ErrName<'r, 's, $($T,)* Output> {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    write!(f, "{}", &self.msg)
                }
            }

            impl<'r, 's, $($T: ArgumentReflection,)* Output: ReturnTypeReflection> core::fmt::Display for $ErrName<'r, 's, $($T,)* Output> {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    write!(f, "{}", &self.msg)
                }
            }

            impl<'r, 's, $($T: ArgumentReflection,)* Output: ReturnTypeReflection> std::error::Error for $ErrName<'r, 's, $($T,)* Output> {
                fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
                    None
                }
            }

            impl<'r, 's, $($T: ArgumentReflection,)* Output: ReturnTypeReflection> $ErrName<'r, 's, $($T,)* Output> {
                /// Constructs a new invocation error.
                #[allow(clippy::too_many_arguments)]
                pub fn new(err_msg: String, runtime: &'r mut Runtime, function_name: &'s str, $($Arg: $T),*) -> Self {
                    Self {
                        msg: err_msg,
                        runtime,
                        function_name,
                        $($Arg,)*
                        output: core::marker::PhantomData,
                    }
                }
            }

            impl<'r, 's, $($T: ArgumentReflection,)* Output: ReturnTypeReflection> $crate::RetryResultExt for core::result::Result<Output, $ErrName<'r, 's, $($T,)* Output>> {
                type Output = Output;

                fn retry(self) -> Self {
                    match self {
                        Ok(output) => Ok(output),
                        Err(err) => {
                            eprintln!("{}", err.msg);
                            while !err.runtime.update() {
                                // Wait until there has been an update that might fix the error
                            }
                            $crate::Runtime::$FnName(err.runtime, err.function_name, $(err.$Arg,)*)
                        }
                    }
                }

                fn wait(mut self) -> Self::Output {
                    loop {
                        if let Ok(output) = self {
                            return output;
                        }
                        self = self.retry();
                    }
                }
            }

            impl Runtime {
                /// Invokes the method `method_name` with arguments `args`, in the library compiled
                /// based on the manifest at `manifest_path`.
                ///
                /// If an error occurs when invoking the method, an error message is logged. The
                /// runtime continues looping until the cause of the error has been resolved.
                #[allow(clippy::too_many_arguments, unused_assignments)]
                pub fn $FnName<'r, 's, $($T: ArgumentReflection,)* Output: ReturnTypeReflection>(
                    runtime: &'r mut Runtime,
                    function_name: &'s str,
                    $($Arg: $T,)*
                ) -> core::result::Result<Output, $ErrName<'r, 's, $($T,)* Output>> {
                    match runtime
                        .get_function_info(function_name)
                        .ok_or(format!("Failed to obtain function '{}'", function_name))
                        .and_then(|function_info| {
                            // Validate function signature
                            let num_args = $crate::count_args!($($T),*);

                            let arg_types = function_info.signature.arg_types();
                            if arg_types.len() != num_args {
                                return Err(format!(
                                    "Invalid number of arguments. Expected: {}. Found: {}.",
                                    num_args,
                                    arg_types.len(),
                                ));
                            }

                            #[allow(unused_mut, unused_variables)]
                            let mut idx = 0;
                            $(
                                if arg_types[idx].guid != $Arg.type_guid() {
                                    return Err(format!(
                                        "Invalid argument type at index {}. Expected: {}. Found: {}.",
                                        idx,
                                        $Arg.type_name(),
                                        arg_types[idx].name(),
                                    ));
                                }
                                idx += 1;
                            )*

                            if let Some(return_type) = function_info.signature.return_type() {
                                match return_type.group {
                                    abi::TypeGroup::FundamentalTypes => {
                                        if return_type.guid != Output::type_guid() {
                                            return Err(format!(
                                                "Invalid return type. Expected: {}. Found: {}",
                                                Output::type_name(),
                                                return_type.name(),
                                            ));
                                        }
                                    }
                                    abi::TypeGroup::StructTypes => {
                                        if <Struct as ReturnTypeReflection>::type_guid() != Output::type_guid() {
                                            return Err(format!(
                                                "Invalid return type. Expected: {}. Found: Struct",
                                                Output::type_name(),
                                            ));
                                        }
                                    }
                                }

                            } else if <() as ReturnTypeReflection>::type_guid() != Output::type_guid() {
                                return Err(format!(
                                    "Invalid return type. Expected: {}. Found: {}",
                                    Output::type_name(),
                                    <() as ReturnTypeReflection>::type_name(),
                                ));
                            }

                            Ok(function_info)
                        }) {
                        Ok(function_info) => {
                            let function: fn($($T::Marshalled),*) -> Output::Marshalled = unsafe {
                                core::mem::transmute(function_info.fn_ptr)
                            };
                            let result = function($($Arg.marshal()),*);

                            // Marshall the result
                            Ok(result.marshal_into(function_info.signature.return_type()))
                        }
                        Err(e) => Err($ErrName::new(e, runtime, function_name, $($Arg),*))
                    }
                }
            }
        )+
    }
}

/// Invokes a runtime function and returns a [`Result`] that implements the [`RetryResultExt`]
/// trait.
///
/// The first argument `invoke_fn` receives is a `Runtime` and the second argument is a function
/// string. This must be a `&str`.
///
/// Additional parameters passed to `invoke_fn` are the arguments of the function in the order
/// given.
#[macro_export]
macro_rules! invoke_fn {
    ($Runtime:expr, $FnName:expr) => {
        $crate::Runtime::invoke_fn0(&mut $Runtime, $FnName)
    };
    ($Runtime:expr, $FnName:expr, $A:expr) => {
        $crate::Runtime::invoke_fn1(&mut $Runtime, $FnName, $A)
    };
    ($Runtime:expr, $FnName:expr, $A:expr, $B:expr) => {
        $crate::Runtime::invoke_fn2(&mut $Runtime, $FnName, $A, $B)
    };
    ($Runtime:expr, $FnName:expr, $A:expr, $B:expr, $C:expr) => {
        $crate::Runtime::invoke_fn3(&mut $Runtime, $FnName, $A, $B, $C)
    };
    ($Runtime:expr, $FnName:expr, $A:expr, $B:expr, $C:expr, $D:expr) => {
        $crate::Runtime::invoke_fn4(&mut $Runtime, $FnName, $A, $B, $C, $D)
    };
    ($Runtime:expr, $FnName:expr, $A:expr, $B:expr, $C:expr, $D:expr, $E:expr) => {
        $crate::Runtime::invoke_fn5(&mut $Runtime, $FnName, $A, $B, $C, $D, $E)
    };
    ($Runtime:expr, $FnName:expr, $A:expr, $B:expr, $C:expr, $D:expr, $E:expr, $F:expr) => {
        $crate::Runtime::invoke_fn6(&mut $Runtime, $FnName, $A, $B, $C, $D, $E, $F)
    };
    ($Runtime:expr, $FnName:expr, $A:expr, $B:expr, $C:expr, $D:expr, $E:expr, $F:expr, $G:expr) => {
        $crate::Runtime::invoke_fn7(&mut $Runtime, $FnName, $A, $B, $C, $D, $E, $F, $G)
    };
    ($Runtime:expr, $FnName:expr, $A:expr, $B:expr, $C:expr, $D:expr, $E:expr, $F:expr, $G:expr, $H:expr) => {
        $crate::Runtime::invoke_fn8(&mut $Runtime, $FnName, $A, $B, $C, $D, $E, $F, $G, $H)
    };
    ($Runtime:expr, $FnName:expr, $A:expr, $B:expr, $C:expr, $D:expr, $E:expr, $F:expr, $G:expr, $H:expr, $I:expr) => {
        $crate::Runtime::invoke_fn9(&mut $Runtime, $FnName, $A, $B, $C, $D, $E, $F, $G, $H, $I)
    };
    ($Runtime:expr, $FnName:expr, $A:expr, $B:expr, $C:expr, $D:expr, $E:expr, $F:expr, $G:expr, $H:expr, $I:expr, $J:expr) => {
        $crate::Runtime::invoke_fn10(
            &mut $Runtime,
            $FnName,
            $A,
            $B,
            $C,
            $D,
            $E,
            $F,
            $G,
            $H,
            $I,
            $J,
        )
    };
    ($Runtime:expr, $FnName:expr, $A:expr, $B:expr, $C:expr, $D:expr, $E:expr, $F:expr, $G:expr, $H:expr, $I:expr, $J:expr, $K:expr) => {
        $crate::Runtime::invoke_fn11(
            $Runtime, $FnName, $A, $B, $C, $D, $E, $F, $G, $H, $I, $J, $K,
        )
    };
    ($Runtime:expr, $FnName:expr, $A:expr, $B:expr, $C:expr, $D:expr, $E:expr, $F:expr, $G:expr, $H:expr, $I:expr, $J:expr, $K:expr, $L:expr) => {
        $crate::Runtime::invoke_fn12(
            $Runtime, $FnName, $A, $B, $C, $D, $E, $F, $G, $H, $I, $J, $K, $L,
        )
    };
}
