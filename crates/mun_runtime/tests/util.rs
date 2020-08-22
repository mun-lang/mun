#![allow(unused_macros)]

macro_rules! assert_invoke_eq {
    ($ExpectedType:ty, $ExpectedResult:expr, $Driver:expr, $($Arg:tt)+) => {
        {
            let runtime = $Driver.runtime();
            let runtime_ref = runtime.borrow();
            let result: $ExpectedType = mun_runtime::invoke_fn!(runtime_ref, $($Arg)*).unwrap();
            assert_eq!(
                result, $ExpectedResult, "{} == {:?}",
                stringify!(mun_runtime::invoke_fn!(runtime_ref, $($Arg)*).unwrap()),
                $ExpectedResult
            );
        }
    }
}
