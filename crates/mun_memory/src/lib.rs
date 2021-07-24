use std::alloc::Layout;

mod cast;
pub mod diff;
pub mod gc;
pub mod mapping;
mod object;

pub use object::Object;
use std::ptr::NonNull;

pub mod prelude {
    pub use crate::diff::{diff, Diff, FieldDiff, FieldEditKind};
    pub use crate::mapping::{Action, FieldMapping};
}

/// A trait used to obtain a type's description.
pub trait TypeDesc {
    /// Returns the name of this type.
    fn name(&self) -> &str;

    /// Returns the `Guid` of this type.
    fn guid(&self) -> &abi::Guid;
}

/// A trait that enables requesting the memory layout of the data allocated for the type.
pub trait HasDynamicMemoryLayout {
    /// Returns the size of the memory allocated for this type given a pointer to the memory.
    ///
    /// # Safety
    ///
    /// This function is unsafe because there are no guarantees about the memory being passed in.
    unsafe fn layout(&self, ptr: NonNull<u8>) -> Layout;
}

/// A trait used to obtain a type's memory description.
pub trait TypeMemory {
    /// Returns the memory layout of this type.
    fn layout(&self) -> Layout;

    /// Returns whether the memory is stack-allocated.
    /// TODO: Split this into a different trait.
    fn is_stack_allocated(&self) -> bool;
}

/// A trait used to obtain a types fields.
pub trait StructFields<T> {
    /// Returns the name and type of each field in the struct
    fn fields(&self) -> Vec<(&str, T)>;
}

/// A trait used to obtain a type's field memory layout.
pub trait StructFieldLayout {
    /// Returns the offset (in bytes) of each field relative to the start of the struct
    fn offsets(&self) -> &[u16];
}

/// A trait that describes an array e.g. `[T]`
pub trait ArrayType<T> {
    /// Returns the type of the elements stored in the array
    fn element_type(&self) -> T;
}

/// A trait that describes the memory layout of an array
pub trait ArrayMemoryLayout {
    type ElementIterator: Iterator<Item = NonNull<u8>>;

    /// Returns the memory to allocate for an array with `n` elements
    fn layout(&self, n: usize) -> Layout;

    /// Returns the memory layout of of an block of allocated memory for this type.
    ///
    /// # Safety
    ///
    /// This function is unsafe because there are no guarantees about the memory being passed in.
    unsafe fn data_layout(&self, ptr: NonNull<u8>) -> Layout;

    /// Returns the length of the array given a pointer to the memory.
    ///
    /// # Safety
    ///
    /// This function is unsafe because there are no guarantees about the memory being passed in.
    unsafe fn retrieve_length(&self, ptr: NonNull<u8>) -> usize;

    /// Returns an iterator over all elements in the array.
    ///
    /// # Safety
    ///
    /// This function is unsafe because there are no guarantees about the memory being passed in.
    ///
    /// The memory pointed to by `ptr` must remain valid until the `ElementIterator` is dropped.
    unsafe fn elements(&self, ptr: NonNull<u8>) -> Self::ElementIterator;

    /// Updates the length of the array as stored in memory
    ///
    /// # Safety
    ///
    /// This function is unsafe because there are no guarantees about the memory being passed in.
    unsafe fn store_length(&self, ptr: NonNull<u8>, n: usize);
}

/// Marks a type to be a possible composite of different types. Implementers implement the
/// [`CompositeType::group`] method and they specify the derived types for
/// [`CompositeType::ArrayType`] and [`CompositeType::StructType`].
///
/// This provides most algorithm with enough information on composite types. Note that this trait by
/// default does not define any trait requirements for the `ArrayType` or `StructType`. However, a
/// specific consumer of this trait might require more strict constraints for these sub-types. For
/// instance when allocating structs only the memory size and alignment of a struct is required to
/// be known and the exact composition doesn't matter. However, when an algorithm needs to iterate
/// the individual fields of the struct (like when mapping every field) this is a requirement and
/// a more specific constraint can be applied to the `StructType` sub-type.
pub trait CompositeType {
    type ArrayType;
    type StructType;

    /// Returns the group to which this type belongs. This indicates how the memory of an object
    /// should be interpreted.
    fn group(&self) -> CompositeTypeKind<'_, Self::ArrayType, Self::StructType>;

    /// If this type represents an struct, returns an object that can be used to query information
    /// regarding its contents.
    fn as_struct(&self) -> Option<&Self::StructType> {
        match self.group() {
            CompositeTypeKind::Struct(s) => Some(s),
            _ => None,
        }
    }

    /// Returns true if this type represents a struct type
    fn is_struct(&self) -> bool {
        matches!(self.group(), CompositeTypeKind::Struct(_))
    }

    /// If this type represents an array, returns an object that can be used to query information
    /// regarding its contents.
    fn as_array(&self) -> Option<&Self::ArrayType> {
        match self.group() {
            CompositeTypeKind::Array(a) => Some(a),
            _ => None,
        }
    }

    /// Returns true if this type represents a array type
    fn is_array(&self) -> bool {
        matches!(self.group(), CompositeTypeKind::Array(_))
    }

    /// Returns true if this type represents a primitive type
    fn is_primitive(&self) -> bool {
        matches!(self.group(), CompositeTypeKind::Primitive)
    }
}

/// Describes which kind of composite type a specific [`CompositeType`] is.
#[derive(Copy, Clone, Debug)]
pub enum CompositeTypeKind<'t, ArrayType, StructType> {
    Primitive,
    Struct(&'t StructType),
    Array(&'t ArrayType),
}
