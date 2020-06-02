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

use inkwell::types::{PointerType, StructType};
use inkwell::values::BasicValueEnum;
use inkwell::AddressSpace;
use parking_lot::RwLock;
use std::any::TypeId;
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
///   impl TransparentValue for Foo {
///       type Target = (u32, f32);
///
///       fn as_target_value(&self, context: &IrValueContext) -> Value<Self::Target> {
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
///   #[ir_name = "Foo"]
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
pub struct Value<T: ConcreteValueType + ?Sized> {
    pub value: T::Value,
}

/// When implemented enables the conversion from a value to a `Value<T>`.
pub trait AsValue<T: ConcreteValueType + ?Sized> {
    /// Creates a `Value<T>` from an instance.
    fn as_value(&self, context: &IrValueContext) -> Value<T>;
}

/// A `TransparentValue` is something that can be represented as a `Value<T>` but which is actually
/// a rewritten version of another type.
pub trait TransparentValue {
    type Target: ConcreteValueType + ?Sized;

    /// Converts the instance to the target value
    fn as_target_value(&self, context: &IrValueContext) -> Value<Self::Target>;
}

/// The context in which an `IrType` operates.
pub struct IrTypeContext<'ctx> {
    pub context: &'ctx Context,
    pub target_data: &'ctx TargetData,
    pub struct_types: RwLock<HashMap<(&'static str, TypeId), StructType>>,
}

/// The context in which an `IrValue` exists.
pub struct IrValueContext<'t, 'ctx, 'm> {
    pub type_context: &'t IrTypeContext<'ctx>,
    pub context: &'ctx Context,
    pub module: &'m Module,
}

/// A trait that represents that a concrete type can be used as `Value<T>` type generic. A type must
/// implement this trait to be able to be represented as a `Value<Self>`.
pub trait ConcreteValueType {
    type Value: ValueType;
}

/// If we take a pointer of value T, which type can it return? This trait dictates that.
pub trait AddressableType<T: ?Sized> {}

/// A trait implemented for types that can determine the IR type of a value without an instance.
pub trait SizedValueType: ConcreteValueType + Sized {
    /// Returns the IR type of a value of this type.
    fn get_ir_type(
        context: &IrTypeContext,
    ) -> <<Self as ConcreteValueType>::Value as ValueType>::Type;
}

/// A trait that returns the pointer type of the specified value.
pub trait PointerValueType: ConcreteValueType {
    /// Returns the pointer type of the value
    fn get_ptr_type(
        context: &IrTypeContext,
        address_space: Option<AddressSpace>,
    ) -> inkwell::types::PointerType;
}

/// A trait that enables the conversion from an inkwell type to a corresponding value type. (e.g.
/// IntType -> IntValue)
pub trait TypeValue {
    type Value: inkwell::values::AnyValue;
}

/// A trait that enables the conversion from an inkwell value to a corresponding type. (e.g.
/// IntValue -> IntType)
pub trait ValueType: Clone + Debug + Copy + Eq + PartialEq + Hash {
    type Type: inkwell::types::AnyType;

    /// Returns the type of the value
    fn get_type(&self) -> Self::Type;
}

/// A trait that is implemented for types that can also be represented as a pointer.
pub trait AddressableTypeValue: TypeValue {
    fn ptr_type(&self, address_space: AddressSpace) -> inkwell::types::PointerType;
}

/// A macro that implements basic traits for inkwell types.
macro_rules! impl_value_type_value {
    ($($ty:ty => $val:ty),+) => {
        $(
            impl TypeValue for $ty {
                type Value = $val;
            }
            impl ValueType for $val {
                type Type = $ty;

                fn get_type(&self) -> Self::Type {
                    self.get_type()
                }
            }
        )*
    }
}

impl_value_type_value! (
    inkwell::types::IntType => inkwell::values::IntValue,
    inkwell::types::FloatType => inkwell::values::FloatValue,
    inkwell::types::ArrayType => inkwell::values::ArrayValue,
    inkwell::types::VectorType => inkwell::values::VectorValue,
    inkwell::types::StructType => inkwell::values::StructValue,
    inkwell::types::PointerType => inkwell::values::PointerValue,
    inkwell::types::FunctionType => inkwell::values::FunctionValue
);

macro_rules! impl_addressable_type_values {
    ($($ty:ty),+) => {
        $(
            impl AddressableTypeValue for $ty {
                fn ptr_type(&self, address_space: AddressSpace) -> inkwell::types::PointerType {
                    self.ptr_type(address_space)
                }
            }
        )*
    }
}

impl_addressable_type_values!(
    inkwell::types::IntType,
    inkwell::types::FloatType,
    inkwell::types::ArrayType,
    inkwell::types::VectorType,
    inkwell::types::StructType,
    inkwell::types::PointerType,
    inkwell::types::FunctionType
);

impl<T: ConcreteValueType + ?Sized> Value<T> {
    /// Returns the type of the value.
    pub fn get_type(&self) -> <T::Value as ValueType>::Type {
        <T::Value as ValueType>::get_type(&self.value)
    }

    /// Constructs a `Value<T>` from an inkwell value.
    pub(super) fn from_raw(value: T::Value) -> Value<T> {
        Value { value }
    }
}

impl<T: SizedValueType + ?Sized> Value<T> {
    /// Returns the inkwell type of this `Value`.
    pub fn get_ir_type(context: &IrTypeContext) -> <T::Value as ValueType>::Type {
        T::get_ir_type(context)
    }
}

impl<T: ConcreteValueType + ?Sized> AsValue<T> for Value<T> {
    fn as_value(&self, _context: &IrValueContext) -> Value<T> {
        *self
    }
}

impl<T: ConcreteValueType + ?Sized> Clone for Value<T> {
    fn clone(&self) -> Self {
        Value { value: self.value }
    }
}

impl<T: ConcreteValueType + ?Sized> Copy for Value<T> {}

impl<T: ConcreteValueType + ?Sized> PartialEq for Value<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<T: ConcreteValueType + ?Sized> Eq for Value<T> {}

impl<T: ConcreteValueType + ?Sized> Hash for Value<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.value.hash(state)
    }
}

impl<T: ConcreteValueType + ?Sized> std::fmt::Debug for Value<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self.value)
    }
}

impl<T: ConcreteValueType + ?Sized> Into<inkwell::values::BasicValueEnum> for Value<T>
where
    T::Value: Into<inkwell::values::BasicValueEnum>,
{
    fn into(self) -> BasicValueEnum {
        self.value.into()
    }
}

pub trait AsValueInto<T> {
    fn as_value_into(&self, context: &IrValueContext) -> T;
}

impl<I, T: ConcreteValueType> AsValueInto<I> for Value<T>
where
    T::Value: Into<I>,
{
    fn as_value_into(&self, _context: &IrValueContext) -> I {
        self.value.into()
    }
}

impl<I, T: ConcreteValueType + AsValue<T>> AsValueInto<I> for T
where
    T::Value: Into<I>,
{
    fn as_value_into(&self, context: &IrValueContext) -> I {
        self.as_value(context).value.into()
    }
}

// A `TransparentValue` can also be represented by a `Value<T>`.
impl<T: TransparentValue> ConcreteValueType for T {
    type Value = <T::Target as ConcreteValueType>::Value;
}

// A `TransparentValue` is sized if the target is also sized.
impl<T: TransparentValue> SizedValueType for T
where
    T::Target: SizedValueType,
{
    fn get_ir_type(context: &IrTypeContext) -> <Self::Value as ValueType>::Type {
        T::Target::get_ir_type(context)
    }
}

// If the target of the transparent value can statically return a pointer type, so can we.
impl<T: TransparentValue> PointerValueType for T
where
    T::Target: PointerValueType,
{
    fn get_ptr_type(context: &IrTypeContext, address_space: Option<AddressSpace>) -> PointerType {
        T::Target::get_ptr_type(context, address_space)
    }
}

// Transparent values can also be represented as `Value<Self>`.
impl<T> AsValue<T> for T
where
    T: TransparentValue,
{
    fn as_value(&self, context: &IrValueContext) -> Value<T> {
        Value::from_raw(self.as_target_value(context).value)
    }
}
