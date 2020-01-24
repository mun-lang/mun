use inkwell::context::Context;
use inkwell::types::{
    AnyType, AnyTypeEnum, BasicType, BasicTypeEnum, FunctionType, IntType, PointerType,
};
use inkwell::AddressSpace;

pub mod adt;
pub mod body;
#[macro_use]
pub(crate) mod dispatch_table;
pub mod function;
pub mod module;
pub mod ty;

/// Try to down cast an `AnyTypeEnum` into a `BasicTypeEnum`.
fn try_convert_any_to_basic(ty: AnyTypeEnum) -> Option<BasicTypeEnum> {
    match ty {
        AnyTypeEnum::ArrayType(t) => Some(t.into()),
        AnyTypeEnum::FloatType(t) => Some(t.into()),
        AnyTypeEnum::IntType(t) => Some(t.into()),
        AnyTypeEnum::PointerType(t) => Some(t.into()),
        AnyTypeEnum::StructType(t) => Some(t.into()),
        AnyTypeEnum::VectorType(t) => Some(t.into()),
        _ => None,
    }
}

/// Defines that a type has a static representation in inkwell
pub trait IsIrType {
    type Type: AnyType;

    fn ir_type(context: &Context) -> Self::Type;
}

pub trait IsBasicIrType {
    fn ir_type(context: &Context) -> BasicTypeEnum;
}

impl<S: BasicType, T: IsIrType<Type = S>> IsBasicIrType for T {
    fn ir_type(context: &Context) -> BasicTypeEnum {
        Self::ir_type(context).as_basic_type_enum()
    }
}

/// Defines that a type can statically be used as a return type for a function
pub trait IsFunctionReturnType {
    fn fn_type(context: &Context, arg_types: &[BasicTypeEnum], is_var_args: bool) -> FunctionType;
}

/// All types that statically have a BasicTypeEnum can also be used as a function return type
impl<T: IsBasicIrType> IsFunctionReturnType for T {
    fn fn_type(context: &Context, arg_types: &[BasicTypeEnum], is_var_args: bool) -> FunctionType {
        T::ir_type(context).fn_type(arg_types, is_var_args)
    }
}

impl IsFunctionReturnType for () {
    fn fn_type(context: &Context, arg_types: &[BasicTypeEnum], is_var_args: bool) -> FunctionType {
        context.void_type().fn_type(arg_types, is_var_args)
    }
}

/// Defines that a value can be converted to an inkwell type
pub trait AsIrType {
    type Type: AnyType;

    fn as_ir_type(&self, context: &Context) -> Self::Type;
}

pub trait AsBasicIrType {
    fn as_ir_type(&self, context: &Context) -> BasicTypeEnum;
}

impl<S: BasicType, T: AsIrType<Type = S>> AsBasicIrType for T {
    fn as_ir_type(&self, context: &Context) -> BasicTypeEnum {
        self.as_ir_type(context).as_basic_type_enum()
    }
}

/// Defines that a value can be used to construct a function type.
pub trait AsFunctionReturnType {
    fn as_fn_type(
        &self,
        context: &Context,
        arg_types: &[BasicTypeEnum],
        is_var_args: bool,
    ) -> FunctionType;
}

impl<T: AsBasicIrType> AsFunctionReturnType for T {
    fn as_fn_type(
        &self,
        context: &Context,
        arg_types: &[BasicTypeEnum],
        is_var_args: bool,
    ) -> FunctionType {
        self.as_ir_type(context).fn_type(arg_types, is_var_args)
    }
}

impl AsFunctionReturnType for () {
    fn as_fn_type(
        &self,
        context: &Context,
        arg_types: &[BasicTypeEnum],
        is_var_args: bool,
    ) -> FunctionType {
        context.void_type().fn_type(arg_types, is_var_args)
    }
}

impl IsIrType for u8 {
    type Type = IntType;

    fn ir_type(context: &Context) -> Self::Type {
        context.i8_type()
    }
}

impl IsIrType for u16 {
    type Type = IntType;

    fn ir_type(context: &Context) -> Self::Type {
        context.i16_type()
    }
}

impl IsIrType for u32 {
    type Type = IntType;

    fn ir_type(context: &Context) -> Self::Type {
        context.i32_type()
    }
}

impl IsIrType for u64 {
    type Type = IntType;

    fn ir_type(context: &Context) -> Self::Type {
        context.i64_type()
    }
}

impl IsIrType for i8 {
    type Type = IntType;

    fn ir_type(context: &Context) -> Self::Type {
        context.i8_type()
    }
}

impl IsIrType for i16 {
    type Type = IntType;

    fn ir_type(context: &Context) -> Self::Type {
        context.i16_type()
    }
}

impl IsIrType for i32 {
    type Type = IntType;

    fn ir_type(context: &Context) -> Self::Type {
        context.i32_type()
    }
}

impl IsIrType for i64 {
    type Type = IntType;

    fn ir_type(context: &Context) -> Self::Type {
        context.i64_type()
    }
}

pub trait IsPointerType {
    fn ir_type(context: &Context) -> PointerType;
}

impl<S: BasicType, T: IsIrType<Type = S>> IsPointerType for *const T {
    fn ir_type(context: &Context) -> PointerType {
        T::ir_type(context).ptr_type(AddressSpace::Const)
    }
}

impl<S: BasicType, T: IsIrType<Type = S>> IsPointerType for *mut T {
    fn ir_type(context: &Context) -> PointerType {
        T::ir_type(context).ptr_type(AddressSpace::Generic)
    }
}

impl<T: IsPointerType> IsIrType for T {
    type Type = PointerType;

    fn ir_type(context: &Context) -> Self::Type {
        T::ir_type(context)
    }
}
