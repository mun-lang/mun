pub(crate) use self::diagnostics::LowerDiagnostic;
use crate::resolve::{HasResolver, TypeNs};
use crate::ty::{Substitution, TyKind};
use crate::{
    arena::map::ArenaMap,
    code_model::StructKind,
    diagnostics::DiagnosticSink,
    name_resolution::Namespace,
    primitive_type::PrimitiveType,
    resolve::Resolver,
    ty::{FnSig, Ty},
    type_ref::{LocalTypeRefId, TypeRef, TypeRefMap, TypeRefSourceMap},
    FileId, Function, HirDatabase, ModuleDef, Path, Struct, TypeAlias,
};
use std::{ops::Index, sync::Arc};

#[derive(Clone, Debug, PartialEq, Eq)]
struct LowerBatchStandardTypes {
    unknown: Ty,
}

impl Default for LowerBatchStandardTypes {
    fn default() -> Self {
        LowerBatchStandardTypes {
            unknown: TyKind::Unknown.intern(),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct LowerResult {
    pub(crate) ty: Ty,
    pub(crate) diagnostics: Vec<LowerDiagnostic>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LowerBatchResult {
    pub(crate) type_ref_to_type: ArenaMap<LocalTypeRefId, Ty>,
    pub(crate) diagnostics: Vec<LowerDiagnostic>,

    standard_types: LowerBatchStandardTypes,
}

impl Index<LocalTypeRefId> for LowerBatchResult {
    type Output = Ty;
    fn index(&self, expr: LocalTypeRefId) -> &Ty {
        self.type_ref_to_type
            .get(expr)
            .unwrap_or(&self.standard_types.unknown)
    }
}

impl LowerBatchResult {
    /// Adds all the `LowerDiagnostic`s of the result to the `DiagnosticSink`.
    pub(crate) fn add_diagnostics(
        &self,
        db: &dyn HirDatabase,
        file_id: FileId,
        source_map: &TypeRefSourceMap,
        sink: &mut DiagnosticSink,
    ) {
        self.diagnostics
            .iter()
            .for_each(|it| it.add_to(db, file_id, source_map, sink))
    }
}

impl Ty {
    pub(crate) fn from_hir(
        db: &dyn HirDatabase,
        resolver: &Resolver,
        type_ref_map: &TypeRefMap,
        type_ref: LocalTypeRefId,
    ) -> LowerResult {
        let mut diagnostics = Vec::new();
        let ty =
            Ty::from_hir_with_diagnostics(db, resolver, type_ref_map, &mut diagnostics, type_ref);
        LowerResult { ty, diagnostics }
    }

    fn from_hir_with_diagnostics(
        db: &dyn HirDatabase,
        resolver: &Resolver,
        type_ref_map: &TypeRefMap,
        diagnostics: &mut Vec<LowerDiagnostic>,
        type_ref: LocalTypeRefId,
    ) -> Ty {
        let res = match &type_ref_map[type_ref] {
            TypeRef::Path(path) => Ty::from_hir_path(db, resolver, type_ref, path, diagnostics),
            TypeRef::Error => Some((TyKind::Unknown.intern(), false)),
            TypeRef::Empty => Some((Ty::unit(), false)),
            TypeRef::Never => Some((TyKind::Never.intern(), false)),
        };
        if let Some((ty, is_cyclic)) = res {
            if is_cyclic {
                diagnostics.push(LowerDiagnostic::CyclicType { id: type_ref })
            }
            ty
        } else {
            diagnostics.push(LowerDiagnostic::UnresolvedType { id: type_ref });
            TyKind::Unknown.intern()
        }
    }

    fn from_hir_path(
        db: &dyn HirDatabase,
        resolver: &Resolver,
        type_ref: LocalTypeRefId,
        path: &Path,
        diagnostics: &mut Vec<LowerDiagnostic>,
    ) -> Option<(Self, bool)> {
        // Find the type
        let (ty, vis) = resolver.resolve_path_as_type_fully(db.upcast(), path)?;

        // Get the definition and visibility
        let def = match ty {
            TypeNs::StructId(id) => TypableDef::Struct(id.into()),
            TypeNs::TypeAliasId(id) => TypableDef::TypeAlias(id.into()),
            TypeNs::PrimitiveType(id) => TypableDef::PrimitiveType(id),
        };

        // Get the current module and see if the type is visible from here
        if let Some(module) = resolver.module() {
            if !vis.is_visible_from(db, module) {
                diagnostics.push(LowerDiagnostic::TypeIsPrivate { id: type_ref })
            }
        }

        Some(db.type_for_def(def, Namespace::Types))
    }
}

pub fn types_from_hir(
    db: &dyn HirDatabase,
    resolver: &Resolver,
    type_ref_map: &TypeRefMap,
) -> Arc<LowerBatchResult> {
    let mut result = LowerBatchResult::default();
    for (id, _) in type_ref_map.iter() {
        let LowerResult { ty, diagnostics } = Ty::from_hir(db, resolver, type_ref_map, id);
        for diagnostic in diagnostics {
            result.diagnostics.push(diagnostic);
        }
        // TODO: Add detection of cyclic types
        result.type_ref_to_type.insert(id, ty);
    }
    Arc::new(result)
}

pub fn lower_struct_query(db: &dyn HirDatabase, s: Struct) -> Arc<LowerBatchResult> {
    let data = s.data(db.upcast());
    types_from_hir(db, &s.id.resolver(db.upcast()), data.type_ref_map())
}

pub fn lower_type_alias_query(db: &dyn HirDatabase, t: TypeAlias) -> Arc<LowerBatchResult> {
    let data = t.data(db.upcast());
    types_from_hir(db, &t.id.resolver(db.upcast()), data.type_ref_map())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TypableDef {
    Function(Function),
    PrimitiveType(PrimitiveType),
    Struct(Struct),
    TypeAlias(TypeAlias),
}

impl From<Function> for TypableDef {
    fn from(f: Function) -> Self {
        TypableDef::Function(f)
    }
}

impl From<PrimitiveType> for TypableDef {
    fn from(f: PrimitiveType) -> Self {
        TypableDef::PrimitiveType(f)
    }
}

impl From<Struct> for TypableDef {
    fn from(f: Struct) -> Self {
        TypableDef::Struct(f)
    }
}

impl From<ModuleDef> for Option<TypableDef> {
    fn from(d: ModuleDef) -> Self {
        match d {
            ModuleDef::Function(f) => Some(TypableDef::Function(f)),
            ModuleDef::PrimitiveType(t) => Some(TypableDef::PrimitiveType(t)),
            ModuleDef::Struct(t) => Some(TypableDef::Struct(t)),
            ModuleDef::TypeAlias(t) => Some(TypableDef::TypeAlias(t)),
            ModuleDef::Module(_) => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CallableDef {
    Function(Function),
    Struct(Struct),
}
impl_froms!(CallableDef: Function, Struct);

impl CallableDef {
    pub fn is_function(self) -> bool {
        matches!(self, CallableDef::Function(_))
    }

    pub fn is_struct(self) -> bool {
        matches!(self, CallableDef::Struct(_))
    }
}

/// Build the declared type of an item. This depends on the namespace; e.g. for
/// `struct Foo(usize)`, we have two types: The type of the struct itself, and
/// the constructor function `(usize) -> Foo` which lives in the values
/// namespace.
pub(crate) fn type_for_def(db: &dyn HirDatabase, def: TypableDef, ns: Namespace) -> (Ty, bool) {
    let ty = match (def, ns) {
        (TypableDef::Function(f), Namespace::Values) => type_for_fn(db, f),
        (TypableDef::PrimitiveType(t), Namespace::Types) => type_for_primitive(t),
        (TypableDef::Struct(s), Namespace::Values) => type_for_struct_constructor(db, s),
        (TypableDef::Struct(s), Namespace::Types) => type_for_struct(db, s),
        (TypableDef::TypeAlias(t), Namespace::Types) => type_for_type_alias(db, t),

        // 'error' cases:
        (TypableDef::Function(_), Namespace::Types) => TyKind::Unknown.intern(),
        (TypableDef::PrimitiveType(_), Namespace::Values) => TyKind::Unknown.intern(),
        (TypableDef::TypeAlias(_), Namespace::Values) => TyKind::Unknown.intern(),
    };
    (ty, false)
}

/// Recover with an unknown type when a cycle is detected in the salsa database.
pub(crate) fn type_for_cycle_recover(
    _db: &dyn HirDatabase,
    _cycle: &[String],
    _def: &TypableDef,
    _ns: &Namespace,
) -> (Ty, bool) {
    (TyKind::Unknown.intern(), true)
}

/// Build the declared type of a static.
fn type_for_primitive(def: PrimitiveType) -> Ty {
    match def {
        PrimitiveType::Float(f) => TyKind::Float(f.into()),
        PrimitiveType::Int(i) => TyKind::Int(i.into()),
        PrimitiveType::Bool => TyKind::Bool,
    }
    .intern()
}

/// Build the declared type of a function. This should not need to look at the
/// function body.
fn type_for_fn(_db: &dyn HirDatabase, def: Function) -> Ty {
    TyKind::FnDef(def.into(), Substitution::empty()).intern()
}

pub(crate) fn callable_item_sig(db: &dyn HirDatabase, def: CallableDef) -> FnSig {
    match def {
        CallableDef::Function(f) => fn_sig_for_fn(db, f),
        CallableDef::Struct(s) => fn_sig_for_struct_constructor(db, s),
    }
}

pub(crate) fn fn_sig_for_fn(db: &dyn HirDatabase, def: Function) -> FnSig {
    let data = def.data(db.upcast());
    let resolver = def.id.resolver(db.upcast());
    let params = data
        .params()
        .iter()
        .map(|tr| Ty::from_hir(db, &resolver, data.type_ref_map(), *tr).ty)
        .collect::<Vec<_>>();
    let ret = Ty::from_hir(db, &resolver, data.type_ref_map(), *data.ret_type()).ty;
    FnSig::from_params_and_return(params, ret)
}

pub(crate) fn fn_sig_for_struct_constructor(db: &dyn HirDatabase, def: Struct) -> FnSig {
    let data = def.data(db.upcast());
    let resolver = def.id.resolver(db.upcast());
    let params = data
        .fields
        .iter()
        .map(|(_, field)| Ty::from_hir(db, &resolver, data.type_ref_map(), field.type_ref).ty)
        .collect::<Vec<_>>();
    let ret = type_for_struct(db, def);
    FnSig::from_params_and_return(params, ret)
}

/// Build the type of a struct constructor.
fn type_for_struct_constructor(db: &dyn HirDatabase, def: Struct) -> Ty {
    let struct_data = db.struct_data(def.id);
    if struct_data.kind == StructKind::Tuple {
        TyKind::FnDef(def.into(), Substitution::empty()).intern()
    } else {
        type_for_struct(db, def)
    }
}

fn type_for_struct(_db: &dyn HirDatabase, def: Struct) -> Ty {
    TyKind::Struct(def).intern()
}

fn type_for_type_alias(db: &dyn HirDatabase, def: TypeAlias) -> Ty {
    let data = def.data(db.upcast());
    let resolver = def.id.resolver(db.upcast());
    let type_ref = def.type_ref(db);
    Ty::from_hir(db, &resolver, data.type_ref_map(), type_ref).ty
}

pub mod diagnostics {
    use crate::diagnostics::{CyclicType, PrivateAccess, UnresolvedType};
    use crate::{
        diagnostics::DiagnosticSink,
        type_ref::{LocalTypeRefId, TypeRefSourceMap},
        FileId, HirDatabase,
    };

    #[derive(Debug, PartialEq, Eq, Clone)]
    pub(crate) enum LowerDiagnostic {
        UnresolvedType { id: LocalTypeRefId },
        TypeIsPrivate { id: LocalTypeRefId },
        CyclicType { id: LocalTypeRefId },
    }

    impl LowerDiagnostic {
        pub(crate) fn add_to(
            &self,
            _db: &dyn HirDatabase,
            file_id: FileId,
            source_map: &TypeRefSourceMap,
            sink: &mut DiagnosticSink,
        ) {
            match self {
                LowerDiagnostic::UnresolvedType { id } => sink.push(UnresolvedType {
                    file: file_id,
                    type_ref: source_map.type_ref_syntax(*id).unwrap(),
                }),
                LowerDiagnostic::CyclicType { id } => sink.push(CyclicType {
                    file: file_id,
                    type_ref: source_map.type_ref_syntax(*id).unwrap(),
                }),
                LowerDiagnostic::TypeIsPrivate { id } => sink.push(PrivateAccess {
                    file: file_id,
                    expr: source_map.type_ref_syntax(*id).unwrap().syntax_node_ptr(),
                }),
            }
        }
    }
}
