use super::{
    AsValue, AsValueInto, ConcreteValueType, IrTypeContext, IrValueContext, SizedValueType, Value,
    ValueType,
};

macro_rules! tuple_impls {
    ( $( $name:ident )* ) => {
        /// Every tuple that contains values that can be converted to [`BasicValueEnum`] can be
        /// represented by a tuple
        impl<'ink, $($name: AsValueInto<'ink, inkwell::values::BasicValueEnum<'ink>>),*> ConcreteValueType<'ink> for ($($name,)*) {
            type Value = inkwell::values::StructValue<'ink>;
        }

        /// Every tuple that contains values that can be converted to [`BasicValueEnum`] and which are
        /// sized, are also sized.
        impl<'ink, $($name: AsValueInto<'ink, inkwell::values::BasicValueEnum<'ink>> + SizedValueType<'ink>),*> SizedValueType<'ink> for ($($name,)*)
        where
            $(
                <<$name as ConcreteValueType<'ink>>::Value as ValueType<'ink>>::Type: Into<inkwell::types::BasicTypeEnum<'ink>>
            ,)*
        {
            fn get_ir_type(context: &IrTypeContext<'ink, '_>) -> inkwell::types::StructType<'ink> {
                context.context.struct_type(&[
                    $($name::get_ir_type(context).into(),)*
                ], false)
            }
        }

        impl<'ink, $($name: AsValueInto<'ink, inkwell::values::BasicValueEnum<'ink>>),*> AsValue<'ink, ($($name,)*)> for ($($name,)*) {
            #[allow(unused_variables)]
            fn as_value(&self, context: &IrValueContext<'ink, '_, '_>) -> Value<'ink, Self> {
                #[allow(non_snake_case)]
                let ($($name,)*) = self;
                Value::from_raw(
                    context.context.const_struct(&[
                        $(
                            $name.as_value_into(context)
                        ,)*],
                        false)
                )
            }
        }
    };
}

tuple_impls! {}
tuple_impls! { A }
tuple_impls! { A B }
tuple_impls! { A B C }
tuple_impls! { A B C D }
tuple_impls! { A B C D E }
tuple_impls! { A B C D E F }
tuple_impls! { A B C D E F G }
tuple_impls! { A B C D E F G H }
tuple_impls! { A B C D E F G H I }
tuple_impls! { A B C D E F G H I J }
tuple_impls! { A B C D E F G H I J K }
tuple_impls! { A B C D E F G H I J K L }
