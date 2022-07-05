#![allow(clippy::mutable_key_type)]

use crate::{HasStaticTypeInfo, TypeInfo};
use lazy_static::lazy_static;
use std::{collections::HashMap, ptr::NonNull, sync::Arc};

type CastFn = fn(NonNull<u8>, NonNull<u8>);

macro_rules! insert_cast_fn {
    { $table:ident, $A:ty, $B:ty } => {
        $table.insert(
            (<$A>::type_info().clone(), <$B>::type_info().clone()),
            cast_from_to::<$A, $B> as CastFn,
        )
    }
}

lazy_static! {
    static ref CAST_FN_TABLE: HashMap<(Arc<TypeInfo>, Arc<TypeInfo>), CastFn> = {
        let mut table = HashMap::new();
        insert_cast_fn!(table, f32, f64);
        insert_cast_fn!(table, i8, i16);
        insert_cast_fn!(table, i8, i32);
        insert_cast_fn!(table, i8, i64);
        insert_cast_fn!(table, i8, i128);
        insert_cast_fn!(table, i16, i32);
        insert_cast_fn!(table, i16, i64);
        insert_cast_fn!(table, i16, i128);
        insert_cast_fn!(table, i32, i64);
        insert_cast_fn!(table, i32, i128);
        insert_cast_fn!(table, i64, i128);
        insert_cast_fn!(table, u8, i16);
        insert_cast_fn!(table, u8, u16);
        insert_cast_fn!(table, u8, i32);
        insert_cast_fn!(table, u8, u32);
        insert_cast_fn!(table, u8, i64);
        insert_cast_fn!(table, u8, u64);
        insert_cast_fn!(table, u8, i128);
        insert_cast_fn!(table, u8, u128);
        insert_cast_fn!(table, u16, i32);
        insert_cast_fn!(table, u16, u32);
        insert_cast_fn!(table, u16, i64);
        insert_cast_fn!(table, u16, u64);
        insert_cast_fn!(table, u16, i128);
        insert_cast_fn!(table, u16, u128);
        insert_cast_fn!(table, u32, i64);
        insert_cast_fn!(table, u32, u64);
        insert_cast_fn!(table, u32, i128);
        insert_cast_fn!(table, u32, u128);
        insert_cast_fn!(table, u64, i128);
        insert_cast_fn!(table, u64, u128);
        table
    };
}

fn cast_from_to<A, B>(src: NonNull<u8>, dest: NonNull<u8>)
where
    A: Copy + Into<B>,
{
    let value = unsafe { *src.cast::<A>().as_ref() };
    unsafe { *dest.cast::<B>().as_mut() = value.into() };
}

pub fn try_cast_from_to(
    old_id: Arc<TypeInfo>,
    new_id: Arc<TypeInfo>,
    src: NonNull<u8>,
    dest: NonNull<u8>,
) -> bool {
    if let Some(cast_fn) = CAST_FN_TABLE.get(&(old_id, new_id)) {
        cast_fn(src, dest);
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::try_cast_from_to;
    use crate::HasStaticTypeInfo;
    use std::ptr::NonNull;

    fn assert_cast<A, B>(a: A, mut b: B)
    where
        A: Copy + Into<B> + HasStaticTypeInfo,
        B: PartialEq + std::fmt::Debug + HasStaticTypeInfo,
    {
        assert!(try_cast_from_to(
            A::type_info().clone(),
            B::type_info().clone(),
            unsafe { NonNull::new_unchecked(&a as *const _ as *mut _) },
            unsafe { NonNull::new_unchecked(&mut b as *mut _) }.cast::<u8>(),
        ));
        assert_eq!(b, a.into());
    }

    #[test]
    fn cast_f32_to_f64() {
        assert_cast(std::f32::consts::PI, 0f64);
    }

    #[test]
    fn cast_i8_to_i16() {
        assert_cast(-5i8, 0i16);
    }

    #[test]
    fn cast_i8_to_i32() {
        assert_cast(-5i8, 0i32);
    }

    #[test]
    fn cast_i8_to_i64() {
        assert_cast(-5i8, 0i64);
    }

    #[test]
    fn cast_i8_to_i128() {
        assert_cast(-5i8, 0i128);
    }

    #[test]
    fn cast_i16_to_i32() {
        assert_cast(-5i16, 0i32);
    }

    #[test]
    fn cast_i16_to_i64() {
        assert_cast(-5i16, 0i64);
    }

    #[test]
    fn cast_i16_to_i128() {
        assert_cast(-5i16, 0i128);
    }

    #[test]
    fn cast_i32_to_i64() {
        assert_cast(-5i32, 0i64);
    }

    #[test]
    fn cast_i32_to_i128() {
        assert_cast(-5i32, 0i128);
    }

    #[test]
    fn cast_i64_to_i128() {
        assert_cast(-5i64, 0i128);
    }

    #[test]
    fn cast_u8_to_i16() {
        assert_cast(5u8, 0i16);
    }

    #[test]
    fn cast_u8_to_u16() {
        assert_cast(5u8, 0u16);
    }

    #[test]
    fn cast_u8_to_i32() {
        assert_cast(5u8, 0i32);
    }

    #[test]
    fn cast_u8_to_u32() {
        assert_cast(5u8, 0u32);
    }

    #[test]
    fn cast_u8_to_i64() {
        assert_cast(5u8, 0i64);
    }

    #[test]
    fn cast_u8_to_u64() {
        assert_cast(5u8, 0u64);
    }

    #[test]
    fn cast_u8_to_i128() {
        assert_cast(5u8, 0i128);
    }

    #[test]
    fn cast_u8_to_u128() {
        assert_cast(5u8, 0u128);
    }

    #[test]
    fn cast_u16_to_i32() {
        assert_cast(5u16, 0i32);
    }

    #[test]
    fn cast_u16_to_u32() {
        assert_cast(5u16, 0u32);
    }

    #[test]
    fn cast_u16_to_i64() {
        assert_cast(5u16, 0i64);
    }

    #[test]
    fn cast_u16_to_u64() {
        assert_cast(5u16, 0u64);
    }

    #[test]
    fn cast_u16_to_i128() {
        assert_cast(5u16, 0i128);
    }

    #[test]
    fn cast_u16_to_u128() {
        assert_cast(5u16, 0u128);
    }

    #[test]
    fn cast_u32_to_i64() {
        assert_cast(5u32, 0i64);
    }

    #[test]
    fn cast_u32_to_u64() {
        assert_cast(5u32, 0u64);
    }

    #[test]
    fn cast_u32_to_i128() {
        assert_cast(5u32, 0i128);
    }

    #[test]
    fn cast_u32_to_u128() {
        assert_cast(5u32, 0u128);
    }

    #[test]
    fn cast_u64_to_i128() {
        assert_cast(5u64, 0i128);
    }

    #[test]
    fn cast_u64_to_u128() {
        assert_cast(5u64, 0u128);
    }
}
