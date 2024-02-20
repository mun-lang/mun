use std::fmt;

use crate::name::{name, Name};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Signedness {
    Signed,
    Unsigned,
}

impl Signedness {
    pub fn is_signed(self) -> bool {
        self == Signedness::Signed
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum IntBitness {
    Xsize,
    X8,
    X16,
    X32,
    X64,
    X128,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum FloatBitness {
    X32,
    X64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PrimitiveInt {
    pub signedness: Signedness,
    pub bitness: IntBitness,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PrimitiveFloat {
    pub bitness: FloatBitness,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrimitiveType {
    Float(PrimitiveFloat),
    Int(PrimitiveInt),
    Bool,
}

impl PrimitiveType {
    #[rustfmt::skip]
    pub const ALL: &'static [(Name, PrimitiveType)] = &[
        (name![bool], PrimitiveType::Bool),

        (name![isize], PrimitiveType::Int(PrimitiveInt::ISIZE)),
        (name![i8], PrimitiveType::Int(PrimitiveInt::I8)),
        (name![i16], PrimitiveType::Int(PrimitiveInt::I16)),
        (name![i32], PrimitiveType::Int(PrimitiveInt::I32)),
        (name![i64], PrimitiveType::Int(PrimitiveInt::I64)),
        (name![i128], PrimitiveType::Int(PrimitiveInt::I128)),

        (name![usize], PrimitiveType::Int(PrimitiveInt::USIZE)),
        (name![u8], PrimitiveType::Int(PrimitiveInt::U8)),
        (name![u16], PrimitiveType::Int(PrimitiveInt::U16)),
        (name![u32], PrimitiveType::Int(PrimitiveInt::U32)),
        (name![u64], PrimitiveType::Int(PrimitiveInt::U64)),
        (name![u128], PrimitiveType::Int(PrimitiveInt::U128)),

        (name![f32], PrimitiveType::Float(PrimitiveFloat::F32)),
        (name![f64], PrimitiveType::Float(PrimitiveFloat::F64)),
    ];
}

impl fmt::Display for PrimitiveType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let type_name = match self {
            PrimitiveType::Bool => "bool",
            PrimitiveType::Int(PrimitiveInt {
                signedness,
                bitness,
            }) => match (signedness, bitness) {
                (Signedness::Signed, IntBitness::Xsize) => "isize",
                (Signedness::Signed, IntBitness::X8) => "i8",
                (Signedness::Signed, IntBitness::X16) => "i16",
                (Signedness::Signed, IntBitness::X32) => "i32",
                (Signedness::Signed, IntBitness::X64) => "i64",
                (Signedness::Signed, IntBitness::X128) => "i128",

                (Signedness::Unsigned, IntBitness::Xsize) => "usize",
                (Signedness::Unsigned, IntBitness::X8) => "u8",
                (Signedness::Unsigned, IntBitness::X16) => "u16",
                (Signedness::Unsigned, IntBitness::X32) => "u32",
                (Signedness::Unsigned, IntBitness::X64) => "u64",
                (Signedness::Unsigned, IntBitness::X128) => "u128",
            },
            PrimitiveType::Float(PrimitiveFloat { bitness }) => match bitness {
                FloatBitness::X32 => "f32",
                FloatBitness::X64 => "f64",
            },
        };
        f.write_str(type_name)
    }
}

#[rustfmt::skip]
impl PrimitiveInt {
    pub const ISIZE: PrimitiveInt = PrimitiveInt { signedness: Signedness::Signed, bitness: IntBitness::Xsize       };
    pub const I8   : PrimitiveInt = PrimitiveInt { signedness: Signedness::Signed, bitness: IntBitness::X8          };
    pub const I16  : PrimitiveInt = PrimitiveInt { signedness: Signedness::Signed, bitness: IntBitness::X16         };
    pub const I32  : PrimitiveInt = PrimitiveInt { signedness: Signedness::Signed, bitness: IntBitness::X32         };
    pub const I64  : PrimitiveInt = PrimitiveInt { signedness: Signedness::Signed, bitness: IntBitness::X64         };
    pub const I128 : PrimitiveInt = PrimitiveInt { signedness: Signedness::Signed, bitness: IntBitness::X128        };

    pub const USIZE: PrimitiveInt = PrimitiveInt { signedness: Signedness::Unsigned, bitness: IntBitness::Xsize     };
    pub const U8   : PrimitiveInt = PrimitiveInt { signedness: Signedness::Unsigned, bitness: IntBitness::X8        };
    pub const U16  : PrimitiveInt = PrimitiveInt { signedness: Signedness::Unsigned, bitness: IntBitness::X16       };
    pub const U32  : PrimitiveInt = PrimitiveInt { signedness: Signedness::Unsigned, bitness: IntBitness::X32       };
    pub const U64  : PrimitiveInt = PrimitiveInt { signedness: Signedness::Unsigned, bitness: IntBitness::X64       };
    pub const U128 : PrimitiveInt = PrimitiveInt { signedness: Signedness::Unsigned, bitness: IntBitness::X128      };


    pub fn from_suffix(suffix: &str) -> Option<PrimitiveInt> {
        let res = match suffix {
            "isize" => Self::ISIZE,
            "i8"    => Self::I8,
            "i16"   => Self::I16,
            "i32"   => Self::I32,
            "i64"   => Self::I64,
            "i128"  => Self::I128,

            "usize" => Self::USIZE,
            "u8"    => Self::U8,
            "u16"   => Self::U16,
            "u32"   => Self::U32,
            "u64"   => Self::U64,
            "u128"  => Self::U128,

            _ => return None,
        };
        Some(res)
    }
}

#[rustfmt::skip]
impl PrimitiveFloat {
    pub const F32: PrimitiveFloat = PrimitiveFloat { bitness: FloatBitness::X32 };
    pub const F64: PrimitiveFloat = PrimitiveFloat { bitness: FloatBitness::X64 };

    pub fn from_suffix(suffix: &str) -> Option<PrimitiveFloat> {
        let res = match suffix {
            "f32" => PrimitiveFloat::F32,
            "f64" => PrimitiveFloat::F64,
            _ => return None,
        };
        Some(res)
    }
}
