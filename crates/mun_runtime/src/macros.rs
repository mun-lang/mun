macro_rules! invoke_fn_impl {
    ($(
        fn $FnName:ident($($Arg:tt: $T:ident),*) -> $ErrName:ident;
    )+) => {
        $(
            /// An invocation error that contains the function name, a mutable reference to the
            /// runtime, passed arguments, and the output type. This allows the caller to retry
            /// the function invocation using the `Retriable` trait.
            pub struct $ErrName<'r, 's, $($T: Reflection,)* Output:Reflection> {
                msg: String,
                runtime: &'r mut Runtime,
                function_name: &'s str,
                $($Arg: $T,)*
                output: core::marker::PhantomData<Output>,
            }

            impl<'r, 's, $($T: Reflection,)* Output: Reflection> core::fmt::Debug for $ErrName<'r, 's, $($T,)* Output> {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    write!(f, "{}", &self.msg)
                }
            }

            impl<'r, 's, $($T: Reflection,)* Output: Reflection> core::fmt::Display for $ErrName<'r, 's, $($T,)* Output> {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    write!(f, "{}", &self.msg)
                }
            }

            impl<'r, 's, $($T: Reflection,)* Output: Reflection> std::error::Error for $ErrName<'r, 's, $($T,)* Output> {
                fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
                    None
                }
            }

            impl<'r, 's, $($T: Reflection,)* Output: Reflection> $ErrName<'r, 's, $($T,)* Output> {
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

            impl<'r, 's, $($T: Reflection,)* Output: Reflection> $crate::RetryResultExt for core::result::Result<Output, $ErrName<'r, 's, $($T,)* Output>> {
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
                #[allow(clippy::too_many_arguments)]
                pub fn $FnName<'r, 's, $($T: Reflection,)* Output: Reflection>(
                    runtime: &'r mut Runtime,
                    function_name: &'s str,
                    $($Arg: $T,)*
                ) -> core::result::Result<Output, $ErrName<'r, 's, $($T,)* Output>> {
                    let function: core::result::Result<fn($($T),*) -> Output, String> = runtime
                        .get_function_info(function_name)
                        .ok_or(format!("Failed to obtain function '{}'", function_name))
                        .and_then(|function| mun_abi::downcast_fn!(function, fn($($T),*) -> Output));

                    match function {
                        Ok(function) => Ok(function($($Arg),*)),
                        Err(e) => Err($ErrName::new(e, runtime, function_name, $($Arg),*)),
                    }
                }
            }
        )+
    }
}

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
