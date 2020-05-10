// use super::{IrTypeContext};
// use crate::ir_value::{IrValueContext};
// use inkwell::types::BasicType;
// use std::marker::PhantomData;
//
// macro_rules! into_function_info_impl {
//     ($(
//         fn($($T:ident),*) -> $R:ident;
//     )+) => {
//         $(
//             impl<$R: HasAssociatedInkwellType, $($T: HasAssociatedInkwellType,)*> HasAssociatedInkwellType
//             for FunctionValue<fn($($T),*) -> $R>
//             where
//                 $R::Type: inkwell::types::BasicType,
//                 $(
//                     $T::Type: inkwell::types::BasicType
//                 ,)*
//             {
//                 type Type = inkwell::types::FunctionType;
//
//                 fn ir_type(context: &IrTypeContext) -> Self::Type {
//                     (&$R::ir_type(context) as &dyn inkwell::types::BasicType).fn_type(&[
//                         $(
//                            $T::ir_type(context).as_basic_type_enum()
//                         ,)*
//                     ], false)
//                 }
//             }
//
//             impl<$($T: HasAssociatedInkwellType,)*> HasAssociatedInkwellType
//             for FunctionValue<fn($($T),*)>
//             where
//                 $(
//                     $T::Type: inkwell::types::BasicType
//                 ,)*
//             {
//                 type Type = inkwell::types::FunctionType;
//
//                 fn ir_type(context: &IrTypeContext) -> Self::Type {
//                     context.context.void_type().fn_type(&[
//                         $(
//                            $T::ir_type(context).as_basic_type_enum()
//                         ,)*
//                     ], false)
//                 }
//             }
//         )+
//     }
// }
//
// into_function_info_impl! {
//     fn() -> R;
//     fn(A) -> R;
//     fn(A, B) -> R;
//     fn(A, B, C) -> R;
//     fn(A, B, C, D) -> R;
//     fn(A, B, C, D, E) -> R;
//     fn(A, B, C, D, E, F) -> R;
//     fn(A, B, C, D, E, F, G) -> R;
//     fn(A, B, C, D, E, F, G, H) -> R;
//     fn(A, B, C, D, E, F, G, H, I) -> R;
//     fn(A, B, C, D, E, F, G, H, I, J) -> R;
// }
//
// impl<T> AsIrValue for FunctionValue<T> {
//     type Type = FunctionValue<T>;
//
//     fn as_ir_value(&self, _context: &IrValueContext) -> Self::Type {
//         *self
//     }
// }
//
// impl<T> HasInkwellValue for FunctionValue<T> {
//     type Value = inkwell::values::FunctionValue;
//
//     fn value(&self) -> Self::Value {
//         self.value
//     }
// }
