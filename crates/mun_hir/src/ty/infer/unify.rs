use std::borrow::Cow;

use crate::{ty::infer::InferenceResultBuilder, Ty};

impl InferenceResultBuilder<'_> {
    /// If `ty` is a type variable, and it has been instantiated, then return
    /// the instantiated type; otherwise returns `ty`.
    pub(crate) fn replace_if_possible<'b>(&mut self, ty: &'b Ty) -> Cow<'b, Ty> {
        self.type_variables.replace_if_possible(self.db, ty)
    }

    /// Unifies the two types. If one or more type variables are involved
    /// instantiate or equate the variables with each other.
    pub(crate) fn unify(&mut self, a: &Ty, b: &Ty) -> bool {
        self.type_variables.unify(self.db, a, b)
    }

    /// Resolves the type as far as currently possible, replacing type variables
    /// by their known types. All types returned by the `infer_*` functions
    /// should be resolved as far as possible, i.e. contain no type
    /// variables with known type.
    pub(crate) fn resolve_ty_as_far_as_possible(&mut self, ty: Ty) -> Ty {
        self.type_variables.resolve_ty_as_far_as_possible(ty)
    }
}
