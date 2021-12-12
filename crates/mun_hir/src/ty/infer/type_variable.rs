use crate::{
    ty::infer::InferTy,
    ty::{TyKind, TypeWalk},
    Substitution, Ty,
};
use ena::unify::{InPlaceUnificationTable, NoError, UnifyKey, UnifyValue};
use std::{borrow::Cow, fmt};

/// The ID of a type variable.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct TypeVarId(pub(crate) u32);

impl fmt::Display for TypeVarId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "'{}", self.0)
    }
}

impl UnifyKey for TypeVarId {
    type Value = TypeVarValue;

    fn index(&self) -> u32 {
        self.0
    }

    fn from_index(i: u32) -> Self {
        TypeVarId(i)
    }

    fn tag() -> &'static str {
        "TypeVarId"
    }
}

/// The value of a type variable: either we already know the type, or we don't
/// know it yet.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum TypeVarValue {
    Known(Ty),
    Unknown,
}

impl TypeVarValue {
    fn known(&self) -> Option<&Ty> {
        match self {
            TypeVarValue::Known(ty) => Some(ty),
            TypeVarValue::Unknown => None,
        }
    }

    fn is_unknown(&self) -> bool {
        match self {
            TypeVarValue::Known(_) => false,
            TypeVarValue::Unknown => true,
        }
    }
}

impl UnifyValue for TypeVarValue {
    type Error = NoError;

    fn unify_values(value1: &Self, value2: &Self) -> Result<Self, NoError> {
        match (value1, value2) {
            // We should never equate two type variables, both of which have
            // known types. Instead, we recursively equate those types.
            (TypeVarValue::Known(t1), TypeVarValue::Known(t2)) => panic!(
                "equating two type variables, both of which have known types: {:?} and {:?}",
                t1, t2
            ),

            // If one side is known, prefer that one.
            (TypeVarValue::Known(..), TypeVarValue::Unknown) => Ok(value1.clone()),
            (TypeVarValue::Unknown, TypeVarValue::Known(..)) => Ok(value2.clone()),

            (TypeVarValue::Unknown, TypeVarValue::Unknown) => Ok(TypeVarValue::Unknown),
        }
    }
}

#[derive(Default)]
pub struct TypeVariableTable {
    eq_relations: InPlaceUnificationTable<TypeVarId>,
}

struct TypeVariableData {
    //    origin: TypeVariableOrigin,
//    diverging: bool,
}

struct Instantiate {
    tv: TypeVarId,
}

struct Delegate;

impl TypeVariableTable {
    /// Constructs a new generic type variable type
    pub fn new_type_var(&mut self) -> Ty {
        TyKind::InferenceVar(InferTy::Type(
            self.eq_relations.new_key(TypeVarValue::Unknown),
        ))
        .intern()
    }

    /// Constructs a new type variable that is used to represent *some* integer type
    pub fn new_integer_var(&mut self) -> Ty {
        TyKind::InferenceVar(InferTy::Int(
            self.eq_relations.new_key(TypeVarValue::Unknown),
        ))
        .intern()
    }

    /// Constructs a new type variable that is used to represent *some* floating-point type
    pub fn new_float_var(&mut self) -> Ty {
        TyKind::InferenceVar(InferTy::Float(
            self.eq_relations.new_key(TypeVarValue::Unknown),
        ))
        .intern()
    }

    /// Unifies the two types. If one or more type variables are involved instantiate or equate the
    /// variables with each other.
    pub fn unify(&mut self, a: &Ty, b: &Ty) -> bool {
        self.unify_inner(a, b)
    }

    /// Unifies the two types. If one or more type variables are involved instantiate or equate the
    /// variables with each other.
    fn unify_inner(&mut self, a: &Ty, b: &Ty) -> bool {
        if a == b {
            return true;
        }

        // First resolve both types as much as possible
        let a = self.replace_if_possible(a);
        let b = self.replace_if_possible(b);
        if a.equals_ctor(&b) {
            match (a.interned(), b.interned()) {
                (TyKind::Tuple(_, a), TyKind::Tuple(_, b)) => self.unify_substitutions(a, b),
                _ => true,
            }
        } else {
            self.unify_inner_trivial(&a, &b)
        }
    }

    fn unify_substitutions(&mut self, substs1: &Substitution, substs2: &Substitution) -> bool {
        substs1
            .0
            .iter()
            .zip(substs2.0.iter())
            .all(|(t1, t2)| self.unify_inner(t1, t2))
    }

    /// Handles unificiation of trivial cases.
    pub(crate) fn unify_inner_trivial(&mut self, a: &Ty, b: &Ty) -> bool {
        match (a.interned(), b.interned()) {
            // Ignore unificiation if dealing with unknown types, there are no guarentees in that case.
            (TyKind::Unknown, _) | (_, TyKind::Unknown) => true,

            // In case of two unknowns of the same type, equate them
            (
                TyKind::InferenceVar(InferTy::Type(tv_a)),
                TyKind::InferenceVar(InferTy::Type(tv_b)),
            )
            | (
                TyKind::InferenceVar(InferTy::Int(tv_a)),
                TyKind::InferenceVar(InferTy::Int(tv_b)),
            )
            | (
                TyKind::InferenceVar(InferTy::Float(tv_a)),
                TyKind::InferenceVar(InferTy::Float(tv_b)),
            ) => {
                self.equate(*tv_a, *tv_b);
                true
            }

            // Instantiate the variable if unifying with a concrete type
            (TyKind::InferenceVar(InferTy::Type(tv)), other)
            | (other, TyKind::InferenceVar(InferTy::Type(tv))) => {
                self.instantiate(*tv, other.clone().intern());
                true
            }

            // Instantiate the variable if unifying an unknown integer type with a concrete integer type
            (TyKind::InferenceVar(InferTy::Int(tv)), other @ TyKind::Int(_))
            | (other @ TyKind::Int(_), TyKind::InferenceVar(InferTy::Int(tv))) => {
                self.instantiate(*tv, other.clone().intern());
                true
            }

            // Instantiate the variable if unifying an unknown float type with a concrete float type
            (TyKind::InferenceVar(InferTy::Float(tv)), other @ TyKind::Float(_))
            | (other @ TyKind::Float(_), TyKind::InferenceVar(InferTy::Float(tv))) => {
                self.instantiate(*tv, other.clone().intern());
                true
            }

            // Was not able to unify the types
            _ => false,
        }
    }

    /// Records that `a == b`
    fn equate(&mut self, a: TypeVarId, b: TypeVarId) {
        debug_assert!(self.eq_relations.probe_value(a).is_unknown());
        debug_assert!(self.eq_relations.probe_value(b).is_unknown());
        self.eq_relations.union(a, b);
    }

    /// Instantiates `tv` with the type `ty`. Instantiation is the process of associating a concrete
    /// type with a type variable which in turn will resolve all equated type variables.
    fn instantiate(&mut self, tv: TypeVarId, ty: Ty) {
        debug_assert!(
            self.eq_relations.probe_value(tv).is_unknown(),
            "instantiating type variable `{:?}` twice: new-value = {:?}, old-value={:?}",
            tv,
            ty,
            self.eq_relations.probe_value(tv).known().unwrap()
        );
        self.eq_relations.union_value(tv, TypeVarValue::Known(ty));
    }

    /// If `ty` is a type variable, and it has been instantiated, then return the instantiated type;
    /// otherwise returns `ty`.
    pub fn replace_if_possible<'t>(&mut self, ty: &'t Ty) -> Cow<'t, Ty> {
        let mut ty = Cow::Borrowed(ty);

        // The type variable could resolve to an int/float variable. Therefore try to resolve up to
        // three times; each type of variable shouldn't occur more than once
        for _i in 0..3 {
            match ty.interned() {
                TyKind::InferenceVar(tv) => {
                    let inner = tv.to_inner();
                    match self.eq_relations.inlined_probe_value(inner).known() {
                        Some(known_ty) => ty = Cow::Owned(known_ty.clone()),
                        _ => return ty,
                    }
                }
                _ => return ty,
            }
        }

        ty
    }

    /// Resolves the type as far as currently possible, replacing type variables by their known
    /// types. All types returned by the `infer_*` functions should be resolved as far as possible,
    /// i.e. contain no type variables with known type.
    pub(crate) fn resolve_ty_as_far_as_possible(&mut self, ty: Ty) -> Ty {
        self.resolve_ty_as_far_as_possible_inner(&mut Vec::new(), ty)
    }

    pub(crate) fn resolve_ty_as_far_as_possible_inner(
        &mut self,
        tv_stack: &mut Vec<TypeVarId>,
        ty: Ty,
    ) -> Ty {
        ty.fold(&mut |ty| match ty.interned() {
            TyKind::InferenceVar(tv) => {
                let inner = tv.to_inner();
                if tv_stack.contains(&inner) {
                    return tv.fallback_value();
                }
                if let Some(known_ty) = self.eq_relations.inlined_probe_value(inner).known() {
                    tv_stack.push(inner);
                    let result =
                        self.resolve_ty_as_far_as_possible_inner(tv_stack, known_ty.clone());
                    tv_stack.pop();
                    result
                } else {
                    ty
                }
            }
            _ => ty,
        })
    }

    /// Resolves the type completely; type variables without known type are replaced by Ty::Unknown.
    pub(crate) fn resolve_ty_completely(&mut self, ty: Ty) -> Ty {
        self.resolve_ty_completely_inner(&mut Vec::new(), ty)
    }

    pub(crate) fn resolve_ty_completely_inner(
        &mut self,
        tv_stack: &mut Vec<TypeVarId>,
        ty: Ty,
    ) -> Ty {
        ty.fold(&mut |ty| match ty.interned() {
            TyKind::InferenceVar(tv) => {
                let inner = tv.to_inner();
                if tv_stack.contains(&inner) {
                    return tv.fallback_value();
                }
                if let Some(known_ty) = self.eq_relations.inlined_probe_value(inner).known() {
                    // known_ty may contain other variables that are known by now
                    tv_stack.push(inner);
                    let result = self.resolve_ty_completely_inner(tv_stack, known_ty.clone());
                    tv_stack.pop();
                    result
                } else {
                    tv.fallback_value()
                }
            }
            _ => ty,
        })
    }

    // /// Returns indices of all variables that are not yet instantiated.
    // pub fn unsolved_variables(&mut self) -> Vec<TypeVarId> {
    //     (0..self.values.len())
    //         .filter_map(|i| {
    //             let tv = TypeVarId::from_index(i as u32);
    //             match self.eq_relations.probe_value(tv) {
    //                 TypeVarValue::Unknown { .. } => Some(tv),
    //                 TypeVarValue::Known { .. } => None,
    //             }
    //         })
    //         .collect()
    // }
    //
    // /// Returns true if the table still contains unresolved type variables
    // pub fn has_unsolved_variables(&mut self) -> bool {
    //     (0..self.values.len()).any(|i| {
    //         let tv = TypeVarId::from_index(i as u32);
    //         match self.eq_relations.probe_value(tv) {
    //             TypeVarValue::Unknown { .. } => true,
    //             TypeVarValue::Known { .. } => false,
    //         }
    //     })
    // }
}
