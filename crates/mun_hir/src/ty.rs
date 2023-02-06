mod infer;
pub(super) mod lower;
mod op;
mod primitives;
mod resolve;

use crate::{
    display::{HirDisplay, HirFormatter},
    ty::infer::InferTy,
    ty::lower::fn_sig_for_struct_constructor,
    HasVisibility, HirDatabase, Struct, StructMemoryKind, TypeAlias, Visibility,
};
pub(crate) use infer::infer_query;
pub use infer::InferenceResult;
pub(crate) use lower::{
    callable_item_sig, fn_sig_for_fn, type_for_cycle_recover, type_for_def, CallableDef, TypableDef,
};
pub use primitives::{FloatTy, IntTy};
pub use resolve::ResolveBitness;
use smallvec::SmallVec;
use std::{fmt, iter::FromIterator, mem, ops::Deref, sync::Arc};

#[cfg(test)]
mod tests;

/// A kind of type.
#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub enum TyKind {
    /// An abstract datatype (structures, tuples, or enumerations)
    /// TODO: Add enumerations
    Struct(Struct),

    /// The primitive floating point type. Written as `float`.
    Float(FloatTy),

    /// The primitive integral type. Written as `int`.
    Int(IntTy),

    /// The primitive boolean type. Written as `bool`.
    Bool,

    /// A tuple type. For example `(f32, f64, bool)`.
    Tuple(usize, Substitution),

    /// A type variable used during type checking. Not to be confused with a type parameter.
    InferenceVar(InferTy),

    /// A type alias
    TypeAlias(TypeAlias),

    /// The never type `never`.
    Never,

    /// The anonymous type of a function declaration/definition. Each function has a unique type,
    /// which is output (for a function named `foo` returning an `number`) as
    /// `fn() -> number {foo}`.
    ///
    /// This includes tuple struct / enum variant constructors as well.
    ///
    /// For example the type of `bar` here:
    ///
    /// ```mun
    /// function foo() -> number { 1 }
    /// let bar = foo; // bar: function() -> number {foo}
    /// ```
    FnDef(CallableDef, Substitution),

    /// An dynamically sized array type
    Array(Ty),

    /// A placeholder for a type which could not be computed; this is propagated to avoid useless
    /// error messages. Doubles as a placeholder where type variables are inserted before type
    /// checking, since we want to try to infer a better type here anyway -- for the IDE use case,
    /// we want to try to infer as much as possible even in the presence of type errors.
    Unknown,
}

/// External representation of a type. This should be cheap to clone.
#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub struct Ty(Arc<TyKind>);

impl HasVisibility for Ty {
    fn visibility(&self, db: &dyn HirDatabase) -> Visibility {
        self.0.visibility(db)
    }
}

impl TyKind {
    /// Constructs a new `Ty` by interning self
    pub fn intern(self) -> Ty {
        Ty(Arc::new(self))
    }
}

impl HasVisibility for TyKind {
    fn visibility(&self, db: &dyn HirDatabase) -> Visibility {
        match self {
            TyKind::Struct(strukt) => strukt.visibility(db),
            TyKind::TypeAlias(type_alias) => type_alias.visibility(db),
            TyKind::FnDef(callable_def, _) => callable_def.visibility(db),
            _ => Visibility::Public,
        }
    }
}

impl Ty {
    /// Returns the `TyKind` from which this instance was constructed
    pub fn interned(&self) -> &TyKind {
        &self.0
    }

    /// Returns a mutable reference of `TyKind` for this instance
    pub fn interned_mut(&mut self) -> &mut TyKind {
        Arc::make_mut(&mut self.0)
    }

    /// Convert this instance back to the `TyKind` that created it.
    pub fn into_inner(self) -> TyKind {
        Arc::try_unwrap(self.0).unwrap_or_else(|a| (*a).clone())
    }
}

impl Ty {
    /// Constructs an instance of the unit type `()`
    pub fn unit() -> Self {
        TyKind::Tuple(0, Substitution::empty()).intern()
    }

    /// Constructs a new struct type
    pub fn struct_ty(strukt: Struct) -> Ty {
        TyKind::Struct(strukt).intern()
    }

    /// If this type represents a struct type, returns the type of the struct.
    pub fn as_struct(&self) -> Option<Struct> {
        match self.interned() {
            TyKind::Struct(s) => Some(*s),
            _ => None,
        }
    }

    /// If this type represents a tuple type, returns a reference to the substitutions of the tuple.
    pub fn as_tuple(&self) -> Option<&Substitution> {
        match self.interned() {
            TyKind::Tuple(_, substs) => Some(substs),
            _ => None,
        }
    }

    /// If this type represents an array type, returns a reference to the element type.
    pub fn as_array(&self) -> Option<&Ty> {
        match self.interned() {
            TyKind::Array(element_ty) => Some(element_ty),
            _ => None,
        }
    }

    /// Returns true if this type represents the empty tuple type
    pub fn is_empty(&self) -> bool {
        matches!(self.interned(), TyKind::Tuple(0, _))
    }

    /// Returns true if this type represents the never type
    pub fn is_never(&self) -> bool {
        matches!(self.interned(), TyKind::Never)
    }

    /// Returns the callable definition for the given expression or `None` if the type does not
    /// represent a callable.
    pub fn as_callable_def(&self) -> Option<CallableDef> {
        match self.interned() {
            TyKind::FnDef(def, _) => Some(*def),
            _ => None,
        }
    }

    /// Returns the callable signature of the type, if the type is callable.
    pub fn callable_sig(&self, db: &dyn HirDatabase) -> Option<FnSig> {
        match self.interned() {
            TyKind::FnDef(def, _) => Some(db.callable_sig(*def)),
            _ => None,
        }
    }

    /// Returns the type's name as a string, if one exists.
    ///
    /// This name needs to be unique as it is used to generate a type's `Guid`.
    pub fn guid_string(&self, db: &dyn HirDatabase) -> Option<String> {
        match self.interned() {
            &TyKind::Struct(s) => {
                let name = s.name(db).to_string();

                Some(if s.data(db.upcast()).memory_kind == StructMemoryKind::Gc {
                    format!("struct {name}")
                } else {
                    let fields: Vec<String> = s
                        .fields(db)
                        .into_iter()
                        .map(|f| {
                            let ty_string = f
                                .ty(db)
                                .guid_string(db)
                                .expect("type should be convertible to a string");
                            format!("{}: {}", f.name(db), ty_string)
                        })
                        .collect();

                    format!(
                        "struct {name}{{{fields}}}",
                        name = name,
                        fields = fields.join(",")
                    )
                })
            }
            TyKind::Bool => Some("core::bool".to_string()),
            TyKind::Float(ty) => Some(format!("core::{}", ty.as_str())),
            TyKind::Int(ty) => Some(format!("core::{}", ty.as_str())),
            TyKind::Array(ty) => Some(format!("[{}]", ty.display(db))),
            _ => None,
        }
    }

    /// Returns true if this instance represents a known type.
    pub fn is_known(&self) -> bool {
        !matches!(self.interned(), TyKind::Unknown)
    }

    /// Returns true if this instance is of an unknown type.
    pub fn is_unknown(&self) -> bool {
        matches!(self.interned(), TyKind::Unknown)
    }

    /// Returns the type parameters of this type if it has some (i.e. is an ADT or function); so
    /// if `self` is an `Option<u32>`, this returns the `u32`
    pub fn type_parameters(&self) -> Option<&Substitution> {
        match self.interned() {
            TyKind::Tuple(_, substs) | TyKind::FnDef(_, substs) => Some(substs),
            _ => None,
        }
    }

    /// Returns a mutable reference to the type parameters of this type if it has some (i.e. is an
    /// ADT or function); so if `self` is an `Option<u32>`, this returns the `u32`
    pub fn type_parameters_mut(&mut self) -> Option<&mut Substitution> {
        match self.interned_mut() {
            TyKind::Tuple(_, substs) | TyKind::FnDef(_, substs) => Some(substs),
            _ => None,
        }
    }

    /// Returns true if the other type has the same type constructor
    pub fn equals_ctor(&self, other: &Ty) -> bool {
        match (self.interned(), other.interned()) {
            (TyKind::Struct(s1), TyKind::Struct(s2)) => s1 == s2,
            (TyKind::Tuple(_, substs1), TyKind::Tuple(_, substs2)) => substs1 == substs2,
            (TyKind::Array(_), TyKind::Array(_)) => true,
            (TyKind::Float(f1), TyKind::Float(f2)) => f1 == f2,
            (TyKind::Int(i1), TyKind::Int(i2)) => i1 == i2,
            (TyKind::FnDef(def, _), TyKind::FnDef(def2, _)) => def == def2,
            (TyKind::Bool, TyKind::Bool) => true,
            _ => false,
        }
    }
}

/// A list of substitutions for generic parameters.
#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub struct Substitution(SmallVec<[Ty; 2]>);

impl Substitution {
    /// Constructs a new empty instance
    pub fn empty() -> Substitution {
        Substitution(SmallVec::new())
    }

    /// Constructs a new instance with a single type
    pub fn single(ty: Ty) -> Substitution {
        Substitution({
            let mut v = SmallVec::new();
            v.push(ty);
            v
        })
    }

    /// Returns a reference to the interned types of this instance
    pub fn interned(&self) -> &[Ty] {
        &self.0
    }

    /// Assumes this instance has a single element and returns it. Panics if this instance doesnt
    /// contain exactly one element.
    pub fn as_single(&self) -> &Ty {
        if self.0.len() != 1 {
            panic!("expected substs of len 1, got {self:?}");
        }
        &self.0[0]
    }
}

impl FromIterator<Ty> for Substitution {
    fn from_iter<T: IntoIterator<Item = Ty>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl Deref for Substitution {
    type Target = [Ty];

    fn deref(&self) -> &[Ty] {
        &self.0
    }
}

impl TypeWalk for Substitution {
    fn walk(&self, f: &mut impl FnMut(&Ty)) {
        for t in &self.0 {
            t.walk(f);
        }
    }

    fn walk_mut(&mut self, f: &mut impl FnMut(&mut Ty)) {
        for t in &mut self.0 {
            t.walk_mut(f);
        }
    }
}

/// A function signature as seen by type inference: Several parameter types and
/// one return type.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FnSig {
    params_and_return: Arc<[Ty]>,
}

impl FnSig {
    pub fn from_params_and_return(mut params: Vec<Ty>, ret: Ty) -> FnSig {
        params.push(ret);
        FnSig {
            params_and_return: params.into(),
        }
    }

    pub fn params(&self) -> &[Ty] {
        &self.params_and_return[0..self.params_and_return.len() - 1]
    }

    pub fn ret(&self) -> &Ty {
        &self.params_and_return[self.params_and_return.len() - 1]
    }

    pub fn marshallable(&self, db: &dyn HirDatabase) -> bool {
        for ty in self.params_and_return.iter() {
            if let Some(s) = ty.as_struct() {
                if s.data(db.upcast()).memory_kind == StructMemoryKind::Value {
                    return false;
                }
            }
        }
        true
    }
}

impl HirDisplay for Ty {
    fn hir_fmt(&self, f: &mut HirFormatter) -> fmt::Result {
        match self.interned() {
            TyKind::Struct(s) => write!(f, "{}", s.name(f.db)),
            TyKind::Float(ty) => write!(f, "{ty}"),
            TyKind::Int(ty) => write!(f, "{ty}"),
            TyKind::Bool => write!(f, "bool"),
            TyKind::Tuple(_, elems) => {
                write!(f, "(")?;
                f.write_joined(elems.iter(), ", ")?;
                if elems.len() == 1 {
                    write!(f, ",")?;
                }
                write!(f, ")")
            }
            TyKind::InferenceVar(tv) => match tv {
                InferTy::Type(tv) => write!(f, "'{}", tv.0),
                InferTy::Int(_) => write!(f, "{{integer}}"),
                InferTy::Float(_) => write!(f, "{{float}}"),
            },
            TyKind::TypeAlias(def) => write!(f, "{}", def.name(f.db)),
            TyKind::Never => write!(f, "never"),
            &TyKind::FnDef(CallableDef::Function(def), _) => {
                let sig = fn_sig_for_fn(f.db, def);
                let name = def.name(f.db);
                write!(f, "function {name}")?;
                write!(f, "(")?;
                f.write_joined(sig.params(), ", ")?;
                write!(f, ") -> {}", sig.ret().display(f.db))
            }
            &TyKind::FnDef(CallableDef::Struct(def), _) => {
                let sig = fn_sig_for_struct_constructor(f.db, def);
                let name = def.name(f.db);
                write!(f, "ctor {name}")?;
                write!(f, "(")?;
                f.write_joined(sig.params(), ", ")?;
                write!(f, ") -> {}", sig.ret().display(f.db))
            }
            TyKind::Array(elem_ty) => write!(f, "[{}]", elem_ty.display(f.db)),
            TyKind::Unknown => write!(f, "{{unknown}}"),
        }
    }
}

impl HirDisplay for &Ty {
    fn hir_fmt(&self, f: &mut HirFormatter) -> fmt::Result {
        HirDisplay::hir_fmt(*self, f)
    }
}

/// This allows walking structures that contain types.
pub trait TypeWalk {
    /// Calls the function `f` for each `Ty` in this instance.
    fn walk(&self, f: &mut impl FnMut(&Ty));

    /// Calls the function `f` for each `Ty` in this instance with a mutable reference.
    fn walk_mut(&mut self, f: &mut impl FnMut(&mut Ty));

    /// Folds this instance by replacing all instances of types with other instances as specified
    /// by the function `f`.
    fn fold(mut self, f: &mut impl FnMut(Ty) -> Ty) -> Self
    where
        Self: Sized,
    {
        self.walk_mut(&mut |ty_mut| {
            let ty = mem::replace(ty_mut, TyKind::Unknown.intern());
            *ty_mut = f(ty);
        });
        self
    }
}

impl TypeWalk for Ty {
    fn walk(&self, f: &mut impl FnMut(&Ty)) {
        match self.interned() {
            TyKind::Array(elem_ty) => f(elem_ty),
            _ => {
                if let Some(substs) = self.type_parameters() {
                    substs.walk(f)
                }
            }
        }
        f(self)
    }

    fn walk_mut(&mut self, f: &mut impl FnMut(&mut Ty)) {
        match self.interned_mut() {
            TyKind::Array(elem_ty) => f(elem_ty),
            _ => {
                if let Some(substs) = self.type_parameters_mut() {
                    substs.walk_mut(f)
                }
            }
        }
        f(self)
    }
}
