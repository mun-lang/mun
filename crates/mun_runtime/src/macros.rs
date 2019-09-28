macro_rules! invoke_fn_impl {
    ($(
        fn $FnName:ident($($Arg:tt: $T:ident),*);
    )+) => {
        $(
            impl MunRuntime {
                /// Invokes the method `method_name` with arguments `args`, in the library compiled based on
                /// the manifest at `manifest_path`.
                ///
                /// If an error occurs when invoking the method, an error message is logged. The runtime
                /// continues looping until the cause of the error has been resolved.
                pub fn $FnName<$($T: Reflection,)* Output: Reflection>(
                    &mut self,
                    function_name: &str,
                    $($Arg: $T,)*
                ) -> Output {
                    // Initialize `updated` to `true` to guarantee the method is run at least once
                    let mut updated = true;
                    loop {
                        if updated {
                            let function: core::result::Result<fn($($T),*) -> Output, String> = self
                                .get_function_info(function_name)
                                .ok_or(format!("Failed to obtain function '{}'", function_name))
                                .and_then(|function| mun_abi::downcast_fn!(function, fn($($T),*) -> Output));

                            match function {
                                Ok(function) => return function($($Arg),*),
                                Err(ref e) => {
                                    eprintln!("{}", e);
                                    updated = false;
                                }
                            }
                        } else {
                            updated = self.update();
                        }
                    }
                }
            }
        )+
    }
}

#[macro_export]
macro_rules! invoke_fn {
    ($Runtime:expr, $FnName:expr) => {
        $Runtime.invoke_fn0($FnName)
    };
    ($Runtime:expr, $FnName:expr, $A:expr) => {
        $Runtime.invoke_fn1($FnName, $A)
    };
    ($Runtime:expr, $FnName:expr, $A:expr, $B:expr) => {
        $Runtime.invoke_fn2($FnName, $A, $B)
    };
    ($Runtime:expr, $FnName:expr, $A:expr, $B:expr, $C:expr) => {
        $Runtime.invoke_fn3($FnName, $A, $B, $C)
    };
    ($Runtime:expr, $FnName:expr, $A:expr, $B:expr, $C:expr, $D:expr) => {
        $Runtime.invoke_fn4($FnName, $A, $B, $C, $D)
    };
    ($Runtime:expr, $FnName:expr, $A:expr, $B:expr, $C:expr, $D:expr, $E:expr) => {
        $Runtime.invoke_fn5($FnName, $A, $B, $C, $D, $E)
    };
    ($Runtime:expr, $FnName:expr, $A:expr, $B:expr, $C:expr, $D:expr, $E:expr, $F:expr) => {
        $Runtime.invoke_fn6($FnName, $A, $B, $C, $D, $E, $F)
    };
    ($Runtime:expr, $FnName:expr, $A:expr, $B:expr, $C:expr, $D:expr, $E:expr, $F:expr, $G:expr) => {
        $Runtime.invoke_fn7($FnName, $A, $B, $C, $D, $E, $F, $G)
    };
    ($Runtime:expr, $FnName:expr, $A:expr, $B:expr, $C:expr, $D:expr, $E:expr, $F:expr, $G:expr, $H:expr) => {
        $Runtime.invoke_fn8($FnName, $A, $B, $C, $D, $E, $F, $G, $H)
    };
    ($Runtime:expr, $FnName:expr, $A:expr, $B:expr, $C:expr, $D:expr, $E:expr, $F:expr, $G:expr, $H:expr, $I:expr) => {
        $Runtime.invoke_fn9($FnName, $A, $B, $C, $D, $E, $F, $G, $H, $I)
    };
    ($Runtime:expr, $FnName:expr, $A:expr, $B:expr, $C:expr, $D:expr, $E:expr, $F:expr, $G:expr, $H:expr, $I:expr, $J:expr) => {
        $Runtime.invoke_fn10($FnName, $A, $B, $C, $D, $E, $F, $G, $H, $I, $J)
    };
    ($Runtime:expr, $FnName:expr, $A:expr, $B:expr, $C:expr, $D:expr, $E:expr, $F:expr, $G:expr, $H:expr, $I:expr, $J:expr, $K:expr) => {
        $Runtime.invoke_fn11($FnName, $A, $B, $C, $D, $E, $F, $G, $H, $I, $J, $K)
    };
    ($Runtime:expr, $FnName:expr, $A:expr, $B:expr, $C:expr, $D:expr, $E:expr, $F:expr, $G:expr, $H:expr, $I:expr, $J:expr, $K:expr, $L:expr) => {
        $Runtime.invoke_fn12($FnName, $A, $B, $C, $D, $E, $F, $G, $H, $I, $J, $K, $L)
    };
}
