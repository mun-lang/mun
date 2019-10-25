pub(crate) use self::diagnostics::LowerDiagnostic;
use crate::code_model::BuiltinType;
use crate::name_resolution::Namespace;
use crate::resolve::{Resolution, Resolver};
use crate::ty::{FnSig, Ty, TypeCtor};
use crate::type_ref::{TypeRef, TypeRefId, TypeRefMap};
use crate::{Function, HirDatabase, ModuleDef, Path};

#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct LowerResult {
    pub(crate) ty: Ty,
    pub(crate) diagnostics: Vec<LowerDiagnostic>,
}

impl Ty {
    pub(crate) fn from_hir(
        db: &impl HirDatabase,
        resolver: &Resolver,
        type_ref_map: &TypeRefMap,
        type_ref: &TypeRefId,
    ) -> LowerResult {
        let mut diagnostics = Vec::new();
        let ty =
            Ty::from_hir_with_diagnostics(db, resolver, type_ref_map, &mut diagnostics, type_ref);
        LowerResult { ty, diagnostics }
    }

    fn from_hir_with_diagnostics(
        db: &impl HirDatabase,
        resolver: &Resolver,
        type_ref_map: &TypeRefMap,
        diagnostics: &mut Vec<LowerDiagnostic>,
        type_ref: &TypeRefId,
    ) -> Ty {
        let res = match &type_ref_map[*type_ref] {
            TypeRef::Path(path) => Ty::from_hir_path(db, resolver, path),
            TypeRef::Error => Some(Ty::Unknown),
            TypeRef::Empty => Some(Ty::Empty),
        };
        if let Some(ty) = res {
            ty
        } else {
            diagnostics.push(LowerDiagnostic::UnresolvedType {
                id: type_ref.clone(),
            });
            Ty::Unknown
        }
    }

    pub(crate) fn from_hir_path(
        db: &impl HirDatabase,
        resolver: &Resolver,
        path: &Path,
    ) -> Option<Self> {
        let resolution = resolver
            .resolve_path_without_assoc_items(db, path)
            .take_types();

        let def = match resolution {
            Some(Resolution::Def(def)) => def,
            Some(Resolution::LocalBinding(..)) => {
                // this should never happen
                panic!("path resolved to local binding in type ns");
            }
            None => return None,
        };

        let typable: TypableDef = match def.into() {
            None => return None,
            Some(it) => it,
        };

        let ty = db.type_for_def(typable, Namespace::Types);
        Some(ty)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TypableDef {
    Function(Function),
    BuiltinType(BuiltinType),
}

impl From<Function> for TypableDef {
    fn from(f: Function) -> Self {
        TypableDef::Function(f)
    }
}

impl From<BuiltinType> for TypableDef {
    fn from(f: BuiltinType) -> Self {
        TypableDef::BuiltinType(f)
    }
}

impl From<ModuleDef> for Option<TypableDef> {
    fn from(d: ModuleDef) -> Self {
        match d {
            ModuleDef::Function(f) => Some(TypableDef::Function(f)),
            ModuleDef::BuiltinType(t) => Some(TypableDef::BuiltinType(t)),
        }
    }
}

/// Build the declared type of an item. This depends on the namespace; e.g. for
/// `struct Foo(usize)`, we have two types: The type of the struct itself, and
/// the constructor function `(usize) -> Foo` which lives in the values
/// namespace.
pub(crate) fn type_for_def(db: &impl HirDatabase, def: TypableDef, ns: Namespace) -> Ty {
    match (def, ns) {
        (TypableDef::Function(f), Namespace::Values) => type_for_fn(db, f),
        (TypableDef::BuiltinType(t), Namespace::Types) => type_for_builtin(t),

        // 'error' cases:
        (TypableDef::Function(_), Namespace::Types) => Ty::Unknown,
        (TypableDef::BuiltinType(_), Namespace::Values) => Ty::Unknown,
    }
}

/// Build the declared type of a static.
fn type_for_builtin(def: BuiltinType) -> Ty {
    Ty::simple(match def {
        BuiltinType::Float => TypeCtor::Float,
        BuiltinType::Int => TypeCtor::Int,
        BuiltinType::Boolean => TypeCtor::Bool,
    })
}

/// Build the declared type of a function. This should not need to look at the
/// function body.
fn type_for_fn(_db: &impl HirDatabase, def: Function) -> Ty {
    Ty::simple(TypeCtor::FnDef(def))
}

pub fn fn_sig_for_fn(db: &impl HirDatabase, def: Function) -> FnSig {
    let data = def.data(db);
    let resolver = def.resolver(db);
    let params = data
        .params()
        .iter()
        .map(|tr| Ty::from_hir(db, &resolver, data.type_ref_map(), tr).ty)
        .collect::<Vec<_>>();
    let ret = Ty::from_hir(db, &resolver, data.type_ref_map(), data.ret_type()).ty;
    FnSig::from_params_and_return(params, ret)
}

pub mod diagnostics {
    use crate::type_ref::TypeRefId;

    #[derive(Debug, PartialEq, Eq, Clone)]
    pub(crate) enum LowerDiagnostic {
        UnresolvedType { id: TypeRefId },
    }
}
