//! Taken from the
//! [librustc_target](https://github.com/rust-lang/rust/tree/master/src/librustc_target) crate.

// use crate::abi::{AbiAndPrefAlign, Align, HasDataLayout, Size};

/// Integers, also used for enum discriminants.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Integer {
    I8,
    I16,
    I32,
    I64,
    I128,
}

impl Integer {
    // pub fn size(self) -> Size {
    //     match self {
    //         Integer::I8 => Size::from_bytes(1),
    //         Integer::I16 => Size::from_bytes(2),
    //         Integer::I32 => Size::from_bytes(4),
    //         Integer::I64 => Size::from_bytes(8),
    //         Integer::I128 => Size::from_bytes(16),
    //     }
    // }
    //
    // pub fn align<C: HasDataLayout>(self, cx: &C) -> AbiAndPrefAlign {
    //     let dl = cx.data_layout();
    //
    //     match self {
    //         Integer::I8 => dl.i8_align,
    //         Integer::I16 => dl.i16_align,
    //         Integer::I32 => dl.i32_align,
    //         Integer::I64 => dl.i64_align,
    //         Integer::I128 => dl.i128_align,
    //     }
    // }
    //
    // /// Finds the smallest Integer type which can represent the signed value.
    // pub fn fit_signed(x: i128) -> Integer {
    //     #[allow(clippy::match_overlapping_arm)]
    //     match x {
    //         -0x0000_0000_0000_0080..=0x0000_0000_0000_007f => Integer::I8,
    //         -0x0000_0000_0000_8000..=0x0000_0000_0000_7fff => Integer::I16,
    //         -0x0000_0000_8000_0000..=0x0000_0000_7fff_ffff => Integer::I32,
    //         -0x8000_0000_0000_0000..=0x7fff_ffff_ffff_ffff => Integer::I64,
    //         _ => Integer::I128,
    //     }
    // }
    //
    // /// Finds the smallest Integer type which can represent the unsigned value.
    // pub fn fit_unsigned(x: u128) -> Integer {
    //     #[allow(clippy::match_overlapping_arm)]
    //     match x {
    //         0..=0x0000_0000_0000_00ff => Integer::I8,
    //         0..=0x0000_0000_0000_ffff => Integer::I16,
    //         0..=0x0000_0000_ffff_ffff => Integer::I32,
    //         0..=0xffff_ffff_ffff_ffff => Integer::I64,
    //         _ => Integer::I128,
    //     }
    // }
    //
    // /// Finds the smallest integer with the given alignment.
    // pub fn for_align<C: HasDataLayout>(cx: &C, wanted: Align) -> Option<Integer>
    // {     let dl = cx.data_layout();
    //
    //     for &candidate in &[
    //         Integer::I8,
    //         Integer::I16,
    //         Integer::I32,
    //         Integer::I64,
    //         Integer::I128,
    //     ] {
    //         if wanted == candidate.align(dl).abi && wanted.bytes() ==
    // candidate.size().bytes() {             return Some(candidate);
    //         }
    //     }
    //     None
    // }
    //
    // /// Find the largest integer with the given alignment or less.
    // pub fn approximate_align<C: HasDataLayout>(cx: &C, wanted: Align) -> Integer
    // {     let dl = cx.data_layout();
    //
    //     for &candidate in &[Integer::I128, Integer::I64, Integer::I32,
    // Integer::I16] {         if wanted >= candidate.align(dl).abi &&
    // wanted.bytes() >= candidate.size().bytes() {             return
    // candidate;         }
    //     }
    //     Integer::I8
    // }
}
