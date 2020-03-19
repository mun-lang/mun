use abi::Guid;
use inkwell::context::Context;
use inkwell::targets::TargetData;
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
    fn type_info(
        context: &inkwell::context::Context,
        target: &inkwell::targets::TargetData,
    ) -> TypeInfo;
}

pub trait HasTypeInfo {
    fn type_info(&self, context: &Context, target: &TargetData) -> TypeInfo;
}

impl<T: HasStaticTypeInfo> HasTypeInfo for T {
    fn type_info(&self, context: &Context, target: &TargetData) -> TypeInfo {
        T::type_info(context, target)
    }
}

pub trait HasStaticTypeName {
    fn type_name(context: &Context, target: &TargetData) -> String;
}

impl<T: HasStaticTypeInfo> HasStaticTypeName for T {
    fn type_name(context: &Context, target: &TargetData) -> String {
        T::type_info(context, target).name
    }
}

macro_rules! impl_fundamental_static_type_info {
    ($(
        $ty:ty
    ),+) => {
        $(
            impl HasStaticTypeInfo for $ty {
                fn type_info(_context: &Context, _target: &TargetData) -> TypeInfo {
                    //let ty = <$ty>::ir_type(context, target);
                    TypeInfo::new(
                        format!("core::{}", stringify!($ty)),
                        // target.get_abi_size(&ty),
                        // target.get_abi_alignment(&ty),
                        TypeGroup::FundamentalTypes,
                    )
                }
            }
        )+
    }
}

impl_fundamental_static_type_info!(u8, u16, u32, u64, i8, i16, i32, i64, f32, f64, bool);

impl<T: HasStaticTypeName> HasStaticTypeInfo for *mut T {
    fn type_info(context: &Context, target: &TargetData) -> TypeInfo {
        //let ty = target.ptr_sized_int_type(None);
        TypeInfo::new(
            format!("*mut {}", T::type_name(context, target)),
            // target.get_abi_size(&ty),
            // target.get_abi_alignment(&ty),
            TypeGroup::FundamentalTypes,
        )
    }
}

impl<T: HasStaticTypeName> HasStaticTypeInfo for *const T {
    fn type_info(
        context: &inkwell::context::Context,
        target: &inkwell::targets::TargetData,
    ) -> TypeInfo {
        //let ty = target.ptr_sized_int_type(None);
        TypeInfo::new(
            format!("*const {}", T::type_name(context, target)),
            // target.get_abi_size(&ty),
            // target.get_abi_alignment(&ty),
            TypeGroup::FundamentalTypes,
        )
    }
}

impl HasStaticTypeInfo for usize {
    fn type_info(context: &Context, target: &TargetData) -> TypeInfo {
        match target.get_pointer_byte_size(None) {
            4 => <u32 as HasStaticTypeInfo>::type_info(context, target),
            8 => <u64 as HasStaticTypeInfo>::type_info(context, target),
            _ => unreachable!("unsupported pointer byte size"),
        }
    }
}

impl HasStaticTypeInfo for isize {
    fn type_info(context: &Context, target: &TargetData) -> TypeInfo {
        match target.get_pointer_byte_size(None) {
            4 => <i32 as HasStaticTypeInfo>::type_info(context, target),
            8 => <i64 as HasStaticTypeInfo>::type_info(context, target),
            _ => unreachable!("unsupported pointer byte size"),
        }
    }
}

impl HasStaticTypeName for TypeInfo {
    fn type_name(_context: &Context, _target: &TargetData) -> String {
        "TypeInfo".to_owned()
    }
}

impl HasStaticTypeName for std::ffi::c_void {
    fn type_name(_context: &Context, _target: &TargetData) -> String {
        "core::void".to_owned()
    }
}

/// A trait that statically defines that a type can be used as a return type for a function.
pub trait HasStaticReturnTypeInfo {
    fn return_type_info(context: &Context, target: &TargetData) -> Option<TypeInfo>;
}

/// A trait that defines that a type can be used as a return type for a function.
pub trait HasReturnTypeInfo {
    fn return_type_info(&self, context: &Context, target: &TargetData) -> Option<TypeInfo>;
}

impl<T: HasStaticReturnTypeInfo> HasReturnTypeInfo for T {
    fn return_type_info(&self, context: &Context, target: &TargetData) -> Option<TypeInfo> {
        T::return_type_info(context, target)
    }
}

impl<T: HasStaticTypeInfo> HasStaticReturnTypeInfo for T {
    fn return_type_info(context: &Context, target: &TargetData) -> Option<TypeInfo> {
        Some(T::type_info(context, target))
    }
}

impl HasStaticReturnTypeInfo for () {
    fn return_type_info(_context: &Context, _target: &TargetData) -> Option<TypeInfo> {
        None
    }
}
