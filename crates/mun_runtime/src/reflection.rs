use crate::{marshal::Marshal, Runtime, StructRef};
use abi::HasStaticTypeInfo;
use memory::{TypeInfo, TypeInfoData};
use std::sync::Arc;

/// Returns whether the specified return type matches the `type_info`.
pub fn equals_return_type<T: ReturnTypeReflection>(
    type_info: &TypeInfo,
) -> Result<(), (&str, &str)> {
    match type_info.data {
        TypeInfoData::Primitive => {
            if type_info.id != T::type_id() {
                return Err((&type_info.name, T::type_name()));
            }
        }
        TypeInfoData::Struct(_) => {
            if <StructRef as ReturnTypeReflection>::type_id() != T::type_id() {
                return Err(("struct", T::type_name()));
            }
        }
    }
    Ok(())
}

/// A type to emulate dynamic typing across compilation units for static types.
pub trait ReturnTypeReflection: Sized {
    /// Retrieves the type's `TypeId`.
    fn type_id() -> abi::TypeId;

    /// Retrieves the type's name.
    fn type_name() -> &'static str;
}

/// A type to emulate dynamic typing across compilation units for statically typed values.
pub trait ArgumentReflection: Sized {
    /// Retrieves the `TypeId` of the value's type.
    fn type_id(&self, runtime: &Runtime) -> abi::TypeId;

    /// Retrieves the name of the value's type.
    fn type_name<'r>(&'r self, runtime: &'r Runtime) -> &'r str;
}

macro_rules! impl_primitive_type {
    ($($ty:ty),+) => {
        $(
            impl ArgumentReflection for $ty {
                fn type_id(&self, _runtime: &Runtime) -> abi::TypeId {
                    Self::type_info().id
                }

                fn type_name(&self, _runtime: &Runtime) -> &str {
                    Self::type_info().name()
                }
            }

            impl ReturnTypeReflection for $ty {
                fn type_id() -> abi::TypeId {
                    Self::type_info().id
                }

                fn type_name() -> &'static str {
                    Self::type_info().name()
                }
            }

            impl<'t> Marshal<'t> for $ty {
                type MunType = $ty;

                fn marshal_from<'r>(value: Self::MunType, _runtime: &'r Runtime) -> Self
                where
                    Self: 't,
                    'r: 't,
                {
                    value
                }

                fn marshal_into<'r>(self) -> Self::MunType {
                    self
                }

                fn marshal_from_ptr<'r>(
                    ptr: std::ptr::NonNull<Self::MunType>,
                    _runtime: &'r Runtime,
                    _type_info: &Arc<TypeInfo>,
                ) -> Self
                where
                    Self: 't,
                    'r: 't,
                {
                    // TODO: Avoid unsafe `read` fn by using adding `Clone` trait to T.
                    // This also requires changes to the `impl Struct`
                    unsafe { ptr.as_ptr().read() }
                }

                fn marshal_to_ptr(
                    value: Self,
                    mut ptr: std::ptr::NonNull<Self::MunType>,
                    runtime: &Runtime,
                    _type_info: &Arc<TypeInfo>,
                ) {
                    unsafe { *ptr.as_mut() = value };
                }
            }
        )+
    }
}

impl_primitive_type!(
    i8,
    i16,
    i32,
    i64,
    i128,
    isize,
    u8,
    u16,
    u32,
    u64,
    u128,
    usize,
    f32,
    f64,
    bool,
    ()
);

impl<T> ArgumentReflection for *const T
where
    *const T: HasStaticTypeInfo,
{
    fn type_id(&self, _runtime: &Runtime) -> abi::TypeId {
        Self::type_info().id
    }

    fn type_name(&self, _runtime: &Runtime) -> &str {
        Self::type_info().name()
    }
}

impl<T> ReturnTypeReflection for *const T
where
    *const T: HasStaticTypeInfo,
{
    fn type_id() -> abi::TypeId {
        Self::type_info().id
    }

    fn type_name() -> &'static str {
        Self::type_info().name()
    }
}

impl<T> ArgumentReflection for *mut T
where
    *mut T: HasStaticTypeInfo,
{
    fn type_id(&self, _runtime: &Runtime) -> abi::TypeId {
        Self::type_info().id
    }

    fn type_name(&self, _runtime: &Runtime) -> &str {
        Self::type_info().name()
    }
}

impl<T> ReturnTypeReflection for *mut T
where
    *mut T: HasStaticTypeInfo,
{
    fn type_id() -> abi::TypeId {
        Self::type_info().id
    }

    fn type_name() -> &'static str {
        Self::type_info().name()
    }
}
