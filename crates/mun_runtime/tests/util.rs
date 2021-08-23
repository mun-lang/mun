#![allow(unused_macros)]

macro_rules! assert_invoke_eq {
    ($ExpectedType:ty, $ExpectedResult:expr, $Driver:expr, $Name:expr, $($Arg:expr),*) => {
        {
            let runtime = $Driver.runtime();
            let runtime_ref = runtime.borrow();
            let result: $ExpectedType = runtime_ref.invoke($Name, ( $($Arg,)*) ).unwrap();
            assert_eq!(
                result, $ExpectedResult, "{} == {:?}",
                stringify!(mun_runtime::invoke_fn!(runtime_ref, $($Arg)*).unwrap()),
                $ExpectedResult
            );
        }
    };
    ($ExpectedType:ty, $ExpectedResult:expr, $Driver:expr, $Name:expr) => {
        assert_invoke_eq!($ExpectedType, $ExpectedResult, $Driver, $Name, )
    }
}
