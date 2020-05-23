use crate::type_info::TypeInfo;
use inkwell::context::Context;
use inkwell::targets::TargetData;
use inkwell::types::{
    AnyType, AnyTypeEnum, BasicType, BasicTypeEnum, FloatType, FunctionType, IntType, PointerType,
};
use inkwell::AddressSpace;

pub(crate) mod abi_types;
pub mod adt;
pub mod body;
#[macro_use]
pub(crate) mod dispatch_table;
pub mod file;
pub(crate) mod file_group;
pub mod function;
mod intrinsics;
pub mod ir_types;
pub mod ty;
pub(crate) mod type_table;

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

    fn ir_type(context: &Context, target: &TargetData) -> Self::Type;
}

/// Defines that a type has a static represention in inkwell that can be described as a BasicType.
pub trait IsBasicIrType {
    fn ir_type(context: &Context, target: &TargetData) -> BasicTypeEnum;
}

impl<S: BasicType, T: IsIrType<Type = S>> IsBasicIrType for T {
    fn ir_type(context: &Context, target: &TargetData) -> BasicTypeEnum {
        Self::ir_type(context, target).as_basic_type_enum()
    }
}

/// Defines that a type can statically be used as a return type for a function
pub trait IsFunctionReturnType {
    fn fn_type(
        context: &Context,
        target: &TargetData,
        arg_types: &[BasicTypeEnum],
        is_var_args: bool,
    ) -> FunctionType;
}

/// All types that statically have a BasicTypeEnum can also be used as a function return type
impl<T: IsBasicIrType> IsFunctionReturnType for T {
    fn fn_type(
        context: &Context,
        target: &TargetData,
        arg_types: &[BasicTypeEnum],
        is_var_args: bool,
    ) -> FunctionType {
        T::ir_type(context, target).fn_type(arg_types, is_var_args)
    }
}

impl IsFunctionReturnType for () {
    fn fn_type(
        context: &Context,
        _target: &TargetData,
        arg_types: &[BasicTypeEnum],
        is_var_args: bool,
    ) -> FunctionType {
        context.void_type().fn_type(arg_types, is_var_args)
    }
}

/// Defines that a value can be converted to an inkwell type
pub trait AsIrType {
    type Type: AnyType;

    fn as_ir_type(&self, context: &Context, target: &TargetData) -> Self::Type;
}

pub trait AsBasicIrType {
    fn as_ir_type(&self, context: &Context, target: &TargetData) -> BasicTypeEnum;
}

impl<S: BasicType, T: AsIrType<Type = S>> AsBasicIrType for T {
    fn as_ir_type(&self, context: &Context, target: &TargetData) -> BasicTypeEnum {
        self.as_ir_type(context, target).as_basic_type_enum()
    }
}

/// Defines that a value can be used to construct a function type.
pub trait AsFunctionReturnType {
    fn as_fn_type(
        &self,
        context: &Context,
        target: &TargetData,
        arg_types: &[BasicTypeEnum],
        is_var_args: bool,
    ) -> FunctionType;
}

impl<T: AsBasicIrType> AsFunctionReturnType for T {
    fn as_fn_type(
        &self,
        context: &Context,
        target: &TargetData,
        arg_types: &[BasicTypeEnum],
        is_var_args: bool,
    ) -> FunctionType {
        self.as_ir_type(context, target)
            .fn_type(arg_types, is_var_args)
    }
}

impl AsFunctionReturnType for () {
    fn as_fn_type(
        &self,
        context: &Context,
        _target: &TargetData,
        arg_types: &[BasicTypeEnum],
        is_var_args: bool,
    ) -> FunctionType {
        context.void_type().fn_type(arg_types, is_var_args)
    }
}

macro_rules! impl_fundamental_ir_types {
    ($(
        $ty:ty => $context_fun:ident():$inkwell_ty:ty
    ),+) => {
        $(
            impl IsIrType for $ty {
                type Type = $inkwell_ty;

                fn ir_type(context: &Context, _target: &TargetData) -> Self::Type {
                    context.$context_fun()
                }
            }
        )+
    }
}

impl_fundamental_ir_types!(
    i8 => i8_type():IntType,
    i16 => i16_type():IntType,
    i32 => i32_type():IntType,
    i64 => i64_type():IntType,
    i128 => i128_type():IntType,

    u8 => i8_type():IntType,
    u16 => i16_type():IntType,
    u32 => i32_type():IntType,
    u64 => i64_type():IntType,
    u128 => i128_type():IntType,

    bool => bool_type():IntType,

    f32 => f32_type():FloatType,
    f64 => f64_type():FloatType
);

impl IsIrType for usize {
    type Type = IntType;

    fn ir_type(_context: &Context, target: &TargetData) -> Self::Type {
        target.ptr_sized_int_type(None)
    }
}

impl IsIrType for isize {
    type Type = IntType;

    fn ir_type(_context: &Context, target: &TargetData) -> Self::Type {
        target.ptr_sized_int_type(None)
    }
}

pub trait IsPointerType {
    fn ir_type(context: &Context, target: &TargetData) -> PointerType;
}

impl<S: BasicType, T: IsIrType<Type = S>> IsPointerType for *const T {
    fn ir_type(context: &Context, target: &TargetData) -> PointerType {
        T::ir_type(context, target).ptr_type(AddressSpace::Generic)
    }
}

// HACK: Manually add `*const TypeInfo`
impl IsPointerType for *const TypeInfo {
    fn ir_type(context: &Context, _target: &TargetData) -> PointerType {
        context.i8_type().ptr_type(AddressSpace::Generic)
    }
}

// HACK: Manually add `*const c_void`
impl IsPointerType for *const std::ffi::c_void {
    fn ir_type(context: &Context, _target: &TargetData) -> PointerType {
        context.i8_type().ptr_type(AddressSpace::Generic)
    }
}

// HACK: Manually add `*mut c_void`
impl IsPointerType for *mut std::ffi::c_void {
    fn ir_type(context: &Context, _target: &TargetData) -> PointerType {
        context.i8_type().ptr_type(AddressSpace::Generic)
    }
}

impl<S: BasicType, T: IsIrType<Type = S>> IsPointerType for *mut T {
    fn ir_type(context: &Context, target: &TargetData) -> PointerType {
        T::ir_type(context, target).ptr_type(AddressSpace::Generic)
    }
}

impl<T: IsPointerType> IsIrType for T {
    type Type = PointerType;

    fn ir_type(context: &Context, target: &TargetData) -> Self::Type {
        T::ir_type(context, target)
    }
}
