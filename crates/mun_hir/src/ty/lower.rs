//! Methods for lower the HIR to types.

use std::{ops::Index, sync::Arc};

use la_arena::ArenaMap;

pub(crate) use self::diagnostics::LowerDiagnostic;
use crate::{
    code_model::StructKind,
    diagnostics::DiagnosticSink,
    ids::ImplId,
    name_resolution::Namespace,
    primitive_type::PrimitiveType,
    resolve::{HasResolver, Resolver, TypeNs},
    ty::{FnSig, Substitution, Ty, TyKind},
    type_ref::{LocalTypeRefId, TypeRef, TypeRefMap, TypeRefSourceMap},
    FileId, Function, HasVisibility, HirDatabase, ModuleDef, Path, Struct, TypeAlias, Visibility,
};

/// A struct which holds resolved type references to `Ty`s.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LowerTyMap {
    pub(crate) type_ref_to_type: ArenaMap<LocalTypeRefId, Ty>,
    pub(crate) diagnostics: Vec<LowerDiagnostic>,

    unknown_ty: Ty,
}

impl Default for LowerTyMap {
    fn default() -> Self {
        LowerTyMap {
            type_ref_to_type: ArenaMap::default(),
            diagnostics: vec![],
            unknown_ty: TyKind::Unknown.intern(),
        }
    }
}

impl Index<LocalTypeRefId> for LowerTyMap {
    type Output = Ty;
    fn index(&self, expr: LocalTypeRefId) -> &Ty {
        self.type_ref_to_type.get(expr).unwrap_or(&self.unknown_ty)
    }
}

impl LowerTyMap {
    /// Adds all the `LowerDiagnostic`s of the result to the `DiagnosticSink`.
    pub(crate) fn add_diagnostics(
        &self,
        db: &dyn HirDatabase,
        file_id: FileId,
        source_map: &TypeRefSourceMap,
        sink: &mut DiagnosticSink<'_>,
    ) {
        self.diagnostics
            .iter()
            .for_each(|it| it.add_to(db, file_id, source_map, sink));
    }
}

impl Ty {
    /// Tries to lower a HIR type reference to an actual resolved type. Besides
    /// the type also returns an diagnostics that where encountered along
    /// the way.
    pub(crate) fn from_hir(
        db: &dyn HirDatabase,
        resolver: &Resolver,
        type_ref_map: &TypeRefMap,
        type_ref: LocalTypeRefId,
    ) -> (Ty, Vec<diagnostics::LowerDiagnostic>) {
        let mut diagnostics = Vec::new();
        let ty =
            Ty::from_hir_with_diagnostics(db, resolver, type_ref_map, &mut diagnostics, type_ref);
        (ty, diagnostics)
    }

    /// Tries to lower a HIR type reference to an actual resolved type. Takes a
    /// mutable reference to a `Vec` which will hold any diagnostics
    /// encountered a long the way.
    fn from_hir_with_diagnostics(
        db: &dyn HirDatabase,
        resolver: &Resolver,
        type_ref_map: &TypeRefMap,
        diagnostics: &mut Vec<LowerDiagnostic>,
        type_ref: LocalTypeRefId,
    ) -> Ty {
        let res = match &type_ref_map[type_ref] {
            TypeRef::Path(path) => Ty::from_path(db, resolver, type_ref, path, diagnostics),
            TypeRef::Error => Some(TyKind::Unknown.intern()),
            TypeRef::Tuple(inner) => {
                let inner_tys = inner.iter().map(|tr| {
                    Self::from_hir_with_diagnostics(db, resolver, type_ref_map, diagnostics, *tr)
                });
                Some(TyKind::Tuple(inner_tys.len(), inner_tys.collect()).intern())
            }
            TypeRef::Never => Some(TyKind::Never.intern()),
            TypeRef::Array(inner) => {
                let inner = Self::from_hir_with_diagnostics(
                    db,
                    resolver,
                    type_ref_map,
                    diagnostics,
                    *inner,
                );
                Some(TyKind::Array(inner).intern())
            }
        };
        if let Some(ty) = res {
            ty
        } else {
            diagnostics.push(LowerDiagnostic::UnresolvedType { id: type_ref });
            TyKind::Unknown.intern()
        }
    }

    /// Constructs a `Ty` from a path.
    fn from_path(
        db: &dyn HirDatabase,
        resolver: &Resolver,
        type_ref: LocalTypeRefId,
        path: &Path,
        diagnostics: &mut Vec<LowerDiagnostic>,
    ) -> Option<Self> {
        // Find the type namespace and visibility
        let (type_ns, vis) = resolver.resolve_path_as_type_fully(db.upcast(), path)?;

        // Get the current module and see if the type is visible from here
        if let Some(module) = resolver.module() {
            if !vis.is_visible_from(db, module) {
                diagnostics.push(LowerDiagnostic::TypeIsPrivate { id: type_ref });
            }
        }

        let type_for_def_fn = |def| Some(db.type_for_def(def, Namespace::Types));

        match type_ns {
            TypeNs::SelfType(id) => Some(db.type_for_impl_self(id)),
            TypeNs::StructId(id) => type_for_def_fn(TypableDef::Struct(id.into())),
            TypeNs::TypeAliasId(id) => type_for_def_fn(TypableDef::TypeAlias(id.into())),
            TypeNs::PrimitiveType(id) => type_for_def_fn(TypableDef::PrimitiveType(id)),
        }
    }
}

/// Resolves all types in the specified `TypeRefMap`.
pub fn lower_types(
    db: &dyn HirDatabase,
    resolver: &Resolver,
    type_ref_map: &TypeRefMap,
) -> Arc<LowerTyMap> {
    let mut result = LowerTyMap::default();
    for (id, _) in type_ref_map.iter() {
        let ty =
            Ty::from_hir_with_diagnostics(db, resolver, type_ref_map, &mut result.diagnostics, id);

        result.type_ref_to_type.insert(id, ty);
    }
    Arc::new(result)
}

pub fn lower_struct_query(db: &dyn HirDatabase, s: Struct) -> Arc<LowerTyMap> {
    let data = s.data(db.upcast());
    lower_types(db, &s.id.resolver(db.upcast()), data.type_ref_map())
}

pub fn lower_type_alias_query(db: &dyn HirDatabase, t: TypeAlias) -> Arc<LowerTyMap> {
    let data = t.data(db.upcast());
    lower_types(db, &t.id.resolver(db.upcast()), data.type_ref_map())
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

impl HasVisibility for CallableDef {
    fn visibility(&self, db: &dyn HirDatabase) -> Visibility {
        match self {
            CallableDef::Struct(strukt) => strukt.visibility(db),
            CallableDef::Function(function) => function.visibility(db),
        }
    }
}

/// Build the declared type of an item. This depends on the namespace; e.g. for
/// `struct Foo(usize)`, we have two types: The type of the struct itself, and
/// the constructor function `(usize) -> Foo` which lives in the values
/// namespace.
pub(crate) fn type_for_def(db: &dyn HirDatabase, def: TypableDef, ns: Namespace) -> Ty {
    match (def, ns) {
        (TypableDef::Function(f), Namespace::Values) => type_for_fn(db, f),
        (TypableDef::PrimitiveType(t), Namespace::Types) => type_for_primitive(t),
        (TypableDef::Struct(s), Namespace::Values) => type_for_struct_constructor(db, s),
        (TypableDef::Struct(s), Namespace::Types) => type_for_struct(db, s),
        (TypableDef::TypeAlias(t), Namespace::Types) => type_for_type_alias(db, t),

        // 'error' cases:
        (TypableDef::Function(_), Namespace::Types)
        | (TypableDef::PrimitiveType(_) | TypableDef::TypeAlias(_), Namespace::Values) => {
            TyKind::Unknown.intern()
        }
    }
}

pub(crate) fn type_for_impl_self(db: &dyn HirDatabase, i: ImplId) -> Ty {
    let impl_data = db.impl_data(i);
    let resolver = i.resolver(db.upcast());
    Ty::from_hir(db, &resolver, &impl_data.type_ref_map, impl_data.self_ty).0
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
        .map(|tr| Ty::from_hir(db, &resolver, data.type_ref_map(), *tr).0)
        .collect::<Vec<_>>();
    let ret = Ty::from_hir(db, &resolver, data.type_ref_map(), *data.ret_type()).0;
    FnSig::from_params_and_return(params, ret)
}

pub(crate) fn fn_sig_for_struct_constructor(db: &dyn HirDatabase, def: Struct) -> FnSig {
    let data = def.data(db.upcast());
    let resolver = def.id.resolver(db.upcast());
    let params = data
        .fields
        .iter()
        .map(|(_, field)| Ty::from_hir(db, &resolver, data.type_ref_map(), field.type_ref).0)
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

fn type_for_type_alias(_db: &dyn HirDatabase, def: TypeAlias) -> Ty {
    TyKind::TypeAlias(def).intern()
}

pub(crate) fn lower_impl_query(db: &dyn HirDatabase, impl_id: ImplId) -> Arc<LowerTyMap> {
    let impl_data = db.impl_data(impl_id);
    let resolver = impl_id.resolver(db.upcast());
    lower_types(db, &resolver, &impl_data.type_ref_map)
}

pub mod diagnostics {
    use crate::{
        diagnostics::{DiagnosticSink, PrivateAccess, UnresolvedType},
        type_ref::{LocalTypeRefId, TypeRefSourceMap},
        FileId, HirDatabase,
    };

    #[derive(Debug, PartialEq, Eq, Clone)]
    pub(crate) enum LowerDiagnostic {
        UnresolvedType { id: LocalTypeRefId },
        TypeIsPrivate { id: LocalTypeRefId },
    }

    impl LowerDiagnostic {
        pub(crate) fn add_to(
            &self,
            _db: &dyn HirDatabase,
            file_id: FileId,
            source_map: &TypeRefSourceMap,
            sink: &mut DiagnosticSink<'_>,
        ) {
            match self {
                LowerDiagnostic::UnresolvedType { id } => sink.push(UnresolvedType {
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
