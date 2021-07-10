use std::alloc::{Layout};

mod cast;
pub mod diff;
pub mod gc;
pub mod mapping;
mod object;

pub use object::Object;

pub mod prelude {
    pub use crate::diff::{diff, Diff, FieldDiff, FieldEditKind};
    pub use crate::mapping::{Action, FieldMapping};
}

/// A trait used to obtain a type's description.
pub trait TypeDesc: Send + Sync {
    /// Returns the name of this type.
    fn name(&self) -> &str;

    /// Returns the `Guid` of this type.
    fn guid(&self) -> &abi::Guid;
}

/// A trait used to obtain a type's memory description.
pub trait TypeMemory: Send + Sync {
    /// Returns the memory layout of this type.
    fn layout(&self) -> Layout;

    /// Returns whether the memory is stack-allocated.
    fn is_stack_allocated(&self) -> bool;
}

/// A trait used to obtain a type's fields.
pub trait StructType<T> {
    /// Returns the type's fields.
    fn fields(&self) -> Vec<(&str, T)>;

    /// Returns the type's fields' offsets.
    fn offsets(&self) -> &[u16];
}

/// A trait that describes an array e.g. [T]
pub trait ArrayType<T> {
    /// Returns the type of the elements stored in the array
    fn element_type(&self) -> T;
}

#[derive(Copy, Clone, Debug)]
pub enum TypeGroup<'t, ArrayType, StructType> {
    Primitive,
    Struct(&'t StructType),
    Array(&'t ArrayType),
}

pub trait TypeComposition: Sized {
    type ArrayType: ArrayType<Self>;
    type StructType: StructType<Self>;

    /// Returns the group to which this type belongs. This indicates how the memory of an object
    /// should be interpreted.
    fn group(&self) -> TypeGroup<'_, Self::ArrayType, Self::StructType>;

    /// If this type represents an struct, returns an object that can be used to query information
    /// regarding its contents.
    fn as_struct(&self) -> Option<&Self::StructType> {
        match self.group() {
            TypeGroup::Struct(s) => Some(s),
            _ => None
        }
    }

    /// Returns true if this type represents a struct type
    fn is_struct(&self) -> bool {
        matches!(self.group(), TypeGroup::Struct(_))
    }

    /// If this type represents an array, returns an object that can be used to query information
    /// regarding its contents.
    fn as_array(&self) -> Option<&Self::ArrayType> {
        match self.group() {
            TypeGroup::Array(a) => Some(a),
            _ => None
        }
    }

    /// Returns true if this type represents a array type
    fn is_array(&self) -> bool {
        matches!(self.group(), TypeGroup::Array(_))
    }

    /// Returns true if this type represents a primitive type
    fn is_primitive(&self) -> bool {
        matches!(self.group(), TypeGroup::Primitive)
    }
}
