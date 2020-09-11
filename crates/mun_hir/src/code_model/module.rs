use super::{Function, Package, Struct, TypeAlias};
use crate::ids::{ItemDefinitionId, ModuleId};
use crate::primitive_type::PrimitiveType;
use crate::{DiagnosticSink, FileId, HirDatabase};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Module {
    pub(crate) id: ModuleId,
}

impl From<ModuleId> for Module {
    fn from(id: ModuleId) -> Self {
        Module { id }
    }
}

impl Module {
    /// Returns the module that corresponds to the given file
    pub fn from_file(db: &dyn HirDatabase, file: FileId) -> Option<Module> {
        Package::all(db)
            .iter()
            .flat_map(|package| package.modules(db))
            .find(|m| m.file_id(db) == Some(file))
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

    /// Iterate over all diagnostics from this `Module` by placing them in the `sink`
    pub fn diagnostics(self, db: &dyn HirDatabase, sink: &mut DiagnosticSink) {
        if let Some(file_id) = self.file_id(db) {
            let item_tree = db.item_tree(file_id);
            for diagnostics in item_tree.diagnostics.iter() {
                diagnostics.add_to(db, &*item_tree, sink);
            }
        }
        for decl in self.declarations(db) {
            match decl {
                ModuleDef::Function(f) => f.diagnostics(db, sink),
                ModuleDef::Struct(s) => s.diagnostics(db, sink),
                ModuleDef::TypeAlias(t) => t.diagnostics(db, sink),
                _ => (),
            }
        }
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
