use std::hash::Hash;
use std::sync::{Arc, Once};

use abi::static_type_map::StaticTypeMap;
use abi::{self, Guid};

/// An owned version of a [`abi::TypeId`]. Using the `abi::TypeId` is cumbersome because it
/// involves dealing with pointers. The `TypeId` introduced here owns all data it refers to, which
/// makes it easier to work with from rust.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct TypeId {
    pub name: String,
    pub data: TypeIdData,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum TypeIdData {
    Concrete(Guid),
    Pointer(PointerTypeId),
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct PointerTypeId {
    pub pointee: Arc<TypeId>,
    pub mutable: bool,
}

pub trait HasStaticTypeId {
    fn type_id() -> &'static Arc<TypeId>;
}

macro_rules! impl_primitive_type_id {
    ($(
        $ty:ty
    ),+) => {
        $(
            impl HasStaticTypeId for $ty {
                fn type_id() -> &'static Arc<TypeId> {
                    static TYPE_INFO: once_cell::sync::OnceCell<Arc<TypeId>> = once_cell::sync::OnceCell::new();
                    TYPE_INFO.get_or_init(|| {
                        let guid = <$ty as abi::BuiltinType>::guid().clone();
                        let name = <$ty as abi::BuiltinType>::name().to_owned();
                        Arc::new(TypeId { name, data: TypeIdData::Concrete(guid) })
                    })
                }
            }
        )+
    }
}

impl_primitive_type_id! {
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
    (),
    std::ffi::c_void
}

impl<T: HasStaticTypeId + 'static> HasStaticTypeId for *const T {
    fn type_id() -> &'static Arc<TypeId> {
        static mut VALUE: Option<StaticTypeMap<Arc<TypeId>>> = None;
        static INIT: Once = Once::new();

        let map = unsafe {
            INIT.call_once(|| {
                VALUE = Some(StaticTypeMap::default());
            });
            VALUE.as_ref().unwrap()
        };

        map.call_once::<T, _>(|| {
            let element_type_id = T::type_id().clone();
            Arc::new(TypeId {
                name: format!("*const {}", &element_type_id.name),
                data: TypeIdData::Pointer(PointerTypeId {
                    pointee: T::type_id().clone(),
                    mutable: false,
                }),
            })
        })
    }
}

impl<T: HasStaticTypeId + 'static> HasStaticTypeId for *mut T {
    fn type_id() -> &'static Arc<TypeId> {
        static mut VALUE: Option<StaticTypeMap<Arc<TypeId>>> = None;
        static INIT: Once = Once::new();

        let map = unsafe {
            INIT.call_once(|| {
                VALUE = Some(StaticTypeMap::default());
            });
            VALUE.as_ref().unwrap()
        };

        map.call_once::<T, _>(|| {
            let element_type_id = T::type_id().clone();
            Arc::new(TypeId {
                name: format!("*mut {}", &element_type_id.name),
                data: TypeIdData::Pointer(PointerTypeId {
                    pointee: T::type_id().clone(),
                    mutable: true,
                }),
            })
        })
    }
}
