use crate::{marshal::MarshalInto, Struct};
use abi::{Guid, TypeInfo};
use md5;

/// Returns whether the specified argument type matches the `type_info`.
pub fn equals_argument_type<'e, 'f, T: ArgumentReflection>(
    type_info: &'e TypeInfo,
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
    type_info: &TypeInfo,
) -> Result<(), (&str, &str)> {
    match type_info.group {
        abi::TypeGroup::FundamentalTypes => {
            if type_info.guid != T::type_guid() {
                return Err((type_info.name(), T::type_name()));
            }
        }
        abi::TypeGroup::StructTypes => {
            if <Struct as ReturnTypeReflection>::type_guid() != T::type_guid() {
                return Err(("struct", T::type_name()));
            }
        }
    }
    Ok(())
}

/// A type to emulate dynamic typing across compilation units for static types.
pub trait ReturnTypeReflection: Sized + 'static {
    /// The resulting type after marshaling.
    type Marshalled: MarshalInto<Self>;

    /// Retrieves the type's `Guid`.
    fn type_guid() -> Guid {
        Guid {
            b: md5::compute(Self::type_name()).0,
        }
    }

    /// Retrieves the type's name.
    fn type_name() -> &'static str;
}

/// A type to emulate dynamic typing across compilation units for statically typed values.
pub trait ArgumentReflection: Sized {
    /// The resulting type after dereferencing.
    type Marshalled: MarshalInto<Self>;

    /// Retrieves the `Guid` of the value's type.
    fn type_guid(&self) -> Guid {
        Guid {
            b: md5::compute(self.type_name()).0,
        }
    }

    /// Retrieves the name of the value's type.
    fn type_name(&self) -> &str;

    /// Marshals the value.
    fn marshal(self) -> Self::Marshalled;
}

impl ArgumentReflection for f64 {
    type Marshalled = Self;

    fn type_name(&self) -> &str {
        <Self as ReturnTypeReflection>::type_name()
    }

    fn marshal(self) -> Self::Marshalled {
        self
    }
}

impl ArgumentReflection for i64 {
    type Marshalled = Self;

    fn type_name(&self) -> &str {
        <Self as ReturnTypeReflection>::type_name()
    }

    fn marshal(self) -> Self::Marshalled {
        self
    }
}

impl ArgumentReflection for bool {
    type Marshalled = Self;

    fn type_name(&self) -> &str {
        <Self as ReturnTypeReflection>::type_name()
    }

    fn marshal(self) -> Self::Marshalled {
        self
    }
}

impl ArgumentReflection for () {
    type Marshalled = Self;

    fn type_name(&self) -> &str {
        <Self as ReturnTypeReflection>::type_name()
    }

    fn marshal(self) -> Self::Marshalled {
        self
    }
}

impl ReturnTypeReflection for f64 {
    type Marshalled = f64;

    fn type_name() -> &'static str {
        "core::float"
    }
}

impl ReturnTypeReflection for i64 {
    type Marshalled = i64;

    fn type_name() -> &'static str {
        "core::int"
    }
}

impl ReturnTypeReflection for bool {
    type Marshalled = bool;

    fn type_name() -> &'static str {
        "core::bool"
    }
}

impl ReturnTypeReflection for () {
    type Marshalled = ();

    fn type_name() -> &'static str {
        "core::empty"
    }
}
