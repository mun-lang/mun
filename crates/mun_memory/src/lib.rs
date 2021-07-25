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
pub trait HasRuntimeMemoryLayout {
    /// Returns the size of the memory allocated for this type given a pointer to the memory.
    ///
    /// # Safety
    ///
    /// This function is unsafe because there are no guarantees about the memory being passed in.
    unsafe fn layout(&self, ptr: NonNull<u8>) -> Layout;
}

/// A trait used to obtain a type's memory description.
pub trait HasCompileTimeMemoryLayout {
    /// Returns the memory layout of this type.
    fn layout(&self) -> Layout;

    /// Returns whether the memory is stack-allocated.
    /// TODO: Split this into a different trait.
    fn is_stack_allocated(&self) -> bool;
}

/// A trait that describes the contents of a struct. This trait does *not* define how the memory of
/// the type is laid out. The memory layout of a struct is defined by the [`StructMemoryLayout`]
/// trait.
pub trait StructType<T> {
    /// Returns the name and type of each field in the struct
    fn fields(&self) -> Vec<(&str, T)>;
}

/// A trait that describes the memory layout of a struct. It can be used to obtain the individual
/// fields of a struct in memory.
pub trait StructMemoryLayout {
    /// Returns the offset (in bytes) of each field relative to the start of the struct
    fn offsets(&self) -> &[u16];
}

/// A trait that describes an array e.g. `[T]`. This trait does *not* define how the memory of the
/// trait is laid out. The memory layout of an array is defined by the [`ArrayMemoryLayout`] trait.
pub trait ArrayType<T> {
    /// Returns the type of the elements stored in the array
    fn element_type(&self) -> T;
}

/// A trait that describes the memory layout of an array
pub trait ArrayMemoryLayout {
    type ElementIterator: Iterator<Item = NonNull<u8>>;

    /// Returns the memory to allocate for an array with `n` elements
    fn layout(&self, n: usize) -> Layout;

    /// Initialize memory allocated for an array with `n` elements. This ensures that the memory
    /// allocated for an array is always properly initialized.
    fn init(&self, n: usize, data: &mut [u8]);

    /// Returns the number of elements in the array given a pointer to the memory.
    ///
    /// # Safety
    ///
    /// This function is unsafe because there are no guarantees that the memory passed into this
    /// function via `ptr` actually refers to memory allocated for this trait.
    unsafe fn retrieve_length(&self, ptr: NonNull<u8>) -> usize;

    /// Updates the number of elements in the array given a pointer to the memory.
    ///
    /// # Safety
    ///
    /// This function is unsafe because there are no guarantees that the memory passed into this
    /// function via `ptr` actually refers to memory allocated for this trait.
    unsafe fn store_length(&self, ptr: NonNull<u8>, n: usize);

    /// Returns the maximum number of elements that can be stored in the array pointed to by `ptr`.
    ///
    /// # Safety
    ///
    /// This function is unsafe because there are no guarantees that the memory passed into this
    /// function via `ptr` actually refers to memory allocated for this trait.
    unsafe fn retrieve_capacity(&self, ptr: NonNull<u8>) -> usize;

    /// Returns an iterator over all elements in the array.
    ///
    /// # Safety
    ///
    /// This function is unsafe because there are no guarantees that the memory passed into this
    /// function via `ptr` actually refers to memory allocated for this trait.
    ///
    /// Note: The memory pointed to by `ptr` must remain valid until the `ElementIterator` is
    /// dropped.
    unsafe fn elements(&self, ptr: NonNull<u8>) -> Self::ElementIterator;
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
