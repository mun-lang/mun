use super::{Function, Package, Struct, TypeAlias};
use crate::{
    ids::{ItemDefinitionId, ModuleId},
    primitive_type::PrimitiveType,
    DiagnosticSink, FileId, HirDatabase, Name,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct Module {
    pub(crate) id: ModuleId,
}

impl From<ModuleId> for Module {
    fn from(id: ModuleId) -> Self {
        Module { id }
    }
}

impl Module {
    /// Returns the package associated with this module
    pub fn package(self) -> Package {
        Package {
            id: self.id.package,
        }
    }

    /// Returns the module that corresponds to the given file
    pub fn from_file(db: &dyn HirDatabase, file: FileId) -> Option<Module> {
        Package::all(db)
            .iter()
            .flat_map(|package| package.modules(db))
            .find(|m| m.file_id(db) == Some(file))
    }

    /// Returns the parent module of this module.
    pub fn parent(self, db: &dyn HirDatabase) -> Option<Module> {
        let module_tree = db.module_tree(self.id.package);
        let parent_id = module_tree[self.id.local_id].parent?;
        Some(Module {
            id: ModuleId {
                package: self.id.package,
                local_id: parent_id,
            },
        })
    }

    /// Returns the name of this module or None if this is the root module
    pub fn name(self, db: &dyn HirDatabase) -> Option<Name> {
        let module_tree = db.module_tree(self.id.package);
        let parent = module_tree[self.id.local_id].parent?;
        module_tree[parent]
            .children
            .iter()
            .find_map(|(name, module_id)| {
                if *module_id == self.id.local_id {
                    Some(name.clone())
                } else {
                    None
                }
            })
    }

    /// Returns the file that defines the module
    pub fn file_id(self, db: &dyn HirDatabase) -> Option<FileId> {
        db.module_tree(self.id.package).modules[self.id.local_id].file
    }

    /// Returns all items declared in this module.
    pub fn declarations(self, db: &dyn HirDatabase) -> Vec<ModuleDef> {
        let package_defs = db.package_defs(self.id.package);
        package_defs.modules[self.id.local_id]
            .declarations()
            .map(ModuleDef::from)
            .collect()
    }

    /// Iterate over all diagnostics from this `Module` by placing them in the
    /// `sink`
    pub fn diagnostics(self, db: &dyn HirDatabase, sink: &mut DiagnosticSink<'_>) {
        // Add diagnostics from the package definitions
        let package_defs = db.package_defs(self.id.package);
        package_defs.add_diagnostics(db.upcast(), self.id.local_id, sink);

        // Add diagnostics from impls
        let inherent_impls = db.inherent_impls_in_package(self.id.package);
        inherent_impls.add_module_diagnostics(db, self.id.local_id, sink);

        // Add diagnostics from the item tree
        if let Some(file_id) = self.file_id(db) {
            let item_tree = db.item_tree(file_id);
            for diagnostics in item_tree.diagnostics.iter() {
                diagnostics.add_to(db, &item_tree, sink);
            }
        }

        // Add diagnostics from the items
        for decl in self.declarations(db) {
            match decl {
                ModuleDef::Function(f) => f.diagnostics(db, sink),
                ModuleDef::Struct(s) => s.diagnostics(db, sink),
                ModuleDef::TypeAlias(t) => t.diagnostics(db, sink),
                _ => (),
            }
        }
    }

    /// Returns all the child modules of this module
    pub fn children(self, db: &dyn HirDatabase) -> Vec<Module> {
        let module_tree = db.module_tree(self.id.package);
        module_tree[self.id.local_id]
            .children
            .values()
            .map(|local_id| Module {
                id: ModuleId {
                    package: self.id.package,
                    local_id: *local_id,
                },
            })
            .collect()
    }

    /// Returns the path from this module to the root module
    pub fn path_to_root(self, db: &dyn HirDatabase) -> Vec<Module> {
        let mut res = vec![self];
        let mut curr = self;
        while let Some(next) = curr.parent(db) {
            res.push(next);
            curr = next;
        }
        res
    }

    /// Returns the name of this module including all parent modules
    pub fn full_name(self, db: &dyn HirDatabase) -> String {
        itertools::Itertools::intersperse(
            self.path_to_root(db)
                .iter()
                .filter_map(|&module| module.name(db))
                .map(|name| name.to_string()),
            String::from("::"),
        )
        .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModuleDef {
    Module(Module),
    Function(Function),
    PrimitiveType(PrimitiveType),
    Struct(Struct),
    TypeAlias(TypeAlias),
}

impl From<Function> for ModuleDef {
    fn from(t: Function) -> Self {
        ModuleDef::Function(t)
    }
}

impl From<PrimitiveType> for ModuleDef {
    fn from(t: PrimitiveType) -> Self {
        ModuleDef::PrimitiveType(t)
    }
}

impl From<Struct> for ModuleDef {
    fn from(t: Struct) -> Self {
        ModuleDef::Struct(t)
    }
}

impl From<TypeAlias> for ModuleDef {
    fn from(t: TypeAlias) -> Self {
        ModuleDef::TypeAlias(t)
    }
}

impl From<Module> for ModuleDef {
    fn from(m: Module) -> Self {
        ModuleDef::Module(m)
    }
}

impl From<ItemDefinitionId> for ModuleDef {
    fn from(id: ItemDefinitionId) -> Self {
        match id {
            ItemDefinitionId::ModuleId(id) => Module { id }.into(),
            ItemDefinitionId::FunctionId(id) => Function { id }.into(),
            ItemDefinitionId::StructId(id) => Struct { id }.into(),
            ItemDefinitionId::TypeAliasId(id) => TypeAlias { id }.into(),
            ItemDefinitionId::PrimitiveType(id) => id.into(),
        }
    }
}
