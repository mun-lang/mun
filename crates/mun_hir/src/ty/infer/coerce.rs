use super::InferenceResultBuilder;
use crate::{ty::TyKind, Ty};

impl<'a> InferenceResultBuilder<'a> {
    /// Unify two types, but may coerce the first one to the second using
    /// implicit coercion rules if needed.
    pub(super) fn coerce(&mut self, from_ty: &Ty, to_ty: &Ty) -> bool {
        let from_ty = self.replace_if_possible(from_ty).into_owned();
        let to_ty = self.replace_if_possible(to_ty);
        self.coerce_inner(from_ty, &to_ty)
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
        match (from_ty.interned(), to_ty.interned()) {
            (TyKind::Never, ..) => return true,
            _ => {
                if self.type_variables.unify_inner_trivial(&from_ty, to_ty) {
                    return true;
                }
            }
        };

        self.unify(&from_ty, to_ty)
    }
}
