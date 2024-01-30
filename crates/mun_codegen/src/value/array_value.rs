use inkwell::{
    types::{BasicType, PointerType},
    values::PointerValue,
    AddressSpace,
};

use super::{
    AddressableType, AsValue, ConcreteValueType, HasConstValue, IrTypeContext, IrValueContext,
    PointerValueType, SizedValueType, TypeValue, Value, ValueType,
};

impl<'ink, T: ConcreteValueType<'ink>> ConcreteValueType<'ink> for [T] {
    type Value = inkwell::values::ArrayValue<'ink>;
}

impl<'ink, T: PointerValueType<'ink>> PointerValueType<'ink> for [T] {
    fn get_ptr_type(
        context: &IrTypeContext<'ink, '_>,
        address_space: Option<AddressSpace>,
    ) -> PointerType<'ink> {
        T::get_ptr_type(context, address_space)
    }
}

impl<'ink, I, T: AddressableType<'ink, I>> AddressableType<'ink, I> for [T] {
    fn ptr_cast(
        value: PointerValue<'ink>,
        _context: &IrValueContext<'ink, '_, '_>,
    ) -> PointerValue<'ink> {
        let ptr_type = value
            .get_type()
            .get_element_type()
            .into_array_type()
            .get_element_type()
            .ptr_type(value.get_type().get_address_space());
        value.const_cast(ptr_type)
    }
}

macro_rules! impl_array(
    ($($size:expr),+) => {
        $(
            impl<'ink, T: SizedValueType<'ink>> ConcreteValueType<'ink> for [T; $size] {
                type Value = inkwell::values::ArrayValue<'ink>;
            }

            impl<'ink, T: SizedValueType<'ink>> SizedValueType<'ink> for [T; $size]
            where
                <<T as ConcreteValueType<'ink>>::Value as ValueType<'ink>>::Type: ConstArrayType<'ink>,
            {
                fn get_ir_type(context: &IrTypeContext<'ink, '_>) -> inkwell::types::ArrayType<'ink> {
                    T::get_ir_type(context).array_type($size)
                }
            }

            impl<'ink, T: PointerValueType<'ink> + SizedValueType<'ink>> PointerValueType<'ink> for [T; $size] {
                fn get_ptr_type(context: &IrTypeContext<'ink, '_>, address_space: Option<AddressSpace>) -> PointerType<'ink> {
                    T::get_ptr_type(context, address_space)
                }
            }

            impl<'ink, E: SizedValueType<'ink>, T: AsValue<'ink, E>> AsValue<'ink, [E; $size]> for [T; $size]
            where
                E::Value: ConstArrayValue<'ink>
            {
                fn as_value(&self, context: &IrValueContext<'ink, '_, '_>) -> Value<'ink, [E; $size]> {
                    let element_ir_type = E::get_ir_type(context.type_context);
                    let values: [E::Value; $size] =
                        array_init::from_iter(self.iter().map(|e| e.as_value(context).value))
                            .expect("unable to construct sized array");
                    let value = E::Value::const_array(values.as_ref(), element_ir_type);
                    Value::from_raw(value)
                }
            }

            impl<'ink, E: SizedValueType<'ink>, T: AsValue<'ink, E>> AsValue<'ink, [E]> for [T; $size]
                where
                    E::Value: ConstArrayValue<'ink>
            {
                fn as_value(&self, context: &IrValueContext<'ink, '_, '_>) -> Value<'ink, [E]> {
                    let element_ir_type = E::get_ir_type(context.type_context);
                    let values: [E::Value; $size] =
                        array_init::from_iter(self.iter().map(|e| e.as_value(context).value))
                            .expect("unable to construct sized array");
                    let value = E::Value::const_array(values.as_ref(), element_ir_type);
                    Value::from_raw(value)
                }
            }

            impl<'ink, T> AddressableType<'ink, [T; $size]> for [T; $size] {}
        )+
    }
);

impl_array!(
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 20, 24, 32, 36, 0x40, 0x80, 0x100
);

impl<'ink, T: ConcreteValueType<'ink> + HasConstValue> HasConstValue for &[T] {
    fn has_const_value() -> bool {
        T::has_const_value()
    }
}

impl<'ink, E: SizedValueType<'ink>, T: AsValue<'ink, E>> AsValue<'ink, [E]> for &[T]
where
    E::Value: ConstArrayValue<'ink>,
{
    fn as_value(&self, context: &IrValueContext<'ink, '_, '_>) -> Value<'ink, [E]> {
        let element_type = E::get_ir_type(context.type_context);
        let elements: Vec<E::Value> = self.iter().map(|v| v.as_value(context).value).collect();
        let value = ConstArrayValue::const_array(&elements, element_type);
        Value::from_raw(value)
    }
}

pub trait IterAsIrValue<'ink, E: SizedValueType<'ink>, T: AsValue<'ink, E>>:
    IntoIterator<Item = T>
{
    /// Returns a `Value<[E]>` that contains all values converted to `Value<E>`.
    fn into_value(self, context: &IrValueContext<'ink, '_, '_>) -> Value<'ink, [E]>;

    /// Constructs a const private global and returns a pointer to it.
    fn into_const_private_pointer<S: AsRef<str>>(
        self,
        name: S,
        context: &IrValueContext<'ink, '_, '_>,
    ) -> Value<'ink, *const E>
    where
        *const E: ConcreteValueType<'ink, Value = inkwell::values::PointerValue<'ink>>,
        E: AddressableType<'ink, E>,
        E: PointerValueType<'ink>,
        Self: Sized,
    {
        self.into_value(context)
            .into_const_private_global(name, context)
            .as_value(context)
    }

    /// Constructs a const private global and returns a pointer to it. If the
    /// iterator is empty a null pointer is returned.
    fn into_const_private_pointer_or_null<S: AsRef<str>>(
        self,
        name: S,
        context: &IrValueContext<'ink, '_, '_>,
    ) -> Value<'ink, *const E>
    where
        *const E: SizedValueType<'ink, Value = inkwell::values::PointerValue<'ink>>,
        E: AddressableType<'ink, E>,
        E: PointerValueType<'ink>,
        Self: Sized,
        E::Value: ConstArrayValue<'ink>,
    {
        let mut iter = self.into_iter().peekable();
        if iter.peek().is_some() {
            iter.into_value(context)
                .into_const_private_global(name, context)
                .as_value(context)
        } else {
            Value::null(context)
        }
    }
}

impl<'ink, E: SizedValueType<'ink>, T: AsValue<'ink, E>, I: IntoIterator<Item = T>>
    IterAsIrValue<'ink, E, T> for I
where
    E::Value: ConstArrayValue<'ink>,
{
    fn into_value(self, context: &IrValueContext<'ink, '_, '_>) -> Value<'ink, [E]> {
        let element_type = E::get_ir_type(context.type_context);
        // eprintln!("constructing array of type {:?}", element_type);
        let elements: Vec<E::Value> = self
            .into_iter()
            .map(|v| v.as_value(context).value)
            // .map(|v| { eprintln!("- type {:?}", v.get_type()); v})
            .collect();
        let value = ConstArrayValue::const_array(&elements, element_type);
        // eprintln!("done, {} elements", elements.len());
        Value::from_raw(value)
    }
}

/// A helper trait that enables the creation of a const LLVM array for a type.
pub trait ConstArrayType<'ink>: BasicType<'ink> + Sized + TypeValue<'ink> {
    fn const_array(
        self,
        values: &[<Self as TypeValue<'ink>>::Value],
    ) -> inkwell::values::ArrayValue<'ink>;
}

/// A helper trait that enables the creation of a const LLVM array for a type.
pub trait ConstArrayValue<'ink>: ValueType<'ink> {
    fn const_array(values: &[Self], ir_type: Self::Type) -> inkwell::values::ArrayValue<'ink>;
}

impl<'ink, T: ConcreteValueType<'ink, Value = inkwell::values::ArrayValue<'ink>> + ?Sized>
    Value<'ink, T>
{
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
            impl<'ink> ConstArrayType<'ink> for $inkwell_type {
                fn const_array(self, values: &[<Self as TypeValue<'ink>>::Value]) -> inkwell::values::ArrayValue<'ink> {
                    <$inkwell_type>::const_array(self, values)
                }
            }

            impl<'ink> ConstArrayValue<'ink> for $inkwell_value {
               fn const_array(values: &[Self], ir_type: Self::Type) -> inkwell::values::ArrayValue<'ink> {
                    Self::Type::const_array(ir_type, values)
               }
            }
        )*
    };
}

impl_array_type!(
    inkwell::types::IntType<'ink> => inkwell::values::IntValue<'ink>,
    inkwell::types::FloatType<'ink> => inkwell::values::FloatValue<'ink>,
    inkwell::types::ArrayType<'ink> => inkwell::values::ArrayValue<'ink>,
    inkwell::types::VectorType<'ink> => inkwell::values::VectorValue<'ink>,
    inkwell::types::StructType<'ink> => inkwell::values::StructValue<'ink>,
    inkwell::types::PointerType<'ink> => inkwell::values::PointerValue<'ink>
);
