use crate::{marshal::Marshal, Runtime, StructRef};
use abi::HasStaticTypeInfo;
use once_cell::sync::OnceCell;

/// Returns whether the specified argument type matches the `type_info`.
pub fn equals_argument_type<'e, 'f, T: ArgumentReflection>(
    runtime: &'f Runtime,
    type_ref: &'e abi::TypeRef,
    arg: &'f T,
) -> Result<(), (&'e str, &'f str)> {
    if type_ref.guid != arg.type_guid(runtime) {
        Err((type_ref.name(), arg.type_name(runtime)))
    } else {
        Ok(())
    }
}

/// Returns whether the specified return type matches the `type_info`.
pub fn equals_return_type<T: ReturnTypeReflection>(
    type_ref: &abi::TypeRef,
) -> Result<(), (&str, &str)> {
    if let abi::TypeRefData::Struct { .. } = &type_ref.data {
        if <StructRef as ReturnTypeReflection>::type_guid() != T::type_guid() {
            return Err(("struct", T::type_name()));
        }
    } else if type_ref.guid != T::type_guid() {
        return Err((type_ref.name(), T::type_name()));
    }
    Ok(())
}

/// A type to emulate dynamic typing across compilation units for static types.
pub trait ReturnTypeReflection: Sized {
    /// Retrieves the type's `Guid`.
    fn type_guid() -> abi::Guid;

    /// Retrieves the type's name.
    fn type_name() -> &'static str;
}

/// A type to emulate dynamic typing across compilation units for statically typed values.
pub trait ArgumentReflection: Sized {
    /// Retrieves the `Guid` of the value's type.
    fn type_guid(&self, runtime: &Runtime) -> abi::Guid;

    /// Retrieves the name of the value's type.
    fn type_name<'r>(&'r self, runtime: &'r Runtime) -> &'r str;
}

macro_rules! impl_primitive_type {
    ($($ty:ty),+) => {
        $(
            impl ArgumentReflection for $ty {
                fn type_guid(&self, _runtime: &Runtime) -> abi::Guid {
                    Self::type_info().guid
                }

                fn type_name(&self, _runtime: &Runtime) -> &str {
                    Self::type_info().name()
                }
            }

            impl ReturnTypeReflection for $ty {
                fn type_guid() -> abi::Guid {
                    Self::type_info().guid
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
                    _type_info: Option<&abi::TypeRef>,
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
                    _type_info: Option<&abi::TypeRef>,
                ) {
                    unsafe { *ptr.as_mut() = value };
                }
            }
        )+
    }
}

impl_primitive_type!(
    i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64, bool
);

impl ReturnTypeReflection for () {
    fn type_name() -> &'static str {
        "core::empty"
    }

    fn type_guid() -> abi::Guid {
        // TODO: Once `const_fn` lands, replace this with a const md5 hash
        static GUID: OnceCell<abi::Guid> = OnceCell::new();
        *GUID.get_or_init(|| abi::Guid(md5::compute(Self::type_name()).0))
    }
}
impl<'t> Marshal<'t> for () {
    type MunType = ();

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
        _ptr: std::ptr::NonNull<Self::MunType>,
        _runtime: &'r Runtime,
        _type_info: Option<&abi::TypeRef>,
    ) -> Self
    where
        Self: 't,
        'r: 't,
    {
    }

    fn marshal_to_ptr(
        _value: Self,
        mut ptr: std::ptr::NonNull<Self::MunType>,
        _type_info: Option<&abi::TypeRef>,
    ) {
        unsafe { *ptr.as_mut() = () };
    }
}

impl<T> ArgumentReflection for *const T
where
    *const T: HasStaticTypeInfo,
{
    fn type_guid(&self, _runtime: &Runtime) -> abi::Guid {
        Self::type_info().guid
    }

    fn type_name(&self, _runtime: &Runtime) -> &str {
        Self::type_info().name()
    }
}

impl<T> ReturnTypeReflection for *const T
where
    *const T: HasStaticTypeInfo,
{
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
    fn type_guid(&self, _runtime: &Runtime) -> abi::Guid {
        Self::type_info().guid
    }

    fn type_name(&self, _runtime: &Runtime) -> &str {
        Self::type_info().name()
    }
}

impl<T> ReturnTypeReflection for *mut T
where
    *mut T: HasStaticTypeInfo,
{
    fn type_guid() -> abi::Guid {
        Self::type_info().guid
    }

    fn type_name() -> &'static str {
        Self::type_info().name()
    }
}
