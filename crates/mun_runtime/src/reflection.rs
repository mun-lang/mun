use crate::{marshal::Marshal, Runtime};
use mun_memory::{HasStaticType, Type};

/// A type to emulate dynamic typing across compilation units for static types.
pub trait ReturnTypeReflection: Sized {
    /// Returns true if this specified type can be stored in an instance `Self`.
    fn accepts_type(ty: &Type) -> bool;

    /// Returns a type hint to indicate the name of this type
    fn type_hint() -> &'static str;
}

/// A type to emulate dynamic typing across compilation units for statically typed values.
pub trait ArgumentReflection: Sized {
    /// Retrieves the argument's type information.
    fn type_info(&self, runtime: &Runtime) -> Type;
}

macro_rules! impl_primitive_type {
    ($($ty:ty),+) => {
        $(
            impl ArgumentReflection for $ty {
                fn type_info(&self, _runtime: &Runtime) -> Type {
                    <Self as HasStaticType>::type_info().clone()
                }
            }

            impl ReturnTypeReflection for $ty {
                fn accepts_type(ty: &Type) -> bool {
                    ty == <Self as HasStaticType>::type_info()
                }

                fn type_hint() -> &'static str {
                    <Self as HasStaticType>::type_info().name()
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
                    _type_info: &Type,
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
                    _type_info: &Type,
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
    *const T: HasStaticType,
{
    fn type_info(&self, _runtime: &Runtime) -> Type {
        <Self as HasStaticType>::type_info().clone()
    }
}

impl<T> ReturnTypeReflection for *const T
where
    *const T: HasStaticType,
{
    fn accepts_type(ty: &Type) -> bool {
        <*const T as HasStaticType>::type_info() == ty
    }

    fn type_hint() -> &'static str {
        <*const T as HasStaticType>::type_info().name()
    }
}

impl<T> ArgumentReflection for *mut T
where
    *mut T: HasStaticType,
{
    fn type_info(&self, _runtime: &Runtime) -> Type {
        <Self as HasStaticType>::type_info().clone()
    }
}

impl<T> ReturnTypeReflection for *mut T
where
    *mut T: HasStaticType,
{
    fn accepts_type(ty: &Type) -> bool {
        <*mut T as HasStaticType>::type_info() == ty
    }

    fn type_hint() -> &'static str {
        <*mut T as HasStaticType>::type_info().name()
    }
}
