use crate::builtin_type::{BuiltinFloat, BuiltinInt, FloatBitness, IntBitness, Signedness};
use mun_target::abi;
use mun_target::abi::Integer;
use std::fmt::{self};
use std::{i128, i16, i32, i64, i8, u128, u16, u32, u64, u8};

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct IntTy {
    pub signedness: Signedness,
    pub bitness: IntBitness,
}

impl fmt::Debug for IntTy {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl fmt::Display for IntTy {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl IntTy {
    pub fn int() -> IntTy {
        IntTy {
            signedness: Signedness::Signed,
            bitness: IntBitness::Undefined,
        }
    }

    pub fn isize() -> IntTy {
        IntTy {
            signedness: Signedness::Signed,
            bitness: IntBitness::Xsize,
        }
    }

    pub fn i8() -> IntTy {
        IntTy {
            signedness: Signedness::Signed,
            bitness: IntBitness::X8,
        }
    }

    pub fn i16() -> IntTy {
        IntTy {
            signedness: Signedness::Signed,
            bitness: IntBitness::X16,
        }
    }

    pub fn i32() -> IntTy {
        IntTy {
            signedness: Signedness::Signed,
            bitness: IntBitness::X32,
        }
    }

    pub fn i64() -> IntTy {
        IntTy {
            signedness: Signedness::Signed,
            bitness: IntBitness::X64,
        }
    }

    pub fn i128() -> IntTy {
        IntTy {
            signedness: Signedness::Signed,
            bitness: IntBitness::X128,
        }
    }

    pub fn usize() -> IntTy {
        IntTy {
            signedness: Signedness::Unsigned,
            bitness: IntBitness::Xsize,
        }
    }

    pub fn u8() -> IntTy {
        IntTy {
            signedness: Signedness::Unsigned,
            bitness: IntBitness::X8,
        }
    }

    pub fn u16() -> IntTy {
        IntTy {
            signedness: Signedness::Unsigned,
            bitness: IntBitness::X16,
        }
    }

    pub fn u32() -> IntTy {
        IntTy {
            signedness: Signedness::Unsigned,
            bitness: IntBitness::X32,
        }
    }

    pub fn u64() -> IntTy {
        IntTy {
            signedness: Signedness::Unsigned,
            bitness: IntBitness::X64,
        }
    }

    pub fn u128() -> IntTy {
        IntTy {
            signedness: Signedness::Unsigned,
            bitness: IntBitness::X128,
        }
    }

    pub fn as_str(self) -> &'static str {
        match (self.signedness, self.bitness) {
            (Signedness::Signed, IntBitness::Undefined) => "int",
            (Signedness::Signed, IntBitness::Xsize) => "isize",
            (Signedness::Signed, IntBitness::X8) => "i8",
            (Signedness::Signed, IntBitness::X16) => "i16",
            (Signedness::Signed, IntBitness::X32) => "i32",
            (Signedness::Signed, IntBitness::X64) => "i64",
            (Signedness::Signed, IntBitness::X128) => "i128",
            (Signedness::Unsigned, IntBitness::Undefined) => "uint",
            (Signedness::Unsigned, IntBitness::Xsize) => "usize",
            (Signedness::Unsigned, IntBitness::X8) => "u8",
            (Signedness::Unsigned, IntBitness::X16) => "u16",
            (Signedness::Unsigned, IntBitness::X32) => "u32",
            (Signedness::Unsigned, IntBitness::X64) => "u64",
            (Signedness::Unsigned, IntBitness::X128) => "u128",
        }
    }

    /// Returns the maximum positive number that this instance can contain.
    pub fn max(self) -> u128 {
        match self.signedness {
            Signedness::Signed => match self.bitness {
                IntBitness::X8 => i8::MAX as u128,
                IntBitness::X16 => i16::MAX as u128,
                IntBitness::X32 => i32::MAX as u128,
                IntBitness::X64 => i64::MAX as u128,
                IntBitness::X128 => i128::MAX as u128,
                _ => unreachable!("cannot determine max size of variable bitness"),
            },
            Signedness::Unsigned => match self.bitness {
                IntBitness::X8 => u8::MAX as u128,
                IntBitness::X16 => u16::MAX as u128,
                IntBitness::X32 => u32::MAX as u128,
                IntBitness::X64 => u64::MAX as u128,
                IntBitness::X128 => u128::MAX,
                _ => unreachable!("cannot determine max size of variable bitness"),
            },
        }
    }
}

impl From<abi::Integer> for IntBitness {
    fn from(i: Integer) -> Self {
        match i {
            Integer::I8 => IntBitness::X8,
            Integer::I16 => IntBitness::X16,
            Integer::I32 => IntBitness::X32,
            Integer::I64 => IntBitness::X64,
            Integer::I128 => IntBitness::X128,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct FloatTy {
    pub bitness: FloatBitness,
}

impl fmt::Debug for FloatTy {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl fmt::Display for FloatTy {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FloatTy {
    pub fn float() -> FloatTy {
        FloatTy {
            bitness: FloatBitness::Undefined,
        }
    }

    pub fn f32() -> FloatTy {
        FloatTy {
            bitness: FloatBitness::X32,
        }
    }

    pub fn f64() -> FloatTy {
        FloatTy {
            bitness: FloatBitness::X64,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self.bitness {
            FloatBitness::X32 => "f32",
            FloatBitness::X64 => "f64",
            FloatBitness::Undefined => "float",
        }
    }
}

impl From<BuiltinInt> for IntTy {
    fn from(t: BuiltinInt) -> Self {
        IntTy {
            signedness: t.signedness,
            bitness: t.bitness,
        }
    }
}

impl From<BuiltinFloat> for FloatTy {
    fn from(t: BuiltinFloat) -> Self {
        FloatTy { bitness: t.bitness }
    }
}
