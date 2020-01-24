macro_rules! intrinsics{
    ($($(#[$attr:meta])* pub fn $name:ident($($arg_name:ident:$arg:ty),*) -> $ret:ty;);*) => {
        $(
            paste::item! {
                pub struct [<Intrinsic $name>];
            }
            paste::item! {
                impl Intrinsic for [<Intrinsic $name>] {
                    fn prototype(&self) -> FunctionPrototype {
                        FunctionPrototype {
                            name: stringify!($name).to_owned(),
                            arg_types: vec![$(<$arg as crate::type_info::HasStaticTypeInfo>::type_info()),*],
                            ret_type: <$ret as crate::type_info::HasStaticReturnTypeInfo>::return_type_info()
                        }
                    }

                    fn ir_type(&self, context: &Context) -> FunctionType {
                        let args = vec![$(<$arg as crate::ir::IsBasicIrType>::ir_type(context)),*];
                        <$ret as crate::ir::IsFunctionReturnType>::fn_type(context, &args, false)
                    }
                }
            }
            paste::item! {
                #[allow(non_upper_case_globals)]
                $(#[$attr])* pub const $name:[<Intrinsic $name>] = [<Intrinsic $name>];
            }
        )*
    };
    ($(#[$attr:meta])*) => {}
}
