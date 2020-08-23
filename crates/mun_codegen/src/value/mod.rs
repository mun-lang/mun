///! This module provides constructs to enable type safe handling of inkwell types.
mod array_value;
mod float_value;
mod function_value;
mod global;
mod int_value;
mod pointer_value;
mod string;
mod tuple_value;

use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::targets::TargetData;

pub use array_value::IterAsIrValue;
pub use global::Global;
pub use string::CanInternalize;

use inkwell::types::{BasicTypeEnum, PointerType, StructType};
use inkwell::values::{BasicValueEnum, PointerValue};
use inkwell::AddressSpace;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

/// Represents a generic inkwell value. This is a wrapper around inkwell types that enforces type
/// safety in the Rust compiler. Rust values that can be converted to inkwell types can be
/// represented as a value. e.g. `Value<u32>`. Internally this holds an `inkwell::values::IntValue`
/// but we maintain the information that it's actually a `u32` value.
///
/// There are several ways to enable a type to be used as a `Value<T>` type through the `AsValue`
/// trait.
///
/// - Implement the [`TransparentValue`] trait which converts the implementor into another
///   `Value<T>` type. Internally the inkwell value is taken from the inner type but a `Value<Self>`
///   is returned. This allows transparent composition. e.g.:
///
///   ```rust
///   # use mun_codegen::value::{AsValue, IrValueContext, TransparentValue, Value};
///   struct Foo {
///      value: u32,
///      bar: f32,
///   }
///
///   impl<'ink> TransparentValue<'ink> for Foo {
///       type Target = (u32, f32);
///
///       fn as_target_value(&self, context: &IrValueContext<'ink, '_, '_>) -> Value<'ink, Self::Target> {
///           (self.value, self.bar).as_value(context)
///       }
///   }
///   ```
///
///   This will result in an anonymous LLVM type: `type { u32, f32 }`
///
/// - Auto derive the `AsValue` trait e.g.:
///   ```ignore
///   #[macro_use] extern crate mun_codegen_macros;
///
///   #[derive(AsValue)]
///   struct Foo {
///       value: u32,
///       bar: f32
///   }
///   ```
///
///   This will result in a _named_ LLVM type: `%Foo = type { u32, f32 }`
///
/// - You can also add completely custom support by implementing the [`ConcreteValueType`] and
///   [`AsValue`] traits. Optionally you might also want to implement the [`SizedValueType`] and
///   [`PointerValueType`] traits.
pub struct Value<'ink, T: ConcreteValueType<'ink> + ?Sized> {
    pub value: T::Value,
}

/// When implemented enables the conversion from a value to a `Value<T>`.
pub trait AsValue<'ink, T: ConcreteValueType<'ink> + ?Sized> {
    /// Creates a `Value<T>` from an instance.
    fn as_value(&self, context: &IrValueContext<'ink, '_, '_>) -> Value<'ink, T>;
}

/// A `TransparentValue` is something that can be represented as a `Value<T>` but which is actually
/// a rewritten version of another type.
pub trait TransparentValue<'ink> {
    type Target: ConcreteValueType<'ink> + ?Sized;

    /// Converts the instance to the target value
    fn as_target_value(&self, context: &IrValueContext<'ink, '_, '_>) -> Value<'ink, Self::Target>;
}

/// The context in which an `IrType` operates.
pub struct IrTypeContext<'ink, 'a> {
    pub context: &'ink Context,
    pub target_data: &'a TargetData,
    pub struct_types: &'a RefCell<HashMap<&'static str, StructType<'ink>>>,
}

/// The context in which an `IrValue` exists.
pub struct IrValueContext<'ink, 'a, 'b> {
    pub context: &'ink Context,
    pub type_context: &'b IrTypeContext<'ink, 'a>,
    pub module: &'b Module<'ink>,
}

/// A trait that represents that a concrete type can be used as `Value<T>` type generic. A type must
/// implement this trait to be able to be represented as a `Value<Self>`.
pub trait ConcreteValueType<'ink> {
    type Value: ValueType<'ink>;
}

/// If we take a pointer of value T, which type can it return? This trait dictates that.
pub trait AddressableType<'ink, T: ?Sized> {
    /// Cast the pointer if required
    fn ptr_cast(
        value: PointerValue<'ink>,
        _context: &IrValueContext<'ink, '_, '_>,
    ) -> PointerValue<'ink> {
        value
    }
}

/// A trait implemented for types that can determine the IR type of a value without an instance.
pub trait SizedValueType<'ink>: ConcreteValueType<'ink> + Sized {
    /// Returns the IR type of a value of this type.
    fn get_ir_type(
        context: &IrTypeContext<'ink, '_>,
    ) -> <<Self as ConcreteValueType<'ink>>::Value as ValueType<'ink>>::Type;
}

/// A trait that returns the pointer type of the specified value.
pub trait PointerValueType<'ink>: ConcreteValueType<'ink> {
    /// Returns the pointer type of the value
    fn get_ptr_type(
        context: &IrTypeContext<'ink, '_>,
        address_space: Option<AddressSpace>,
    ) -> inkwell::types::PointerType<'ink>;
}

/// A trait that enables the conversion from an inkwell type to a corresponding value type. (e.g.
/// IntType -> IntValue)
pub trait TypeValue<'ink> {
    type Value: inkwell::values::AnyValue<'ink>;
}

/// A trait that enables the conversion from an inkwell value to a corresponding type. (e.g.
/// IntValue -> IntType)
pub trait ValueType<'ink>: Clone + Debug + Copy + Eq + PartialEq + Hash {
    type Type: inkwell::types::AnyType<'ink>;

    /// Returns the type of the value
    fn get_type(&self) -> Self::Type;
}

/// A trait that is implemented for types that can also be represented as a pointer.
pub trait AddressableTypeValue<'ink>: TypeValue<'ink> {
    fn ptr_type(&self, address_space: AddressSpace) -> inkwell::types::PointerType<'ink>;
}

/// A macro that implements basic traits for inkwell types.
macro_rules! impl_value_type_value {
    ($($ty:ty => $val:ty),+) => {
        $(
            impl<'ink> TypeValue<'ink> for $ty {
                type Value = $val;
            }
            impl<'ink> ValueType<'ink> for $val {
                type Type = $ty;

                fn get_type(&self) -> Self::Type {
                    Self::get_type(self)
                }
            }
        )*
    }
}

impl_value_type_value! (
    inkwell::types::IntType<'ink> => inkwell::values::IntValue<'ink>,
    inkwell::types::FloatType<'ink> => inkwell::values::FloatValue<'ink>,
    inkwell::types::ArrayType<'ink> => inkwell::values::ArrayValue<'ink>,
    inkwell::types::VectorType<'ink> => inkwell::values::VectorValue<'ink>,
    inkwell::types::StructType<'ink> => inkwell::values::StructValue<'ink>,
    inkwell::types::PointerType<'ink> => inkwell::values::PointerValue<'ink>,
    inkwell::types::FunctionType<'ink> => inkwell::values::FunctionValue<'ink>
);

macro_rules! impl_addressable_type_values {
    ($($ty:ty),+) => {
        $(
            impl<'ink> AddressableTypeValue<'ink> for $ty {
                fn ptr_type(&self, address_space: AddressSpace) -> inkwell::types::PointerType<'ink> {
                    Self::ptr_type(self, address_space)
                }
            }
        )*
    }
}

impl_addressable_type_values!(
    inkwell::types::IntType<'ink>,
    inkwell::types::FloatType<'ink>,
    inkwell::types::ArrayType<'ink>,
    inkwell::types::VectorType<'ink>,
    inkwell::types::StructType<'ink>,
    inkwell::types::PointerType<'ink>,
    inkwell::types::FunctionType<'ink>
);

impl<'ink> AddressableTypeValue<'ink> for inkwell::types::BasicTypeEnum<'ink> {
    fn ptr_type(&self, address_space: AddressSpace) -> PointerType<'ink> {
        match self {
            BasicTypeEnum::ArrayType(ty) => ty.ptr_type(address_space),
            BasicTypeEnum::FloatType(ty) => ty.ptr_type(address_space),
            BasicTypeEnum::IntType(ty) => ty.ptr_type(address_space),
            BasicTypeEnum::PointerType(ty) => ty.ptr_type(address_space),
            BasicTypeEnum::StructType(ty) => ty.ptr_type(address_space),
            BasicTypeEnum::VectorType(ty) => ty.ptr_type(address_space),
        }
    }
}

impl<'ink> TypeValue<'ink> for inkwell::types::BasicTypeEnum<'ink> {
    type Value = inkwell::values::BasicValueEnum<'ink>;
}

impl<'ink, T: ConcreteValueType<'ink> + ?Sized> Value<'ink, T> {
    /// Returns the type of the value.
    pub fn get_type(&self) -> <T::Value as ValueType<'ink>>::Type {
        <T::Value as ValueType>::get_type(&self.value)
    }

    /// Constructs a `Value<T>` from an inkwell value.
    pub(super) fn from_raw(value: T::Value) -> Value<'ink, T> {
        Value { value }
    }
}

impl<'ink, T: ConcreteValueType<'ink> + ?Sized> Value<'ink, *const T>
where
    *const T: SizedValueType<'ink, Value = PointerValue<'ink>>,
    <*const T as ConcreteValueType<'ink>>::Value: ValueType<'ink, Type = PointerType<'ink>>,
{
    /// Constructs a value by casting the specified pointer value to this value
    pub fn with_cast(value: PointerValue<'ink>, context: &IrValueContext<'ink, '_, '_>) -> Self {
        let target_type = <*const T>::get_ir_type(context.type_context);
        Value {
            value: if value.get_type() == target_type {
                value
            } else {
                value.const_cast(target_type)
            },
        }
    }
}

impl<'ink, T: ConcreteValueType<'ink> + ?Sized> Value<'ink, *mut T>
where
    *mut T: SizedValueType<'ink, Value = PointerValue<'ink>>,
    <*mut T as ConcreteValueType<'ink>>::Value: ValueType<'ink, Type = PointerType<'ink>>,
{
    /// Constructs a value by casting the specified pointer value to this value
    pub fn with_cast(value: PointerValue<'ink>, context: &IrValueContext<'ink, '_, '_>) -> Self {
        let target_type = <*mut T>::get_ir_type(context.type_context);
        Value {
            value: if value.get_type() == target_type {
                value
            } else {
                value.const_cast(target_type)
            },
        }
    }
}

impl<'ink, T: SizedValueType<'ink> + ?Sized> Value<'ink, T> {
    /// Returns the inkwell type of this `Value`.
    pub fn get_ir_type(context: &IrTypeContext<'ink, '_>) -> <T::Value as ValueType<'ink>>::Type {
        T::get_ir_type(context)
    }
}

impl<'ink, T: ConcreteValueType<'ink> + ?Sized> AsValue<'ink, T> for Value<'ink, T> {
    fn as_value(&self, _context: &IrValueContext<'ink, '_, '_>) -> Value<'ink, T> {
        *self
    }
}

impl<'ink, T: ConcreteValueType<'ink> + ?Sized> Clone for Value<'ink, T> {
    fn clone(&self) -> Self {
        Value { value: self.value }
    }
}

impl<'ink, T: ConcreteValueType<'ink> + ?Sized> Copy for Value<'ink, T> {}

impl<'ink, T: ConcreteValueType<'ink> + ?Sized> PartialEq for Value<'ink, T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<'ink, T: ConcreteValueType<'ink> + ?Sized> Eq for Value<'ink, T> {}

impl<'ink, T: ConcreteValueType<'ink> + ?Sized> Hash for Value<'ink, T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.value.hash(state)
    }
}

impl<'ink, T: ConcreteValueType<'ink> + ?Sized> std::fmt::Debug for Value<'ink, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self.value)
    }
}

impl<'ink, T: ConcreteValueType<'ink> + ?Sized> Into<inkwell::values::BasicValueEnum<'ink>>
    for Value<'ink, T>
where
    T::Value: Into<inkwell::values::BasicValueEnum<'ink>>,
{
    fn into(self) -> BasicValueEnum<'ink> {
        self.value.into()
    }
}

pub trait AsValueInto<'ink, T> {
    fn as_value_into(&self, context: &IrValueContext<'ink, '_, '_>) -> T;
}

impl<'ink, I, T: ConcreteValueType<'ink>> AsValueInto<'ink, I> for Value<'ink, T>
where
    T::Value: Into<I>,
{
    fn as_value_into(&self, _context: &IrValueContext<'ink, '_, '_>) -> I {
        self.value.into()
    }
}

impl<'ink, I, T: ConcreteValueType<'ink> + AsValue<'ink, T>> AsValueInto<'ink, I> for T
where
    T::Value: Into<I>,
{
    fn as_value_into(&self, context: &IrValueContext<'ink, '_, '_>) -> I {
        self.as_value(context).value.into()
    }
}

// A `TransparentValue` can also be represented by a `Value<T>`.
impl<'ink, T: TransparentValue<'ink>> ConcreteValueType<'ink> for T {
    type Value = <T::Target as ConcreteValueType<'ink>>::Value;
}

// A `TransparentValue` is sized if the target is also sized.
impl<'ink, T: TransparentValue<'ink>> SizedValueType<'ink> for T
where
    T::Target: SizedValueType<'ink>,
{
    fn get_ir_type(context: &IrTypeContext<'ink, '_>) -> <Self::Value as ValueType<'ink>>::Type {
        T::Target::get_ir_type(context)
    }
}

// If the target of the transparent value can statically return a pointer type, so can we.
impl<'ink, T: TransparentValue<'ink>> PointerValueType<'ink> for T
where
    T::Target: PointerValueType<'ink>,
{
    fn get_ptr_type(
        context: &IrTypeContext<'ink, '_>,
        address_space: Option<AddressSpace>,
    ) -> PointerType<'ink> {
        T::Target::get_ptr_type(context, address_space)
    }
}

// If the target is addressable as I, the transparent value is also addressable as I.
impl<
        'ink,
        I: ?Sized,
        U: ConcreteValueType<'ink> + ?Sized + AddressableType<'ink, I>,
        T: TransparentValue<'ink, Target = U>,
    > AddressableType<'ink, I> for T
{
    fn ptr_cast(
        value: PointerValue<'ink>,
        context: &IrValueContext<'ink, '_, '_>,
    ) -> PointerValue<'ink> {
        <T::Target as AddressableType<'ink, I>>::ptr_cast(value, context)
    }
}

// Transparent values can also be represented as `Value<Self>`.
impl<'ink, T> AsValue<'ink, T> for T
where
    T: TransparentValue<'ink>,
{
    fn as_value(&self, context: &IrValueContext<'ink, '_, '_>) -> Value<'ink, T> {
        Value::from_raw(self.as_target_value(context).value)
    }
}
