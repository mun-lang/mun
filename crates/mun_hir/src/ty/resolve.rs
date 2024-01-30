use mun_target::{abi, abi::TargetDataLayout};

use super::primitives::IntTy;
use crate::{FloatBitness, FloatTy, IntBitness};

pub trait ResolveBitness {
    /// Resolves any variable bitness into concrete values.
    fn resolve(&self, target: &abi::TargetDataLayout) -> Self;
}

impl ResolveBitness for IntBitness {
    fn resolve(&self, data_layout: &abi::TargetDataLayout) -> IntBitness {
        match self {
            IntBitness::Xsize => data_layout.ptr_sized_integer().into(),
            IntBitness::X8
            | IntBitness::X16
            | IntBitness::X32
            | IntBitness::X64
            | IntBitness::X128 => *self,
        }
    }
}

impl ResolveBitness for FloatBitness {
    fn resolve(&self, _data_layout: &abi::TargetDataLayout) -> FloatBitness {
        match self {
            FloatBitness::X32 | FloatBitness::X64 => *self,
        }
    }
}

impl ResolveBitness for IntTy {
    fn resolve(&self, target: &TargetDataLayout) -> Self {
        IntTy {
            bitness: self.bitness.resolve(target),
            signedness: self.signedness,
        }
    }
}

impl ResolveBitness for FloatTy {
    fn resolve(&self, target: &TargetDataLayout) -> Self {
        FloatTy {
            bitness: self.bitness.resolve(target),
        }
    }
}
