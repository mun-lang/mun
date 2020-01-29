use abi::TypeGroup;
use std::hash::{Hash, Hasher};

pub type Guid = [u8; 16];

#[derive(Clone, Eq, Ord, PartialOrd, Debug)]
pub struct TypeInfo {
    pub guid: Guid,
    pub name: String,
    pub group: TypeGroup,
}

impl Hash for TypeInfo {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write(&self.guid)
    }
}

impl PartialEq for TypeInfo {
    fn eq(&self, other: &Self) -> bool {
        self.guid == other.guid
    }
}

impl TypeInfo {
    pub fn new<S: AsRef<str>>(name: S, group: TypeGroup) -> TypeInfo {
        TypeInfo {
            name: name.as_ref().to_string(),
            guid: md5::compute(name.as_ref()).0,
            group,
        }
    }
}

/// A trait that statically defines that a type can be used as an argument.
pub trait HasStaticTypeInfo {
    fn type_info() -> TypeInfo;
}

pub trait HasTypeInfo {
    fn type_info(&self) -> TypeInfo;
}

impl<T: HasStaticTypeInfo> HasTypeInfo for T {
    fn type_info(&self) -> TypeInfo {
        T::type_info()
    }
}

impl HasStaticTypeInfo for u8 {
    fn type_info() -> TypeInfo {
        TypeInfo::new("core::u8", TypeGroup::FundamentalTypes)
    }
}

impl HasStaticTypeInfo for u64 {
    fn type_info() -> TypeInfo {
        TypeInfo::new("core::u64", TypeGroup::FundamentalTypes)
    }
}

impl HasStaticTypeInfo for i64 {
    fn type_info() -> TypeInfo {
        TypeInfo::new("core::i64", TypeGroup::FundamentalTypes)
    }
}

impl HasStaticTypeInfo for f32 {
    fn type_info() -> TypeInfo {
        TypeInfo::new("core::f32", TypeGroup::FundamentalTypes)
    }
}

impl HasStaticTypeInfo for bool {
    fn type_info() -> TypeInfo {
        TypeInfo::new("core::bool", TypeGroup::FundamentalTypes)
    }
}

impl<T: HasStaticTypeInfo> HasStaticTypeInfo for *mut T {
    fn type_info() -> TypeInfo {
        TypeInfo::new(
            format!("*mut {}", T::type_info().name),
            TypeGroup::FundamentalTypes,
        )
    }
}

impl HasStaticTypeInfo for usize {
    fn type_info() -> TypeInfo {
        TypeInfo::new("core::usize", TypeGroup::FundamentalTypes)
    }
}

impl<T: HasStaticTypeInfo> HasStaticTypeInfo for *const T {
    fn type_info() -> TypeInfo {
        TypeInfo::new(
            format!("*const {}", T::type_info().name),
            TypeGroup::FundamentalTypes,
        )
    }
}

/// A trait that statically defines that a type can be used as a return type for a function.
pub trait HasStaticReturnTypeInfo {
    fn return_type_info() -> Option<TypeInfo>;
}

/// A trait that defines that a type can be used as a return type for a function.
pub trait HasReturnTypeInfo {
    fn return_type_info(&self) -> Option<TypeInfo>;
}

impl<T: HasStaticReturnTypeInfo> HasReturnTypeInfo for T {
    fn return_type_info(&self) -> Option<TypeInfo> {
        T::return_type_info()
    }
}

impl<T: HasStaticTypeInfo> HasStaticReturnTypeInfo for T {
    fn return_type_info() -> Option<TypeInfo> {
        Some(T::type_info())
    }
}

impl HasStaticReturnTypeInfo for () {
    fn return_type_info() -> Option<TypeInfo> {
        None
    }
}
