use super::Ty;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PointerTy {
    pub pointee_ty: Ty,
    pub mutability: Mutability,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Mutability {
    Mut,
    Shared,
}

impl Mutability {
    pub fn from_mutable(mutable: bool) -> Mutability {
        if mutable {
            Mutability::Mut
        } else {
            Mutability::Shared
        }
    }

    pub fn as_keyword_for_ptr(self) -> &'static str {
        match self {
            Mutability::Shared => "const ",
            Mutability::Mut => "mut ",
        }
    }

    pub fn is_const(self) -> bool {
        matches!(self, Mutability::Shared)
    }
}
