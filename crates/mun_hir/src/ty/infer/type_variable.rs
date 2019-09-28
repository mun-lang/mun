use crate::Ty;
use drop_bomb::DropBomb;
use ena::snapshot_vec::{SnapshotVec, SnapshotVecDelegate};
use ena::unify::{InPlace, InPlaceUnificationTable, NoError, UnifyKey, UnifyValue};
use std::borrow::Cow;
use std::fmt;

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
    values: SnapshotVec<Delegate>,
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
    /// Creates a new generic infer type variable
    pub fn new_type_var(&mut self) -> TypeVarId {
        let eq_key = self.eq_relations.new_key(TypeVarValue::Unknown);
        let index = self.values.push(TypeVariableData {});
        assert_eq!(eq_key.0, index as u32);
        eq_key
    }

    /// Records that `a == b`
    pub fn equate(&mut self, a: TypeVarId, b: TypeVarId) {
        debug_assert!(self.eq_relations.probe_value(a).is_unknown());
        debug_assert!(self.eq_relations.probe_value(b).is_unknown());
        self.eq_relations.union(a, b);
    }

    /// Instantiates `tv` with the type `ty`.
    pub fn instantiate(&mut self, tv: TypeVarId, ty: Ty) {
        debug_assert!(
            self.eq_relations.probe_value(tv).is_unknown(),
            "instantiating type variable `{:?}` twice: new-value = {:?}, old-value={:?}",
            tv,
            ty,
            self.eq_relations.probe_value(tv).known().unwrap()
        );
        self.eq_relations.union_value(tv, TypeVarValue::Known(ty));
    }

    /// If `ty` is a type-inference variable, and it has been instantiated, then return the
    /// instantiated type; otherwise returns `ty`.
    pub fn replace_if_possible<'t>(&mut self, ty: &'t Ty) -> Cow<'t, Ty> {
        let ty = Cow::Borrowed(ty);
        match &*ty {
            Ty::Infer(tv) => match self.eq_relations.probe_value(*tv).known() {
                Some(known_ty) => Cow::Owned(known_ty.clone()),
                _ => ty,
            },
            _ => ty,
        }
    }

    /// Returns indices of all variables that are not yet instantiated.
    pub fn unsolved_variables(&mut self) -> Vec<TypeVarId> {
        (0..self.values.len())
            .filter_map(|i| {
                let tv = TypeVarId::from_index(i as u32);
                match self.eq_relations.probe_value(tv) {
                    TypeVarValue::Unknown { .. } => Some(tv),
                    TypeVarValue::Known { .. } => None,
                }
            })
            .collect()
    }

    /// Returns true if the table still contains unresolved type variables
    pub fn has_unsolved_variables(&mut self) -> bool {
        (0..self.values.len()).any(|i| {
            let tv = TypeVarId::from_index(i as u32);
            match self.eq_relations.probe_value(tv) {
                TypeVarValue::Unknown { .. } => true,
                TypeVarValue::Known { .. } => false,
            }
        })
    }
}

pub struct Snapshot {
    snapshot: ena::snapshot_vec::Snapshot,
    eq_snapshot: ena::unify::Snapshot<InPlace<TypeVarId>>,
    bomb: DropBomb,
}

impl TypeVariableTable {
    /// Creates a snapshot of the type variable state. This snapshot must later be committed
    /// (`commit`) or rolled back (`rollback_to()`). Nested snapshots are permitted but must be
    /// processed in a stack-like fashion.
    pub fn snapshot(&mut self) -> Snapshot {
        Snapshot {
            snapshot: self.values.start_snapshot(),
            eq_snapshot: self.eq_relations.snapshot(),
            bomb: DropBomb::new("Snapshot must be committed or rolled back"),
        }
    }

    /// Undoes all changes since the snapshot was created. Any snapshot created since that point
    /// must already have been committed or rolled back.
    pub fn rollback_to(&mut self, s: Snapshot) {
        let Snapshot {
            snapshot,
            eq_snapshot,
            mut bomb,
        } = s;
        self.values.rollback_to(snapshot);
        self.eq_relations.rollback_to(eq_snapshot);
        bomb.defuse();
    }

    /// Commits all changes since the snapshot was created, making them permanent (unless this
    /// snapshot was created within another snapshot). Any snapshot created since that point
    /// must already have been committed or rolled back.
    pub fn commit(&mut self, s: Snapshot) {
        let Snapshot {
            snapshot,
            eq_snapshot,
            mut bomb,
        } = s;
        self.values.commit(snapshot);
        self.eq_relations.commit(eq_snapshot);
        bomb.defuse();
    }
}

impl SnapshotVecDelegate for Delegate {
    type Value = TypeVariableData;
    type Undo = Instantiate;

    fn reverse(_values: &mut Vec<TypeVariableData>, _action: Instantiate) {
        // We don't actually have to *do* anything to reverse an
        // instantiation; the value for a variable is stored in the
        // `eq_relations` and hence its rollback code will handle
        // it. In fact, we could *almost* just remove the
        // `SnapshotVec` entirely, except that we would have to
        // reproduce *some* of its logic, since we want to know which
        // type variables have been instantiated since the snapshot
        // was started, so we can implement `types_escaping_snapshot`.
        //
        // (If we extended the `UnificationTable` to let us see which
        // values have been unified and so forth, that might also
        // suffice.)
    }
}
