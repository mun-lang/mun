use super::{
    AsValue, AsValueInto, ConcreteValueType, IrTypeContext, IrValueContext, SizedValueType, Value,
    ValueType,
};

macro_rules! tuple_impls {
    ( $( $name:ident )* ) => {
        /// Every tuple that contains values that can be converted to BasicValueEnum can be
        /// represented by a tuple
        impl<$($name: AsValueInto<inkwell::values::BasicValueEnum>),*> ConcreteValueType for ($($name,)*) {
            type Value = inkwell::values::StructValue;
        }

        /// Every tuple that contains values that can be converted to BasicValueEnum and which are
        /// sized, are also sized.
        impl<$($name: AsValueInto<inkwell::values::BasicValueEnum> + SizedValueType),*> SizedValueType for ($($name,)*)
        where
            $(
                <<$name as ConcreteValueType>::Value as ValueType>::Type: Into<inkwell::types::BasicTypeEnum>
            ,)*
        {
            fn get_ir_type(context: &IrTypeContext) -> inkwell::types::StructType {
                context.context.struct_type(&[
                    $($name::get_ir_type(context).into(),)*
                ], false)
            }
        }

        impl<$($name: AsValueInto<inkwell::values::BasicValueEnum>),*> AsValue<($($name,)*)> for ($($name,)*) {
            #[allow(unused_variables)]
            fn as_value(&self, context: &IrValueContext) -> Value<Self> {
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
