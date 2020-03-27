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

impl ArgumentReflection for f64 {
    type Marshalled = Self;

    fn type_name(&self) -> &str {
        <Self as ReturnTypeReflection>::type_name()
    }

    fn marshal(self) -> Self::Marshalled {
        self
    }

    fn type_guid(&self) -> abi::Guid {
        <Self as ReturnTypeReflection>::type_guid()
    }
}

impl ArgumentReflection for f32 {
    type Marshalled = Self;

    fn type_name(&self) -> &str {
        <Self as ReturnTypeReflection>::type_name()
    }

    fn marshal(self) -> Self::Marshalled {
        self
    }

    fn type_guid(&self) -> abi::Guid {
        <Self as ReturnTypeReflection>::type_guid()
    }
}

impl ArgumentReflection for isize {
    type Marshalled = Self;

    fn type_name(&self) -> &str {
        <Self as ReturnTypeReflection>::type_name()
    }

    fn marshal(self) -> Self::Marshalled {
        self
    }

    fn type_guid(&self) -> abi::Guid {
        <Self as ReturnTypeReflection>::type_guid()
    }
}

impl ArgumentReflection for usize {
    type Marshalled = Self;

    fn type_name(&self) -> &str {
        <Self as ReturnTypeReflection>::type_name()
    }

    fn marshal(self) -> Self::Marshalled {
        self
    }

    fn type_guid(&self) -> abi::Guid {
        <Self as ReturnTypeReflection>::type_guid()
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

    fn type_guid(&self) -> abi::Guid {
        <Self as ReturnTypeReflection>::type_guid()
    }
}

impl ArgumentReflection for i32 {
    type Marshalled = Self;

    fn type_name(&self) -> &str {
        <Self as ReturnTypeReflection>::type_name()
    }

    fn marshal(self) -> Self::Marshalled {
        self
    }

    fn type_guid(&self) -> abi::Guid {
        <Self as ReturnTypeReflection>::type_guid()
    }
}

impl ArgumentReflection for i16 {
    type Marshalled = Self;

    fn type_name(&self) -> &str {
        <Self as ReturnTypeReflection>::type_name()
    }

    fn marshal(self) -> Self::Marshalled {
        self
    }

    fn type_guid(&self) -> abi::Guid {
        <Self as ReturnTypeReflection>::type_guid()
    }
}

impl ArgumentReflection for i8 {
    type Marshalled = Self;

    fn type_name(&self) -> &str {
        <Self as ReturnTypeReflection>::type_name()
    }

    fn marshal(self) -> Self::Marshalled {
        self
    }

    fn type_guid(&self) -> abi::Guid {
        <Self as ReturnTypeReflection>::type_guid()
    }
}

impl ArgumentReflection for u64 {
    type Marshalled = Self;

    fn type_name(&self) -> &str {
        <Self as ReturnTypeReflection>::type_name()
    }

    fn marshal(self) -> Self::Marshalled {
        self
    }

    fn type_guid(&self) -> abi::Guid {
        <Self as ReturnTypeReflection>::type_guid()
    }
}

impl ArgumentReflection for u32 {
    type Marshalled = Self;

    fn type_name(&self) -> &str {
        <Self as ReturnTypeReflection>::type_name()
    }

    fn marshal(self) -> Self::Marshalled {
        self
    }

    fn type_guid(&self) -> abi::Guid {
        <Self as ReturnTypeReflection>::type_guid()
    }
}

impl ArgumentReflection for u16 {
    type Marshalled = Self;

    fn type_name(&self) -> &str {
        <Self as ReturnTypeReflection>::type_name()
    }

    fn marshal(self) -> Self::Marshalled {
        self
    }

    fn type_guid(&self) -> abi::Guid {
        <Self as ReturnTypeReflection>::type_guid()
    }
}

impl ArgumentReflection for u8 {
    type Marshalled = Self;

    fn type_name(&self) -> &str {
        <Self as ReturnTypeReflection>::type_name()
    }

    fn marshal(self) -> Self::Marshalled {
        self
    }

    fn type_guid(&self) -> abi::Guid {
        <Self as ReturnTypeReflection>::type_guid()
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

    fn type_guid(&self) -> abi::Guid {
        <Self as ReturnTypeReflection>::type_guid()
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

impl ArgumentReflection for *const u8 {
    type Marshalled = Self;

    fn type_name(&self) -> &str {
        <Self as ReturnTypeReflection>::type_name()
    }

    fn marshal(self) -> Self::Marshalled {
        self
    }

    fn type_guid(&self) -> abi::Guid {
        <Self as ReturnTypeReflection>::type_guid()
    }
}

impl ArgumentReflection for *mut u8 {
    type Marshalled = Self;

    fn type_name(&self) -> &str {
        <Self as ReturnTypeReflection>::type_name()
    }

    fn marshal(self) -> Self::Marshalled {
        self
    }

    fn type_guid(&self) -> abi::Guid {
        <Self as ReturnTypeReflection>::type_guid()
    }
}

impl ArgumentReflection for *const abi::TypeInfo {
    type Marshalled = Self;

    fn type_name(&self) -> &str {
        "*const TypeInfo"
    }

    fn marshal(self) -> Self::Marshalled {
        self
    }

    fn type_guid(&self) -> abi::Guid {
        <Self as ReturnTypeReflection>::type_guid()
    }
}

impl ArgumentReflection for *const std::ffi::c_void {
    type Marshalled = Self;

    fn type_name(&self) -> &str {
        "*const core::void"
    }

    fn marshal(self) -> Self::Marshalled {
        self
    }

    fn type_guid(&self) -> abi::Guid {
        <Self as ReturnTypeReflection>::type_guid()
    }
}

impl ArgumentReflection for *mut std::ffi::c_void {
    type Marshalled = Self;

    fn type_name(&self) -> &str {
        "*mut core::void"
    }

    fn marshal(self) -> Self::Marshalled {
        self
    }

    fn type_guid(&self) -> abi::Guid {
        <Self as ReturnTypeReflection>::type_guid()
    }
}

impl ArgumentReflection for *const *mut std::ffi::c_void {
    type Marshalled = Self;

    fn type_name(&self) -> &str {
        "*const *mut core::void"
    }

    fn marshal(self) -> Self::Marshalled {
        self
    }

    fn type_guid(&self) -> abi::Guid {
        <Self as ReturnTypeReflection>::type_guid()
    }
}

impl ReturnTypeReflection for *const abi::TypeInfo {
    type Marshalled = Self;

    fn type_name() -> &'static str {
        "*const TypeInfo"
    }

    fn type_guid() -> abi::Guid {
        <Self>::type_info().guid
    }
}

impl ReturnTypeReflection for *const std::ffi::c_void {
    type Marshalled = Self;

    fn type_name() -> &'static str {
        "*const core::void"
    }

    fn type_guid() -> abi::Guid {
        <Self>::type_info().guid
    }
}

impl ReturnTypeReflection for *mut std::ffi::c_void {
    type Marshalled = Self;

    fn type_name() -> &'static str {
        "*mut core::void"
    }

    fn type_guid() -> abi::Guid {
        <Self>::type_info().guid
    }
}

impl ReturnTypeReflection for *const *mut std::ffi::c_void {
    type Marshalled = Self;

    fn type_name() -> &'static str {
        "*const *mut core::void"
    }

    fn type_guid() -> abi::Guid {
        <Self>::type_info().guid
    }
}

impl ReturnTypeReflection for f64 {
    type Marshalled = f64;

    fn type_name() -> &'static str {
        "core::f64"
    }

    fn type_guid() -> abi::Guid {
        <Self>::type_info().guid
    }
}

impl ReturnTypeReflection for f32 {
    type Marshalled = f32;

    fn type_name() -> &'static str {
        "core::f32"
    }

    fn type_guid() -> abi::Guid {
        <Self>::type_info().guid
    }
}

#[cfg(target_pointer_width = "64")]
impl ReturnTypeReflection for isize {
    type Marshalled = isize;

    fn type_name() -> &'static str {
        "core::i64"
    }

    fn type_guid() -> abi::Guid {
        <Self>::type_info().guid
    }
}

#[cfg(target_pointer_width = "32")]
impl ReturnTypeReflection for isize {
    type Marshalled = isize;

    fn type_name() -> &'static str {
        "core::i32"
    }

    fn type_guid() -> abi::Guid {
        <Self>::type_info().guid
    }
}

impl ReturnTypeReflection for i64 {
    type Marshalled = i64;

    fn type_name() -> &'static str {
        "core::i64"
    }

    fn type_guid() -> abi::Guid {
        <Self>::type_info().guid
    }
}

impl ReturnTypeReflection for i32 {
    type Marshalled = i32;

    fn type_name() -> &'static str {
        "core::i32"
    }

    fn type_guid() -> abi::Guid {
        <Self>::type_info().guid
    }
}

impl ReturnTypeReflection for i16 {
    type Marshalled = i16;

    fn type_name() -> &'static str {
        "core::i16"
    }

    fn type_guid() -> abi::Guid {
        <Self>::type_info().guid
    }
}

impl ReturnTypeReflection for i8 {
    type Marshalled = i8;

    fn type_name() -> &'static str {
        "core::i8"
    }

    fn type_guid() -> abi::Guid {
        <Self>::type_info().guid
    }
}

#[cfg(target_pointer_width = "64")]
impl ReturnTypeReflection for usize {
    type Marshalled = usize;

    fn type_name() -> &'static str {
        "core::u64"
    }

    fn type_guid() -> abi::Guid {
        <Self>::type_info().guid
    }
}

#[cfg(target_pointer_width = "32")]
impl ReturnTypeReflection for usize {
    type Marshalled = usize;

    fn type_name() -> &'static str {
        "core::u32"
    }

    fn type_guid() -> abi::Guid {
        <Self>::type_info().guid
    }
}

impl ReturnTypeReflection for u64 {
    type Marshalled = u64;

    fn type_name() -> &'static str {
        "core::u64"
    }

    fn type_guid() -> abi::Guid {
        <Self>::type_info().guid
    }
}

impl ReturnTypeReflection for u32 {
    type Marshalled = u32;

    fn type_name() -> &'static str {
        "core::u32"
    }

    fn type_guid() -> abi::Guid {
        <Self>::type_info().guid
    }
}

impl ReturnTypeReflection for u16 {
    type Marshalled = u16;

    fn type_name() -> &'static str {
        "core::u16"
    }

    fn type_guid() -> abi::Guid {
        <Self>::type_info().guid
    }
}

impl ReturnTypeReflection for u8 {
    type Marshalled = u8;

    fn type_name() -> &'static str {
        "core::u8"
    }

    fn type_guid() -> abi::Guid {
        <Self>::type_info().guid
    }
}

impl ReturnTypeReflection for bool {
    type Marshalled = bool;

    fn type_name() -> &'static str {
        "core::bool"
    }

    fn type_guid() -> abi::Guid {
        <Self>::type_info().guid
    }
}

impl ReturnTypeReflection for () {
    type Marshalled = ();

    fn type_name() -> &'static str {
        "core::empty"
    }
}

impl ReturnTypeReflection for *const u8 {
    type Marshalled = Self;

    fn type_name() -> &'static str {
        "*const core::u8"
    }

    fn type_guid() -> abi::Guid {
        <Self>::type_info().guid
    }
}

impl ReturnTypeReflection for *mut u8 {
    type Marshalled = Self;

    fn type_name() -> &'static str {
        "*mut core::u8"
    }

    fn type_guid() -> abi::Guid {
        <*mut u8>::type_info().guid
    }
}
