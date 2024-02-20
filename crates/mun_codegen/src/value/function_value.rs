use inkwell::types::BasicType;

use super::{ConcreteValueType, IrTypeContext, PointerValueType, SizedValueType, ValueType};

macro_rules! into_function_info_impl {
    ($(
        fn($($T:ident),*) -> $R:ident;
    )+) => {
        $(
            impl<'ink, $R:ConcreteValueType<'ink>, $($T:ConcreteValueType<'ink>,)*> ConcreteValueType<'ink> for fn($($T,)*) -> $R {
                type Value = inkwell::values::FunctionValue<'ink>;
            }
            impl<'ink, $R:SizedValueType<'ink> + 'ink, $($T:SizedValueType<'ink>,)*> SizedValueType<'ink> for fn($($T,)*) -> $R
            where
                <<R as ConcreteValueType<'ink>>::Value as ValueType<'ink>>::Type: inkwell::types::BasicType<'ink>,
                $(
                    <$T::Value as ValueType<'ink>>::Type: inkwell::types::BasicType<'ink>
                ),*
            {
                fn get_ir_type(context: &IrTypeContext<'ink, '_>) -> inkwell::types::FunctionType<'ink> {
                    // This is a bit of a dirty hack. The problem is that in this specific case we
                    // want the type () to be represented as void in LLVM.
                    if std::any::type_name::<$R>() == std::any::type_name::<()>() {
                        context.context.void_type().fn_type(
                            &[
                                $(
                                    $T::get_ir_type(context).as_basic_type_enum().into()
                                ),*
                            ],
                            false
                        )
                    } else {
                        $R::get_ir_type(context).fn_type(
                            &[
                                $(
                                    $T::get_ir_type(context).as_basic_type_enum().into()
                                ),*
                            ],
                            false
                        )
                    }
                }
            }

            impl<'ink, $R:SizedValueType<'ink> + 'ink, $($T:SizedValueType<'ink>,)*> PointerValueType<'ink> for fn($($T,)*) -> $R
            where
                <<R as ConcreteValueType<'ink>>::Value as ValueType<'ink>>::Type: inkwell::types::BasicType<'ink>,
                $(
                    <$T::Value as ValueType<'ink>>::Type: inkwell::types::BasicType<'ink>
                ),*
            {
                fn get_ptr_type(
                    context: &IrTypeContext<'ink, '_>,
                    address_space: Option<inkwell::AddressSpace>,
                ) -> inkwell::types::PointerType<'ink>
                {
                    debug_assert!(
                        address_space.is_none() || address_space == Some(inkwell::AddressSpace::default()),
                        "Functions can only live in generic address space"
                    );
                    Self::get_ir_type(context).ptr_type(inkwell::AddressSpace::default())
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
