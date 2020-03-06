pub(crate) use self::diagnostics::LowerDiagnostic;
use crate::adt::StructKind;
use crate::arena::map::ArenaMap;
use crate::buildin_type::BuiltinType;
use crate::diagnostics::DiagnosticSink;
use crate::name_resolution::Namespace;
use crate::resolve::{Resolution, Resolver};
use crate::ty::{FnSig, Ty, TypeCtor};
use crate::type_ref::{TypeRef, TypeRefId, TypeRefMap, TypeRefSourceMap};
use crate::{FileId, Function, HirDatabase, ModuleDef, Path, Struct};
use std::ops::Index;
use std::sync::Arc;

#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct LowerResult {
    pub(crate) ty: Ty,
    pub(crate) diagnostics: Vec<LowerDiagnostic>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LowerBatchResult {
    pub(crate) type_ref_to_type: ArenaMap<TypeRefId, Ty>,
    pub(crate) diagnostics: Vec<LowerDiagnostic>,
}

impl Index<TypeRefId> for LowerBatchResult {
    type Output = Ty;
    fn index(&self, expr: TypeRefId) -> &Ty {
        self.type_ref_to_type.get(expr).unwrap_or(&Ty::Unknown)
    }
}

impl LowerBatchResult {
    /// Adds all the `LowerDiagnostic`s of the result to the `DiagnosticSink`.
    pub(crate) fn add_diagnostics(
        &self,
        db: &impl HirDatabase,
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
        db: &impl HirDatabase,
        resolver: &Resolver,
        type_ref_map: &TypeRefMap,
        type_ref: TypeRefId,
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
        type_ref: TypeRefId,
    ) -> Ty {
        let res = match &type_ref_map[type_ref] {
            TypeRef::Path(path) => Ty::from_hir_path(db, resolver, path),
            TypeRef::Error => Some(Ty::Unknown),
            TypeRef::Empty => Some(Ty::Empty),
            TypeRef::Never => Some(Ty::simple(TypeCtor::Never)),
        };
        if let Some(ty) = res {
            ty
        } else {
            diagnostics.push(LowerDiagnostic::UnresolvedType { id: type_ref });
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

pub fn types_from_hir(
    db: &impl HirDatabase,
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

pub fn lower_struct_query(db: &impl HirDatabase, s: Struct) -> Arc<LowerBatchResult> {
    let data = s.data(db);
    types_from_hir(db, &s.resolver(db), data.type_ref_map())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TypableDef {
    Function(Function),
    BuiltinType(BuiltinType),
    Struct(Struct),
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

impl From<Struct> for TypableDef {
    fn from(f: Struct) -> Self {
        TypableDef::Struct(f)
    }
}

impl From<ModuleDef> for Option<TypableDef> {
    fn from(d: ModuleDef) -> Self {
        match d {
            ModuleDef::Function(f) => Some(TypableDef::Function(f)),
            ModuleDef::BuiltinType(t) => Some(TypableDef::BuiltinType(t)),
            ModuleDef::Struct(t) => Some(TypableDef::Struct(t)),
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
        match self {
            CallableDef::Function(_) => true,
            _ => false,
        }
    }

    pub fn is_struct(self) -> bool {
        match self {
            CallableDef::Struct(_) => true,
            _ => false,
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
        (TypableDef::Struct(s), Namespace::Values) => type_for_struct_constructor(db, s),
        (TypableDef::Struct(s), Namespace::Types) => type_for_struct(db, s),

        // 'error' cases:
        (TypableDef::Function(_), Namespace::Types) => Ty::Unknown,
        (TypableDef::BuiltinType(_), Namespace::Values) => Ty::Unknown,
    }
}

/// Build the declared type of a static.
fn type_for_builtin(def: BuiltinType) -> Ty {
    Ty::simple(match def {
        BuiltinType::Float(f) => TypeCtor::Float(f.into()),
        BuiltinType::Int(i) => TypeCtor::Int(i.into()),
        BuiltinType::Bool => TypeCtor::Bool,
    })
}

/// Build the declared type of a function. This should not need to look at the
/// function body.
fn type_for_fn(_db: &impl HirDatabase, def: Function) -> Ty {
    Ty::simple(TypeCtor::FnDef(def.into()))
}

pub(crate) fn callable_item_sig(db: &impl HirDatabase, def: CallableDef) -> FnSig {
    match def {
        CallableDef::Function(f) => fn_sig_for_fn(db, f),
        CallableDef::Struct(s) => fn_sig_for_struct_constructor(db, s),
    }
}

pub(crate) fn fn_sig_for_fn(db: &impl HirDatabase, def: Function) -> FnSig {
    let data = def.data(db);
    let resolver = def.resolver(db);
    let params = data
        .params()
        .iter()
        .map(|tr| Ty::from_hir(db, &resolver, data.type_ref_map(), *tr).ty)
        .collect::<Vec<_>>();
    let ret = Ty::from_hir(db, &resolver, data.type_ref_map(), *data.ret_type()).ty;
    FnSig::from_params_and_return(params, ret)
}

pub(crate) fn fn_sig_for_struct_constructor(db: &impl HirDatabase, def: Struct) -> FnSig {
    let data = def.data(db);
    let resolver = def.resolver(db);
    let params = data
        .fields
        .iter()
        .map(|(_, field)| Ty::from_hir(db, &resolver, data.type_ref_map(), field.type_ref).ty)
        .collect::<Vec<_>>();
    let ret = type_for_struct(db, def);
    FnSig::from_params_and_return(params, ret)
}

/// Build the type of a struct constructor.
fn type_for_struct_constructor(db: &impl HirDatabase, def: Struct) -> Ty {
    let struct_data = db.struct_data(def.id);
    if struct_data.kind == StructKind::Tuple {
        Ty::simple(TypeCtor::FnDef(def.into()))
    } else {
        type_for_struct(db, def)
    }
}

fn type_for_struct(_db: &impl HirDatabase, def: Struct) -> Ty {
    Ty::simple(TypeCtor::Struct(def))
}

pub mod diagnostics {
    use crate::diagnostics::UnresolvedType;
    use crate::{
        diagnostics::DiagnosticSink,
        type_ref::{TypeRefId, TypeRefSourceMap},
        FileId, HirDatabase,
    };

    #[derive(Debug, PartialEq, Eq, Clone)]
    pub(crate) enum LowerDiagnostic {
        UnresolvedType { id: TypeRefId },
    }

    impl LowerDiagnostic {
        pub(crate) fn add_to(
            &self,
            _db: &impl HirDatabase,
            file_id: FileId,
            source_map: &TypeRefSourceMap,
            sink: &mut DiagnosticSink,
        ) {
            match self {
                LowerDiagnostic::UnresolvedType { id } => sink.push(UnresolvedType {
                    file: file_id,
                    type_ref: source_map.type_ref_syntax(*id).unwrap(),
                }),
            }
        }
    }
}
