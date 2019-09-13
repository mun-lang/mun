use std::fmt::Debug;

use crate::prelude::*;
use libloading::{Library, Symbol};

/// Reflection information about a method.
#[derive(Debug)]
pub struct MethodInfo {
    name: String,
    privacy: Privacy,
    pub args: &'static [&'static TypeInfo],
    pub returns: Option<&'static TypeInfo>,
    factory: &'static dyn MethodFactory,
}

impl MethodInfo {
    /// Constructs a new `MethodInfo`.
    pub fn new(
        name: &str,
        privacy: Privacy,
        args: &'static [&'static TypeInfo],
        returns: Option<&'static TypeInfo>,
        factory: &'static dyn MethodFactory,
    ) -> MethodInfo {
        MethodInfo {
            name: name.to_string(),
            privacy,
            args,
            returns,
            factory,
        }
    }

    /// Loads the method's symbol from the specified shared `library`.
    pub fn load<'a>(&'a self, library: &'a Library) -> libloading::Result<Box<dyn Invokable + 'a>> {
        self.factory.of(library, self)
    }
}

impl MemberInfo for MethodInfo {
    fn name(&self) -> &str {
        &self.name
    }

    fn privacy(&self) -> Privacy {
        self.privacy
    }

    fn is_private(&self) -> bool {
        self.privacy == Privacy::Private
    }

    fn is_public(&self) -> bool {
        self.privacy == Privacy::Public
    }
}

/// A common trait for invokable methods with reflectable argument types and result.
pub trait Invokable {
    fn invoke(&self, args: &[&dyn Reflectable]) -> Result<Box<dyn Reflectable>, String>;
}

/// A common trait for construction of `Invokable` methods based on `MethodInfo`.
pub trait MethodFactory: Debug + Sync + Send {
    fn of<'a>(
        &self,
        library: &'a Library,
        info: &'a MethodInfo,
    ) -> libloading::Result<Box<dyn Invokable + 'a>>;
}

macro_rules! phantom {
    ($second:tt) => {
        std::marker::PhantomData
    };
}

macro_rules! method_factories {
    ($(
        $FactoryName:ident -> $MethodName:ident {
            fn($($idx:tt: $T:ident),*) -> $Output:ident
        }
    )+) => {
        $(
            struct $MethodName<'lib, $($T: Clone + Reflection,)* Output: Reflection> {
                symbol: Symbol<'lib, fn($($T),*) -> Output>,
                info: &'lib MethodInfo,
            }

            impl<'lib, $($T: Clone + Reflection,)* Output: Reflection>
                $MethodName<'lib, $($T,)* Output>
            {
                pub fn new(symbol: Symbol<'lib, fn($($T),*) -> Output>, info: &'lib MethodInfo) -> Self {
                    Self { symbol, info }
                }
            }

            impl<'lib, $($T: Clone + Reflection,)* Output: Reflection> Invokable
                for $MethodName<'lib, $($T,)* Output>
            {
                fn invoke(&self, args: &[&dyn Reflectable]) -> Result<Box<dyn Reflectable>, String> {
                    if self.info.args.len() != args.len() {
                        return Err(format!(
                            "Invalid number of arguments. Expected: {}. Found: {}.",
                            self.info.args.len(),
                            args.len()
                        ));
                    }

                    let result: Output = (self.symbol)($(
                        args[$idx].downcast_ref::<$T>().ok_or(format!(
                            "Invalid argument type at index {}. Expected: {}. Found: {}.",
                            $idx,
                            self.info.args[$idx].name,
                            args[$idx].reflect().name
                        ))?.clone()),*
                    );

                    if let Some(return_type) = self.info.returns {
                        let result_type = result.reflect();
                        if result_type.uuid != return_type.uuid {
                            return Err(format!(
                                "Invalid return type. Expected: Some({}). Found: Some({})",
                                return_type.name, result_type.name,
                            ));
                        }
                    }

                    Ok(Box::new(result))
                }
            }

            pub struct $FactoryName<$($T: Clone + Reflection,)* Output: Reflection>(
                $(std::marker::PhantomData<$T>,)*
                std::marker::PhantomData<$Output>
            );

            impl<$($T: Clone + Reflection,)* Output: Reflection> $FactoryName<$($T,)* Output>
            {
                pub fn new() -> Self {
                    Self(
                        $(phantom!($T),)*
                        std::marker::PhantomData
                    )
                }
            }

            impl<$($T: Clone + Reflection,)* Output: Reflection> Debug
                for $FactoryName<$($T,)* Output>
            {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    let args: Vec<&'static str> = vec![$($T::type_info().name),*];
                    let args: String = args.join(", ");
                    write!(
                        f,
                        "MethodFactory::<fn({}) -> {}>",
                        &args,
                        Output::type_info().name,
                    )
                }
            }

            impl<$($T: Clone + Reflection,)* Output: Reflection> MethodFactory
                for $FactoryName<$($T,)* Output>
            {
                fn of<'a>(
                    &self,
                    library: &'a Library,
                    info: &'a MethodInfo,
                ) -> libloading::Result<Box<dyn Invokable + 'a>> {
                    let symbol = unsafe { library.get(info.name().as_ref()) }?;

                    Ok(Box::new($MethodName::<$($T,)* Output>::new(symbol, info)))
                }
            }

            unsafe impl<$($T: Clone + Reflection,)* Output: Reflection> Send
                for $FactoryName<$($T,)* Output> {}

            unsafe impl<$($T: Clone + Reflection,)* Output: Reflection> Sync
                for $FactoryName<$($T,)* Output> {}
        )+
    }
}

method_factories! {
    NoArgsMethodFactory -> NoArgsMethod {
        fn() -> Output
    }

    OneArg -> OneArgsMethod {
        fn(0: A) -> Output
    }

    TwoArgsMethodFactory -> TwoArgsMethod {
        fn(0: A, 1: B) -> Output
    }

    ThreeArgsMethodFactory -> ThreeArgsMethod {
        fn(0: A, 1: B, 2: C) -> Output
    }

    FourArgsMethodFactory -> FourArgsMethod {
        fn(0: A, 1: B, 2: C, 3: D) -> Output
    }

    FiveArgsMethodFactory -> FiveArgsMethod {
        fn(0: A, 1: B, 2: C, 3: D, 4: E) -> Output
    }

    SixArgsMethodFactory -> SixArgsMethod {
        fn(0: A, 1: B, 2: C, 3: D, 4: E, 5: F) -> Output
    }

    SevenArgsMethodFactory -> SevenArgsMethod {
        fn(0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G) -> Output
    }

    EightArgsMethodFactory -> EightArgsMethod {
        fn(0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H) -> Output
    }

    NineArgsMethodFactory -> NineArgsMethod {
        fn(0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H, 8: I) -> Output
    }

    TenArgsMethodFactory -> TenArgsMethod {
        fn(0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H, 8: I, 9: J) -> Output
    }

    ElevenArgsMethodFactory -> ElevenArgsMethod {
        fn(0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H, 8: I, 9: J, 10: K) -> Output
    }

    TwelveArgsMethodFactory -> TwelveArgsMethod {
        fn(0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H, 8: I, 9: J, 10: K, 11: L) -> Output
    }
}
