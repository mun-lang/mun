use super::{ConcreteValueType, IrTypeContext, PointerValueType, SizedValueType, ValueType};
use inkwell::types::BasicType;

macro_rules! into_function_info_impl {
    ($(
        fn($($T:ident),*) -> $R:ident;
    )+) => {
        $(
            impl<$R:ConcreteValueType, $($T:ConcreteValueType,)*> ConcreteValueType for fn($($T,)*) -> $R {
                type Value = inkwell::values::FunctionValue;
            }
            impl<$R:SizedValueType + 'static, $($T:SizedValueType,)*> SizedValueType for fn($($T,)*) -> $R
            where
                <<R as ConcreteValueType>::Value as ValueType>::Type: inkwell::types::BasicType,
                $(
                    <$T::Value as ValueType>::Type: inkwell::types::BasicType
                ),*
            {
                fn get_ir_type(context: &IrTypeContext) -> inkwell::types::FunctionType {
                    // This is a bit of a dirty hack. The problem is that in this specific case we
                    // want the type () to be represented as void in LLVM.
                    if std::any::TypeId::of::<$R>() == std::any::TypeId::of::<()>() {
                        context.context.void_type().fn_type(
                            &[
                                $(
                                    $T::get_ir_type(context).as_basic_type_enum()
                                ),*
                            ],
                            false
                        )
                    } else {
                        $R::get_ir_type(context).fn_type(
                            &[
                                $(
                                    $T::get_ir_type(context).as_basic_type_enum()
                                ),*
                            ],
                            false
                        )
                    }
                }
            }

            impl<$R:SizedValueType + 'static, $($T:SizedValueType,)*> PointerValueType for fn($($T,)*) -> $R
            where
                <<R as ConcreteValueType>::Value as ValueType>::Type: inkwell::types::BasicType,
                $(
                    <$T::Value as ValueType>::Type: inkwell::types::BasicType
                ),*
            {
                fn get_ptr_type(
                    context: &IrTypeContext,
                    address_space: Option<inkwell::AddressSpace>,
                ) -> inkwell::types::PointerType
                {
                    debug_assert!(
                        address_space.is_none() || address_space == Some(inkwell::AddressSpace::Generic),
                        "Functions can only live in generic address space"
                    );
                    Self::get_ir_type(context).ptr_type(inkwell::AddressSpace::Generic)
                }
            }
        )+
    }
}

into_function_info_impl! {
    fn() -> R;
    fn(A) -> R;
    fn(A, B) -> R;
    fn(A, B, C) -> R;
    fn(A, B, C, D) -> R;
    fn(A, B, C, D, E) -> R;
    fn(A, B, C, D, E, F) -> R;
    fn(A, B, C, D, E, F, G) -> R;
    fn(A, B, C, D, E, F, G, H) -> R;
    fn(A, B, C, D, E, F, G, H, I) -> R;
    fn(A, B, C, D, E, F, G, H, I, J) -> R;
}
