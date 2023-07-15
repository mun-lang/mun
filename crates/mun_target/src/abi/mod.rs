//! Taken from the
//! [librustc_target](https://github.com/rust-lang/rust/tree/master/src/librustc_target) crate.

mod align;
mod integer;
mod size;

use crate::spec::Target;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

pub use align::Align;
pub use integer::Integer;
pub use size::Size;

/// Parsed [Data layout](http://llvm.org/docs/LangRef.html#data-layout) for a target, which contains
/// everything needed to compute layouts.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct TargetDataLayout {
    pub endian: Endian,
    pub i1_align: AbiAndPrefAlign,
    pub i8_align: AbiAndPrefAlign,
    pub i16_align: AbiAndPrefAlign,
    pub i32_align: AbiAndPrefAlign,
    pub i64_align: AbiAndPrefAlign,
    pub i128_align: AbiAndPrefAlign,
    pub f32_align: AbiAndPrefAlign,
    pub f64_align: AbiAndPrefAlign,
    pub pointer_size: Size,
    pub pointer_align: AbiAndPrefAlign,
    pub aggregate_align: AbiAndPrefAlign,

    /// Alignments for vector types.
    pub vector_align: Vec<(Size, AbiAndPrefAlign)>,

    pub instruction_address_space: u32,
}

impl Default for TargetDataLayout {
    /// Creates an instance of `TargetDataLayout`.
    fn default() -> TargetDataLayout {
        let align = |bits| Align::from_bits(bits).unwrap();
        TargetDataLayout {
            endian: Endian::Big,
            i1_align: AbiAndPrefAlign::new(align(8)),
            i8_align: AbiAndPrefAlign::new(align(8)),
            i16_align: AbiAndPrefAlign::new(align(16)),
            i32_align: AbiAndPrefAlign::new(align(32)),
            i64_align: AbiAndPrefAlign {
                abi: align(32),
                pref: align(64),
            },
            i128_align: AbiAndPrefAlign {
                abi: align(32),
                pref: align(64),
            },
            f32_align: AbiAndPrefAlign::new(align(32)),
            f64_align: AbiAndPrefAlign::new(align(64)),
            pointer_size: Size::from_bits(64),
            pointer_align: AbiAndPrefAlign::new(align(64)),
            aggregate_align: AbiAndPrefAlign {
                abi: align(0),
                pref: align(64),
            },
            vector_align: vec![
                (Size::from_bits(64), AbiAndPrefAlign::new(align(64))),
                (Size::from_bits(128), AbiAndPrefAlign::new(align(128))),
            ],
            instruction_address_space: 0,
        }
    }
}

impl TargetDataLayout {
    pub fn parse(target: &Target) -> Result<TargetDataLayout, String> {
        // Parse an address space index from a string.
        let parse_address_space = |s: &str, cause: &str| {
            s.parse::<u32>().map_err(|err| {
                format!("invalid address space `{s}` for `{cause}` in \"data-layout\": {err}")
            })
        };

        // Parse a bit count from a string.
        let parse_bits = |s: &str, kind: &str, cause: &str| {
            s.parse::<u64>().map_err(|err| {
                format!("invalid {kind} `{s}` for `{cause}` in \"data-layout\": {err}")
            })
        };

        // Parse a size string.
        let size = |s: &str, cause: &str| parse_bits(s, "size", cause).map(Size::from_bits);

        // Parse an alignment string.
        let align = |s: &[&str], cause: &str| {
            if s.is_empty() {
                return Err(format!(
                    "missing alignment for `{cause}` in \"data-layout\""
                ));
            }
            let align_from_bits = |bits| {
                Align::from_bits(bits).map_err(|err| {
                    format!("invalid alignment for `{cause}` in \"data-layout\": {err}")
                })
            };
            let abi = parse_bits(s[0], "alignment", cause)?;
            let pref = s
                .get(1)
                .map_or(Ok(abi), |pref| parse_bits(pref, "alignment", cause))?;
            Ok(AbiAndPrefAlign {
                abi: align_from_bits(abi)?,
                pref: align_from_bits(pref)?,
            })
        };

        let mut dl = TargetDataLayout::default();
        let mut i128_align_src = 64;
        for spec in target.data_layout.split('-') {
            let spec_parts = spec.split(':').collect::<Vec<_>>();

            match &*spec_parts {
                ["e"] => dl.endian = Endian::Little,
                ["E"] => dl.endian = Endian::Big,
                [p] if p.starts_with('P') => {
                    dl.instruction_address_space = parse_address_space(&p[1..], "P")?
                }
                ["a", ref a @ ..] => dl.aggregate_align = align(a, "a")?,
                ["f32", ref a @ ..] => dl.f32_align = align(a, "f32")?,
                ["f64", ref a @ ..] => dl.f64_align = align(a, "f64")?,
                [p @ "p", s, ref a @ ..] | [p @ "p0", s, ref a @ ..] => {
                    dl.pointer_size = size(s, p)?;
                    dl.pointer_align = align(a, p)?;
                }
                [s, ref a @ ..] if s.starts_with('i') => {
                    let bits = match s[1..].parse::<u64>() {
                        Ok(bits) => bits,
                        Err(_) => {
                            size(&s[1..], "i")?; // For the user error.
                            continue;
                        }
                    };
                    let a = align(a, s)?;
                    match bits {
                        1 => dl.i1_align = a,
                        8 => dl.i8_align = a,
                        16 => dl.i16_align = a,
                        32 => dl.i32_align = a,
                        64 => dl.i64_align = a,
                        _ => {}
                    }
                    if bits >= i128_align_src && bits <= 128 {
                        // Default alignment for i128 is decided by taking the alignment of
                        // largest-sized i{64..=128}.
                        i128_align_src = bits;
                        dl.i128_align = a;
                    }
                }
                [s, ref a @ ..] if s.starts_with('v') => {
                    let v_size = size(&s[1..], "v")?;
                    let a = align(a, s)?;
                    if let Some(v) = dl.vector_align.iter_mut().find(|v| v.0 == v_size) {
                        v.1 = a;
                        continue;
                    }
                    // No existing entry, add a new one.
                    dl.vector_align.push((v_size, a));
                }
                _ => {} // Ignore everything else.
            }
        }

        // Perform consistency checks against the Target information.
        if dl.endian != target.options.endian {
            return Err(format!(
                "inconsistent target specification: \"data-layout\" claims \
                                architecture is {}-endian, while \"target-endian\" is `{}`",
                dl.endian, target.options.endian
            ));
        }

        let target_pointer_width: u64 = target.pointer_width.into();
        if dl.pointer_size.bits() != target_pointer_width {
            return Err(format!(
                "inconsistent target specification: \"data-layout\" claims \
                                pointers are {}-bit, while \"target-pointer-width\" is `{}`",
                dl.pointer_size.bits(),
                target.pointer_width
            ));
        }

        Ok(dl)
    }

    // /// Returns exclusive upper bound on object size.
    // ///
    // /// The theoretical maximum object size is defined as the maximum positive `isize` value.
    // /// This ensures that the `offset` semantics remain well-defined by allowing it to correctly
    // /// index every address within an object along with one byte past the end, along with allowing
    // /// `isize` to store the difference between any two pointers into an object.
    // ///
    // /// The upper bound on 64-bit currently needs to be lower because LLVM uses a 64-bit integer
    // /// to represent object size in bits. It would need to be 1 << 61 to account for this, but is
    // /// currently conservatively bounded to 1 << 47 as that is enough to cover the current usable
    // /// address space on 64-bit ARMv8 and x86_64.
    // pub fn obj_size_bound(&self) -> u64 {
    //     match self.pointer_size.bits() {
    //         16 => 1 << 15,
    //         32 => 1 << 31,
    //         64 => 1 << 47,
    //         bits => panic!("obj_size_bound: unknown pointer bit size {}", bits),
    //     }
    // }

    pub fn ptr_sized_integer(&self) -> Integer {
        use Integer::*;
        match self.pointer_size.bits() {
            16 => I16,
            32 => I32,
            64 => I64,
            bits => panic!("ptr_sized_integer: unknown pointer bit size {bits}"),
        }
    }

    // pub fn vector_align(&self, vec_size: Size) -> AbiAndPrefAlign {
    //     for &(size, align) in &self.vector_align {
    //         if size == vec_size {
    //             return align;
    //         }
    //     }
    //     // Default to natural alignment, which is what LLVM does.
    //     // That is, use the size, rounded up to a power of 2.
    //     AbiAndPrefAlign::new(Align::from_bytes(vec_size.bytes().next_power_of_two()).unwrap())
    // }
}

// pub trait HasDataLayout {
//     fn data_layout(&self) -> &TargetDataLayout;
// }
//
// impl HasDataLayout for TargetDataLayout {
//     fn data_layout(&self) -> &TargetDataLayout {
//         self
//     }
// }

/// Endianness of the target
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum Endian {
    Little,
    Big,
}

impl Endian {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Little => "little",
            Self::Big => "big",
        }
    }
}

impl Display for Endian {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for Endian {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "little" => Ok(Self::Little),
            "big" => Ok(Self::Big),
            _ => Err(format!(r#"unknown endian: "{s}""#)),
        }
    }
}

/// A pair of alignments, ABI-mandated and preferred.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct AbiAndPrefAlign {
    pub abi: Align,
    pub pref: Align,
}

impl AbiAndPrefAlign {
    pub fn new(align: Align) -> AbiAndPrefAlign {
        AbiAndPrefAlign {
            abi: align,
            pref: align,
        }
    }

    // pub fn min(self, other: AbiAndPrefAlign) -> AbiAndPrefAlign {
    //     AbiAndPrefAlign {
    //         abi: self.abi.min(other.abi),
    //         pref: self.pref.min(other.pref),
    //     }
    // }
    //
    // pub fn max(self, other: AbiAndPrefAlign) -> AbiAndPrefAlign {
    //     AbiAndPrefAlign {
    //         abi: self.abi.max(other.abi),
    //         pref: self.pref.max(other.pref),
    //     }
    // }
}
