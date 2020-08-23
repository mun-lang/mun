use super::ir::IsIrType;
use crate::IrDatabase;
use abi::Guid;
use inkwell::context::Context;
use inkwell::targets::TargetData;
use inkwell::types::AnyType;
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

impl TypeGroup {
    pub fn to_abi_type(&self) -> abi::TypeGroup {
        match self {
            TypeGroup::FundamentalTypes => abi::TypeGroup::FundamentalTypes,
            TypeGroup::StructTypes(_) => abi::TypeGroup::StructTypes,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeSize {
    // The size of the type in bits
    pub bit_size: u64,

    // The number of bytes required to store the type
    pub store_size: u64,

    // The number of bytes between successive object, including alignment and padding
    pub alloc_size: u64,

    // The alignment of the type
    pub alignment: u32,
}

impl TypeSize {
    pub fn from_ir_type(ty: &impl AnyType, target: &TargetData) -> Self {
        Self {
            bit_size: target.get_bit_size(ty),
            store_size: target.get_store_size(ty),
            alloc_size: target.get_abi_size(ty),
            alignment: target.get_abi_alignment(ty),
        }
    }
}

#[derive(Clone, Debug, Eq)]
pub struct TypeInfo {
    pub guid: Guid,
    pub name: String,
    pub group: TypeGroup,
    pub size: TypeSize,
}

impl Hash for TypeInfo {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write(&self.guid.0)
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
    pub fn new_fundamental<S: AsRef<str>>(name: S, type_size: TypeSize) -> TypeInfo {
        TypeInfo {
            name: name.as_ref().to_string(),
            guid: Guid(md5::compute(name.as_ref()).0),
            group: TypeGroup::FundamentalTypes,
            size: type_size,
        }
    }

    pub fn new_struct(db: &dyn IrDatabase, s: hir::Struct, type_size: TypeSize) -> TypeInfo {
        let name = s.name(db.upcast()).to_string();
        let guid_string = {
            let fields: Vec<String> = s
                .fields(db.upcast())
                .into_iter()
                .map(|f| {
                    let ty_string = f
                        .ty(db.upcast())
                        .guid_string(db.upcast())
                        .expect("type should be convertible to a string");
                    format!("{}: {}", f.name(db.upcast()).to_string(), ty_string)
                })
                .collect();

            format!(
                "struct {name}{{{fields}}}",
                name = &name,
                fields = fields.join(",")
            )
        };
        Self {
            guid: Guid(md5::compute(&guid_string).0),
            name,
            group: TypeGroup::StructTypes(s),
            size: type_size,
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
                fn type_info(context: &Context, target: &TargetData) -> TypeInfo {
                    let ty = <$ty as IsIrType>::ir_type(context, target);
                    TypeInfo::new_fundamental(
                        format!("core::{}", stringify!($ty)),
                        TypeSize::from_ir_type(&ty, target)
                    )
                }
            }
        )+
    }
}

impl_fundamental_static_type_info!(
    u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, f32, f64, bool
);

impl<T: HasStaticTypeName> HasStaticTypeInfo for *mut T {
    fn type_info(context: &Context, target: &TargetData) -> TypeInfo {
        let ty = target.ptr_sized_int_type(None);
        TypeInfo::new_fundamental(
            format!("*mut {}", T::type_name(context, target)),
            TypeSize::from_ir_type(&ty, target),
        )
    }
}

impl<T: HasStaticTypeName> HasStaticTypeInfo for *const T {
    fn type_info(
        context: &inkwell::context::Context,
        target: &inkwell::targets::TargetData,
    ) -> TypeInfo {
        let ty = target.ptr_sized_int_type(None);
        TypeInfo::new_fundamental(
            format!("*const {}", T::type_name(context, target)),
            TypeSize::from_ir_type(&ty, target),
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
