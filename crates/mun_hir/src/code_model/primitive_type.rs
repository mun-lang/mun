use crate::{name::AsName, ty::lower::type_for_primitive, Name, Ty};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PrimitiveType {
    pub(crate) inner: crate::primitive_type::PrimitiveType,
}

impl PrimitiveType {
    /// Returns the type of the primitive
    pub fn ty(self, _db: &dyn crate::HirDatabase) -> Ty {
        type_for_primitive(self)
    }

    /// Returns the name of the primitive
    pub fn name(self) -> Name {
        self.inner.as_name()
    }
}

impl From<crate::primitive_type::PrimitiveType> for PrimitiveType {
    fn from(inner: crate::primitive_type::PrimitiveType) -> Self {
        PrimitiveType { inner }
    }
}
