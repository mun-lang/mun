///! Taken from the
///! [librustc_target](https://github.com/rust-lang/rust/tree/master/src/librustc_target) crate.
use super::Size;

/// Alignment of a type in bytes (always a power of two).
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Align {
    pow2: u8,
}

impl Align {
    pub fn from_bits(bits: u64) -> Result<Align, String> {
        Align::from_bytes(Size::from_bits(bits).bytes())
    }

    pub fn from_bytes(align: u64) -> Result<Align, String> {
        // Treat an alignment of 0 bytes like 1-byte alignment.
        if align == 0 {
            return Ok(Align { pow2: 0 });
        }

        let mut bytes = align;
        let mut pow2: u8 = 0;
        while (bytes & 1) == 0 {
            pow2 += 1;
            bytes >>= 1;
        }
        if bytes != 1 {
            return Err(format!("`{align}` is not a power of 2"));
        }
        if pow2 > 29 {
            return Err(format!("`{align}` is too large"));
        }

        Ok(Align { pow2 })
    }

    // pub fn bytes(self) -> u64 {
    //     1 << self.pow2
    // }
    //
    // pub fn bits(self) -> u64 {
    //     self.bytes() * 8
    // }
    //
    // /// Computes the best alignment possible for the given offset
    // /// (the largest power of two that the offset is a multiple of).
    // ///
    // /// N.B., for an offset of `0`, this happens to return `2^64`.
    // pub fn max_for_offset(offset: Size) -> Align {
    //     Align {
    //         pow2: offset.bytes().trailing_zeros() as u8,
    //     }
    // }
    //
    // /// Lower the alignment, if necessary, such that the given offset
    // /// is aligned to it (the offset is a multiple of the alignment).
    // pub fn restrict_for_offset(self, offset: Size) -> Align {
    //     self.min(Align::max_for_offset(offset))
    // }
}
