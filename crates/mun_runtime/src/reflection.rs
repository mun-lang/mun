use crate::type_info::HasStaticTypeInfo;
use crate::{marshal::Marshal, StructRef};
use md5;

/// Returns whether the specified argument type matches the `type_info`.
pub fn equals_argument_type<'e, 'f, T: ArgumentReflection>(
    type_info: &'e abi::TypeInfo,
    arg: &'f T,
) -> Result<(), (&'e str, &'f str)> {
    if type_info.guid != arg.type_guid() {
        Err((type_info.name(), arg.type_name()))
    } else {
        Ok(())
    }
}

/// Returns whether the specified return type matches the `type_info`.
pub fn equals_return_type<T: ReturnTypeReflection>(
    type_info: &abi::TypeInfo,
) -> Result<(), (&str, &str)> {
    match type_info.group {
        abi::TypeGroup::FundamentalTypes => {
            if type_info.guid != T::type_guid() {
                return Err((type_info.name(), T::type_name()));
            }
        }
        abi::TypeGroup::StructTypes => {
            if <StructRef as ReturnTypeReflection>::type_guid() != T::type_guid() {
                return Err(("struct", T::type_name()));
            }
        }
    }
    Ok(())
}

/// A type to emulate dynamic typing across compilation units for static types.
pub trait ReturnTypeReflection: Sized {
    /// The resulting type after marshaling.
    type Marshalled: Marshal<Self>;

    /// Retrieves the type's `Guid`.
    fn type_guid() -> abi::Guid {
        abi::Guid {
            b: md5::compute(Self::type_name()).0,
        }
    }

    /// Retrieves the type's name.
    fn type_name() -> &'static str;
}

/// A type to emulate dynamic typing across compilation units for statically typed values.
pub trait ArgumentReflection: Sized {
    /// The resulting type after dereferencing.
    type Marshalled: Marshal<Self>;

    /// Retrieves the `Guid` of the value's type.
    fn type_guid(&self) -> abi::Guid {
        abi::Guid {
            b: md5::compute(self.type_name()).0,
        }
    }

    /// Retrieves the name of the value's type.
    fn type_name(&self) -> &str;

    /// Marshals the value.
    fn marshal(self) -> Self::Marshalled;
}

macro_rules! impl_primitive_type {
    ($($ty:ty),+) => {
        $(
            impl ArgumentReflection for $ty {
                type Marshalled = Self;

                fn type_guid(&self) -> abi::Guid {
                    Self::type_info().guid
                }

                fn type_name(&self) -> &str {
                    Self::type_info().name()
                }

                fn marshal(self) -> Self::Marshalled {
                    self
                }
            }

            impl ReturnTypeReflection for $ty {
                type Marshalled = Self;

                fn type_name() -> &'static str {
                    Self::type_info().name()
                }

                fn type_guid() -> abi::Guid {
                    Self::type_info().guid
                }
            }
        )+
    }
}

impl_primitive_type!(i8, i16, i32, i64, isize, u8, u16, u32, u64, usize, f32, f64, bool);

impl ArgumentReflection for () {
    type Marshalled = Self;

    fn type_name(&self) -> &str {
        <Self as ReturnTypeReflection>::type_name()
    }

    fn marshal(self) -> Self::Marshalled {
        self
    }
}

impl ReturnTypeReflection for () {
    type Marshalled = ();

    fn type_name() -> &'static str {
        "core::empty"
    }
}

impl<T> ArgumentReflection for *const T
where
    *const T: HasStaticTypeInfo,
{
    type Marshalled = Self;

    fn type_guid(&self) -> abi::Guid {
        Self::type_info().guid
    }

    fn type_name(&self) -> &str {
        Self::type_info().name()
    }

    fn marshal(self) -> Self::Marshalled {
        self
    }
}

impl<T> ReturnTypeReflection for *const T
where
    *const T: HasStaticTypeInfo,
{
    type Marshalled = Self;

    fn type_guid() -> abi::Guid {
        Self::type_info().guid
    }

    fn type_name() -> &'static str {
        Self::type_info().name()
    }
}

impl<T> ArgumentReflection for *mut T
where
    *mut T: HasStaticTypeInfo,
{
    type Marshalled = Self;

    fn type_guid(&self) -> abi::Guid {
        Self::type_info().guid
    }

    fn type_name(&self) -> &str {
        Self::type_info().name()
    }

    fn marshal(self) -> Self::Marshalled {
        self
    }
}

impl<T> ReturnTypeReflection for *mut T
where
    *mut T: HasStaticTypeInfo,
{
    type Marshalled = Self;

    fn type_guid() -> abi::Guid {
        Self::type_info().guid
    }

    fn type_name() -> &'static str {
        Self::type_info().name()
    }
}
