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
pub mod function;
mod intrinsics;
pub mod module;
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

impl IsIrType for u8 {
    type Type = IntType;

    fn ir_type(context: &Context, _target: &TargetData) -> Self::Type {
        context.i8_type()
    }
}

impl IsIrType for u16 {
    type Type = IntType;

    fn ir_type(context: &Context, _target: &TargetData) -> Self::Type {
        context.i16_type()
    }
}

impl IsIrType for u32 {
    type Type = IntType;

    fn ir_type(context: &Context, _target: &TargetData) -> Self::Type {
        context.i32_type()
    }
}

impl IsIrType for u64 {
    type Type = IntType;

    fn ir_type(context: &Context, _target: &TargetData) -> Self::Type {
        context.i64_type()
    }
}

impl IsIrType for i8 {
    type Type = IntType;

    fn ir_type(context: &Context, _target: &TargetData) -> Self::Type {
        context.i8_type()
    }
}

impl IsIrType for i16 {
    type Type = IntType;

    fn ir_type(context: &Context, _target: &TargetData) -> Self::Type {
        context.i16_type()
    }
}

impl IsIrType for i32 {
    type Type = IntType;

    fn ir_type(context: &Context, _target: &TargetData) -> Self::Type {
        context.i32_type()
    }
}

impl IsIrType for i64 {
    type Type = IntType;

    fn ir_type(context: &Context, _target: &TargetData) -> Self::Type {
        context.i64_type()
    }
}

impl IsIrType for usize {
    type Type = IntType;

    fn ir_type(context: &Context, target: &TargetData) -> Self::Type {
        match target.get_pointer_byte_size(None) {
            4 => <u32 as IsIrType>::ir_type(context, target),
            8 => <u64 as IsIrType>::ir_type(context, target),
            _ => unimplemented!("unsupported pointer byte size"),
        }
    }
}

impl IsIrType for isize {
    type Type = IntType;

    fn ir_type(context: &Context, target: &TargetData) -> Self::Type {
        match target.get_pointer_byte_size(None) {
            4 => <i32 as IsIrType>::ir_type(context, target),
            8 => <i64 as IsIrType>::ir_type(context, target),
            _ => unimplemented!("unsupported pointer byte size"),
        }
    }
}

impl IsIrType for f32 {
    type Type = FloatType;

    fn ir_type(context: &Context, _target: &TargetData) -> Self::Type {
        context.f32_type()
    }
}

impl IsIrType for f64 {
    type Type = FloatType;

    fn ir_type(context: &Context, _target: &TargetData) -> Self::Type {
        context.f64_type()
    }
}

impl IsIrType for bool {
    type Type = IntType;

    fn ir_type(context: &Context, _target: &TargetData) -> Self::Type {
        context.bool_type()
    }
}

pub trait IsPointerType {
    fn ir_type(context: &Context, target: &TargetData) -> PointerType;
}

impl<S: BasicType, T: IsIrType<Type = S>> IsPointerType for *const T {
    fn ir_type(context: &Context, target: &TargetData) -> PointerType {
        T::ir_type(context, target).ptr_type(AddressSpace::Const)
    }
}

// HACK: Manually add `*const TypeInfo`
impl IsPointerType for *const TypeInfo {
    fn ir_type(context: &Context, _target: &TargetData) -> PointerType {
        context.i8_type().ptr_type(AddressSpace::Const)
    }
}

// HACK: Manually add `*const c_void`
impl IsPointerType for *const std::ffi::c_void {
    fn ir_type(context: &Context, _target: &TargetData) -> PointerType {
        context.i8_type().ptr_type(AddressSpace::Const)
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
