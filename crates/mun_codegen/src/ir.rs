use inkwell::{
    context::Context,
    targets::TargetData,
    types::{
        AnyType, BasicMetadataTypeEnum, BasicType, BasicTypeEnum, FunctionType, IntType,
        PointerType,
    },
    AddressSpace,
};

mod array;
// pub mod body;
#[macro_use]
pub(crate) mod dispatch_table;
// pub mod file;
// pub(crate) mod file_group;
// pub mod function;
pub mod intrinsics;
mod reference;
pub mod ty;
pub(crate) mod type_table;
pub mod types;

/// Defines that a type has a static representation in inkwell
pub trait IsIrType<'ink> {
    type Type: AnyType<'ink>;

    fn ir_type(context: &'ink Context, target: &TargetData) -> Self::Type;
}

/// Defines that a type has a static represention in inkwell that can be
/// described as a `BasicType`.
pub trait IsBasicIrType<'ink> {
    fn ir_type(context: &'ink Context, target: &TargetData) -> BasicTypeEnum<'ink>;
}

impl<'ink, S: BasicType<'ink>, T: IsIrType<'ink, Type = S>> IsBasicIrType<'ink> for T {
    fn ir_type(context: &'ink Context, target: &TargetData) -> BasicTypeEnum<'ink> {
        Self::ir_type(context, target).as_basic_type_enum()
    }
}

/// Defines that a type can statically be used as a return type for a function
pub trait IsFunctionReturnType<'ink> {
    fn fn_type(
        context: &'ink Context,
        target: &TargetData,
        arg_types: &[BasicMetadataTypeEnum<'ink>],
        is_var_args: bool,
    ) -> FunctionType<'ink>;
}

/// All types that statically have a `BasicTypeEnum` can also be used as a
/// function return type
impl<'ink, T: IsBasicIrType<'ink>> IsFunctionReturnType<'ink> for T {
    fn fn_type(
        context: &'ink Context,
        target: &TargetData,
        arg_types: &[BasicMetadataTypeEnum<'ink>],
        is_var_args: bool,
    ) -> FunctionType<'ink> {
        T::ir_type(context, target).fn_type(arg_types, is_var_args)
    }
}

impl<'ink> IsFunctionReturnType<'ink> for () {
    fn fn_type(
        context: &'ink Context,
        _target: &TargetData,
        arg_types: &[BasicMetadataTypeEnum<'ink>],
        is_var_args: bool,
    ) -> FunctionType<'ink> {
        context.void_type().fn_type(arg_types, is_var_args)
    }
}

impl<'ink> IsIrType<'ink> for usize {
    type Type = IntType<'ink>;

    fn ir_type(context: &'ink Context, target: &TargetData) -> Self::Type {
        match target.get_pointer_byte_size(None) {
            4 => context.i32_type(),
            8 => context.i64_type(),
            _ => panic!("unsupported pointer byte size"),
        }
    }
}

impl<'ink> IsIrType<'ink> for isize {
    type Type = IntType<'ink>;

    fn ir_type(context: &'ink Context, target: &TargetData) -> Self::Type {
        match target.get_pointer_byte_size(None) {
            4 => context.i32_type(),
            8 => context.i64_type(),
            _ => panic!("unsupported pointer byte size"),
        }
    }
}

pub trait IsPointerType<'ink> {
    fn ir_type(context: &'ink Context, target: &TargetData) -> PointerType<'ink>;
}

impl<'ink, S: BasicType<'ink>, T: IsIrType<'ink, Type = S>> IsPointerType<'ink> for *const T {
    fn ir_type(context: &'ink Context, target: &TargetData) -> PointerType<'ink> {
        T::ir_type(context, target).ptr_type(AddressSpace::default())
    }
}

impl<'ink> IsPointerType<'ink> for *const std::ffi::c_void {
    fn ir_type(context: &'ink Context, _target: &TargetData) -> PointerType<'ink> {
        context.i8_type().ptr_type(AddressSpace::default())
    }
}

impl<'ink> IsPointerType<'ink> for *mut std::ffi::c_void {
    fn ir_type(context: &'ink Context, _target: &TargetData) -> PointerType<'ink> {
        context.i8_type().ptr_type(AddressSpace::default())
    }
}

impl<'ink, S: BasicType<'ink>, T: IsIrType<'ink, Type = S>> IsPointerType<'ink> for *mut T {
    fn ir_type(context: &'ink Context, target: &TargetData) -> PointerType<'ink> {
        T::ir_type(context, target).ptr_type(AddressSpace::default())
    }
}

impl<'ink, T: IsPointerType<'ink>> IsIrType<'ink> for T {
    type Type = PointerType<'ink>;

    fn ir_type(context: &'ink Context, target: &TargetData) -> Self::Type {
        T::ir_type(context, target)
    }
}
