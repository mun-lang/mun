use super::InferenceResultBuilder;
use crate::{HirDatabase, Ty, TypeCtor};

impl<'a, D: HirDatabase> InferenceResultBuilder<'a, D> {
    /// Unify two types, but may coerce the first one to the second using implicit coercion rules if
    /// needed.
    pub(super) fn coerce(&mut self, from_ty: &Ty, to_ty: &Ty) -> bool {
        self.coerce_inner(from_ty.clone(), &to_ty)
    }

    /// Merge two types from different branches, with possible implicit coerce.
    pub(super) fn coerce_merge_branch(&mut self, ty1: &Ty, ty2: &Ty) -> Option<Ty> {
        if self.coerce(ty1, ty2) {
            Some(ty2.clone())
        } else if self.coerce(ty2, ty1) {
            Some(ty1.clone())
        } else {
            None
        }
    }

    fn coerce_inner(&mut self, from_ty: Ty, to_ty: &Ty) -> bool {
        match (&from_ty, to_ty) {
            (ty_app!(TypeCtor::Never), ..) => return true,
            _ => {
                if self.unify_inner_trivial(&from_ty, &to_ty) {
                    return true;
                }
            }
        };

        self.unify(&from_ty, to_ty)
    }
}
