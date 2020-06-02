use crate::value::{
    AddressableType, AsValue, ConcreteValueType, IrTypeContext, IrValueContext, PointerValueType,
    SizedValueType, TypeValue, Value, ValueType,
};
use inkwell::types::{BasicType, PointerType};
use inkwell::AddressSpace;

impl<T: ConcreteValueType> ConcreteValueType for [T] {
    type Value = inkwell::values::ArrayValue;
}

impl<T: PointerValueType> PointerValueType for [T] {
    fn get_ptr_type(context: &IrTypeContext, address_space: Option<AddressSpace>) -> PointerType {
        T::get_ptr_type(context, address_space)
    }
}

impl<I, T: AddressableType<I>> AddressableType<I> for [T] {}
impl<I, T: AddressableType<I>> AddressableType<[I]> for [T] {}

macro_rules! impl_array(
    ($($size:expr),+) => {
        $(
            impl<T: SizedValueType> ConcreteValueType for [T; $size] {
                type Value = inkwell::values::ArrayValue;
            }

            impl<T: SizedValueType> SizedValueType for [T; $size]
            where
                <<T as ConcreteValueType>::Value as ValueType>::Type: ConstArrayType,
            {
                fn get_ir_type(context: &IrTypeContext) -> inkwell::types::ArrayType {
                    T::get_ir_type(context).array_type($size)
                }
            }

            impl<T: PointerValueType + SizedValueType> PointerValueType for [T; $size] {
                fn get_ptr_type(context: &IrTypeContext, address_space: Option<AddressSpace>) -> PointerType {
                    T::get_ptr_type(context, address_space)
                }
            }

            impl<E: SizedValueType, T: AsValue<E>> AsValue<[E; $size]> for [T; $size]
            where
                E::Value: ConstArrayValue
            {
                fn as_value(&self, context: &IrValueContext) -> Value<[E; $size]> {
                    let element_ir_type = E::get_ir_type(context.type_context);
                    let values: [E::Value; $size] =
                        array_init::from_iter(self.iter().map(|e| e.as_value(context).value))
                            .expect("unable to construct sized array");
                    let value = E::Value::const_array(values.as_ref(), &element_ir_type);
                    Value::from_raw(value)
                }
            }

            impl<E: SizedValueType, T: AsValue<E>> AsValue<[E]> for [T; $size]
                where
                    E::Value: ConstArrayValue
            {
                fn as_value(&self, context: &IrValueContext) -> Value<[E]> {
                    let element_ir_type = E::get_ir_type(context.type_context);
                    let values: [E::Value; $size] =
                        array_init::from_iter(self.iter().map(|e| e.as_value(context).value))
                            .expect("unable to construct sized array");
                    let value = E::Value::const_array(values.as_ref(), &element_ir_type);
                    Value::from_raw(value)
                }
            }
        )+
    }
);

impl_array!(
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 20, 24, 32, 36, 0x40, 0x80, 0x100
);

impl<E: SizedValueType, T: AsValue<E>> AsValue<[E]> for &[T]
where
    E::Value: ConstArrayValue,
{
    fn as_value(&self, context: &IrValueContext) -> Value<[E]> {
        let element_type = E::get_ir_type(context.type_context);
        let elements: Vec<E::Value> = self.iter().map(|v| v.as_value(context).value).collect();
        let value = ConstArrayValue::const_array(&elements, &element_type);
        Value::from_raw(value)
    }
}

pub trait IterAsIrValue<E: SizedValueType, T: AsValue<E>>: IntoIterator<Item = T> {
    /// Returns a `Value<[E]>` that contains all values converted to `Value<E>`.
    fn as_value(self, context: &IrValueContext) -> Value<[E]>;

    /// Constructs a const private global and returns a pointer to it.
    fn into_const_private_pointer<S: AsRef<str>>(
        self,
        name: S,
        context: &IrValueContext,
    ) -> Value<*const E>
    where
        *const E: ConcreteValueType<Value = inkwell::values::PointerValue>,
        E: AddressableType<E>,
        Self: Sized,
    {
        self.as_value(context)
            .into_const_private_global(name, context)
            .as_value(context)
    }

    /// Constructs a const private global and returns a pointer to it. If the iterator is empty a
    /// null pointer is returned.
    fn into_const_private_pointer_or_null<S: AsRef<str>>(
        self,
        name: S,
        context: &IrValueContext,
    ) -> Value<*const E>
    where
        *const E: SizedValueType<Value = inkwell::values::PointerValue>,
        E: AddressableType<E>,
        Self: Sized,
        E::Value: ConstArrayValue,
    {
        let mut iter = self.into_iter().peekable();
        if iter.peek().is_some() {
            iter.as_value(context)
                .into_const_private_global(name, context)
                .as_value(context)
        } else {
            Value::null(context)
        }
    }
}

impl<E: SizedValueType, T: AsValue<E>, I: IntoIterator<Item = T>> IterAsIrValue<E, T> for I
where
    E::Value: ConstArrayValue,
{
    fn as_value(self, context: &IrValueContext) -> Value<[E]> {
        let element_type = E::get_ir_type(context.type_context);
        let elements: Vec<E::Value> = self
            .into_iter()
            .map(|v| v.as_value(context).value)
            .collect();
        let value = ConstArrayValue::const_array(&elements, &element_type);
        Value::from_raw(value)
    }
}

/// A helper trait that enables the creation of a const LLVM array for a type.
pub trait ConstArrayType: BasicType + Sized + TypeValue {
    fn const_array(&self, values: &[<Self as TypeValue>::Value]) -> inkwell::values::ArrayValue;
}

/// A helper trait that enables the creation of a const LLVM array for a type.
pub trait ConstArrayValue: ValueType {
    fn const_array(values: &[Self], ir_type: &Self::Type) -> inkwell::values::ArrayValue;
}

impl<T: ConcreteValueType<Value = inkwell::values::ArrayValue> + ?Sized> Value<T> {
    /// Returns the number of elements in the array.
    pub fn len(self) -> usize {
        self.value.get_type().len() as usize
    }

    /// Returns true if the value represents an empty array.
    pub fn is_empty(self) -> bool {
        self.value.get_type().len() == 0
    }
}

macro_rules! impl_array_type {
    ($($inkwell_type:ty => $inkwell_value:ty),+) => {
        $(
            impl ConstArrayType for $inkwell_type {
                fn const_array(&self, values: &[<Self as TypeValue>::Value]) -> inkwell::values::ArrayValue {
                    <$inkwell_type>::const_array(self, values)
                }
            }

            impl ConstArrayValue for $inkwell_value {
               fn const_array(values: &[Self], ir_type: &Self::Type) -> inkwell::values::ArrayValue {
                    Self::Type::const_array(ir_type, values)
               }
            }
        )*
    };
}

impl_array_type!(
    inkwell::types::IntType => inkwell::values::IntValue,
    inkwell::types::FloatType => inkwell::values::FloatValue,
    inkwell::types::ArrayType => inkwell::values::ArrayValue,
    inkwell::types::VectorType => inkwell::values::VectorValue,
    inkwell::types::StructType => inkwell::values::StructValue,
    inkwell::types::PointerType => inkwell::values::PointerValue
);
