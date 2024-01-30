//! Taken from the
//! [librustc_target](https://github.com/rust-lang/rust/tree/master/src/librustc_target) crate.

// use crate::abi::{Align, HasDataLayout};
use std::convert::TryInto;
// use std::ops::{Add, AddAssign, Mul, Sub};

/// Size of a type in bytes.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Size {
    raw: u64,
}

impl Size {
    pub const ZERO: Size = Size { raw: 0 };

    #[inline]
    pub fn from_bits(bits: impl TryInto<u64>) -> Size {
        let bits = bits.try_into().ok().unwrap();
        // Avoid potential overflow from `bits + 7`.
        Size::from_bytes(bits / 8 + ((bits % 8) + 7) / 8)
    }

    #[inline]
    pub fn from_bytes(bytes: impl TryInto<u64>) -> Size {
        Size {
            raw: bytes.try_into().ok().unwrap(),
        }
    }

    #[inline]
    pub fn bytes(self) -> u64 {
        self.raw
    }

    // #[inline]
    // pub fn bytes_usize(self) -> usize {
    //     self.bytes().try_into().unwrap()
    // }

    #[inline]
    pub fn bits(self) -> u64 {
        self.bytes().checked_mul(8).unwrap_or_else(|| {
            panic!(
                "Size::bits: {} bytes in bits doesn't fit in u64",
                self.bytes()
            )
        })
    }

    // #[inline]
    // pub fn bits_usize(self) -> usize {
    //     self.bits().try_into().unwrap()
    // }
    //
    // #[inline]
    // pub fn align_to(self, align: Align) -> Size {
    //     let mask = align.bytes() - 1;
    //     Size::from_bytes((self.bytes() + mask) & !mask)
    // }
    //
    // #[inline]
    // pub fn is_aligned(self, align: Align) -> bool {
    //     let mask = align.bytes() - 1;
    //     self.bytes() & mask == 0
    // }
    //
    // #[inline]
    // pub fn checked_add<C: HasDataLayout>(self, offset: Size, cx: &C) ->
    // Option<Size> {     let dl = cx.data_layout();
    //
    //     let bytes = self.bytes().checked_add(offset.bytes())?;
    //
    //     if bytes < dl.obj_size_bound() {
    //         Some(Size::from_bytes(bytes))
    //     } else {
    //         None
    //     }
    // }
    //
    // #[inline]
    // pub fn checked_mul<C: HasDataLayout>(self, count: u64, cx: &C) ->
    // Option<Size> {     let dl = cx.data_layout();
    //
    //     let bytes = self.bytes().checked_mul(count)?;
    //     if bytes < dl.obj_size_bound() {
    //         Some(Size::from_bytes(bytes))
    //     } else {
    //         None
    //     }
    // }
}

// // Panicking addition, subtraction and multiplication for convenience.
// // Avoid during layout computation, return `LayoutError` instead.
//
// impl Add for Size {
//     type Output = Size;
//     #[inline]
//     fn add(self, other: Size) -> Size {
//         Size::from_bytes(self.bytes().checked_add(other.bytes()).
// unwrap_or_else(|| {             panic!(
//                 "Size::add: {} + {} doesn't fit in u64",
//                 self.bytes(),
//                 other.bytes()
//             )
//         }))
//     }
// }
//
// impl Sub for Size {
//     type Output = Size;
//     #[inline]
//     fn sub(self, other: Size) -> Size {
//         Size::from_bytes(self.bytes().checked_sub(other.bytes()).
// unwrap_or_else(|| {             panic!(
//                 "Size::sub: {} - {} would result in negative size",
//                 self.bytes(),
//                 other.bytes()
//             )
//         }))
//     }
// }
//
// impl Mul<Size> for u64 {
//     type Output = Size;
//     #[inline]
//     fn mul(self, size: Size) -> Size {
//         size * self
//     }
// }
//
// impl Mul<u64> for Size {
//     type Output = Size;
//     #[inline]
//     fn mul(self, count: u64) -> Size {
//         match self.bytes().checked_mul(count) {
//             Some(bytes) => Size::from_bytes(bytes),
//             None => panic!("Size::mul: {} * {} doesn't fit in u64",
// self.bytes(), count),         }
//     }
// }
//
// impl AddAssign for Size {
//     #[inline]
//     fn add_assign(&mut self, other: Size) {
//         *self = *self + other;
//     }
// }
