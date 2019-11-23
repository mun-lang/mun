mod per_ns;

pub use self::per_ns::{Namespace, PerNs};
use crate::{code_model::BuiltinType, FileId, HirDatabase, ModuleDef, Name};
use once_cell::sync::Lazy;
use rustc_hash::FxHashMap;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Resolution {
    /// None for unresolved
    pub def: PerNs<ModuleDef>,
    //    /// ident by which this is imported into local scope.
    //    pub import: Option<ImportId>,
}

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct ModuleScope {
    items: FxHashMap<Name, Resolution>,
}

static BUILTIN_SCOPE: Lazy<FxHashMap<Name, Resolution>> = Lazy::new(|| {
    BuiltinType::ALL
        .iter()
        .map(|(name, ty)| {
            (
                name.clone(),
                Resolution {
                    def: PerNs::types(ty.clone().into()),
                },
            )
        })
        .collect()
});

impl ModuleScope {
    pub fn entries<'a>(&'a self) -> impl Iterator<Item = (&'a Name, &'a Resolution)> + 'a {
        //FIXME: shadowing
        self.items.iter().chain(BUILTIN_SCOPE.iter())
    }
    pub fn get(&self, name: &Name) -> Option<&Resolution> {
        self.items.get(name).or_else(|| BUILTIN_SCOPE.get(name))
    }
}

pub(crate) fn module_scope_query(db: &impl HirDatabase, file_id: FileId) -> Arc<ModuleScope> {
    let mut scope = ModuleScope::default();
    let defs = db.module_data(file_id);
    for def in defs.definitions() {
        #[allow(clippy::single_match)]
        match def {
            ModuleDef::Function(f) => {
                scope.items.insert(
                    f.name(db),
                    Resolution {
                        def: PerNs::values(*def),
                    },
                );
            }
            ModuleDef::Struct(s) => {
                scope.items.insert(
                    s.name(db),
                    Resolution {
                        def: PerNs::types(*def),
                    },
                );
            }
            _ => {}
        }
    }
    Arc::new(scope)
}
