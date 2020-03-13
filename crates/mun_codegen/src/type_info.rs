use abi::Guid;
use std::hash::{Hash, Hasher};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TypeGroup {
    FundamentalTypes,
    StructTypes(hir::Struct),
}

impl From<TypeGroup> for u64 {
    fn from(group: TypeGroup) -> Self {
        match group {
            TypeGroup::FundamentalTypes => 0,
            TypeGroup::StructTypes(_) => 1,
        }
    }
}

#[derive(Clone, Debug, Eq)]
pub struct TypeInfo {
    pub guid: Guid,
    pub name: String,
    pub group: TypeGroup,
}

impl Hash for TypeInfo {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write(&self.guid.b)
    }
}

impl PartialEq for TypeInfo {
    fn eq(&self, other: &Self) -> bool {
        self.guid == other.guid
    }
}

impl Ord for TypeInfo {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.guid.cmp(&other.guid)
    }
}

impl PartialOrd for TypeInfo {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl TypeInfo {
    pub fn new<S: AsRef<str>>(name: S, group: TypeGroup) -> TypeInfo {
        TypeInfo {
            name: name.as_ref().to_string(),
            guid: Guid {
                b: md5::compute(name.as_ref()).0,
            },
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

impl HasStaticTypeInfo for std::ffi::c_void {
    fn type_info() -> TypeInfo {
        TypeInfo::new("core::void", TypeGroup::FundamentalTypes)
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
        TypeInfo::new("core::u64", TypeGroup::FundamentalTypes)
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

// HACK: Manually add `*const TypeInfo`
impl HasStaticTypeInfo for *const TypeInfo {
    fn type_info() -> TypeInfo {
        TypeInfo::new("*const TypeInfo", TypeGroup::FundamentalTypes)
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
