use crate::{
    code_model::src::HasSource,
    ids::{DefWithBodyId, FunctionId, ItemDefinitionId, Lookup, StructId, TypeAliasId},
    item_scope::ItemScope,
    DefDatabase, FileId, HirDatabase, InFile, ModuleId,
};
use mun_syntax::{ast, match_ast, AstNode, SyntaxNode};
use rustc_hash::FxHashMap;

pub(super) type SourceToDefCache = FxHashMap<SourceToDefContainer, SourceToDefMap>;

/// An object that can be used to efficiently find definitions of source objects. It is used to
/// find HIR elements for corresponding AST elements.
pub(super) struct SourceToDefContext<'a, 'db> {
    pub(super) db: &'db dyn HirDatabase,
    pub(super) cache: &'a mut SourceToDefCache,
}

impl SourceToDefContext<'_, '_> {
    /// Find the container for the given syntax tree node.
    pub(super) fn find_container(
        &mut self,
        src: InFile<&SyntaxNode>,
    ) -> Option<SourceToDefContainer> {
        for container in std::iter::successors(Some(src.cloned()), move |node| {
            node.value.parent().map(|parent| node.with_value(parent))
        })
        .skip(1)
        {
            let res: SourceToDefContainer = match_ast! {
                match (container.value) {
                    ast::FunctionDef(it) => {
                        let def = self.fn_to_def(container.with_value(it))?;
                        DefWithBodyId::from(def).into()
                    },
                    _ => continue,
                }
            };
            return Some(res);
        }

        let def = self.file_to_def(src.file_id)?;
        Some(def.into())
    }

    /// Find the `FunctionId` associated with the specified syntax tree node.
    fn fn_to_def(&mut self, src: InFile<ast::FunctionDef>) -> Option<FunctionId> {
        let container = self.find_container(src.as_ref().map(AstNode::syntax))?;
        let db = self.db;
        let def_map = &*self
            .cache
            .entry(container)
            .or_insert_with(|| container.source_to_def_map(db));
        def_map.functions.get(&src).copied()
    }

    /// Finds the `ModuleId` associated with the specified `file`
    fn file_to_def(&self, file_id: FileId) -> Option<ModuleId> {
        let source_root_id = self.db.file_source_root(file_id);
        let packages = self.db.packages();
        let result = packages
            .iter()
            .filter(|package_id| packages[*package_id].source_root == source_root_id)
            .find_map(|package_id| {
                let module_tree = self.db.module_tree(package_id);
                let module_id = module_tree.module_for_file(file_id)?;
                Some(ModuleId {
                    package: package_id,
                    local_id: module_id,
                })
            });
        result
    }
}

/// A container that holds other items.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) enum SourceToDefContainer {
    DefWithBodyId(DefWithBodyId),
    ModuleId(ModuleId),
}

impl From<DefWithBodyId> for SourceToDefContainer {
    fn from(id: DefWithBodyId) -> Self {
        SourceToDefContainer::DefWithBodyId(id)
    }
}

impl From<ModuleId> for SourceToDefContainer {
    fn from(id: ModuleId) -> Self {
        SourceToDefContainer::ModuleId(id)
    }
}

impl SourceToDefContainer {
    fn source_to_def_map(self, db: &dyn HirDatabase) -> SourceToDefMap {
        match self {
            SourceToDefContainer::DefWithBodyId(id) => id.source_to_def_map(db),
            SourceToDefContainer::ModuleId(id) => id.source_to_def_map(db),
        }
    }
}

/// A trait to construct a `SourceToDefMap` from a definition like a module.
trait SourceToDef {
    /// Returns all definitions in `self`.
    fn source_to_def_map(&self, db: &dyn HirDatabase) -> SourceToDefMap;
}

impl SourceToDef for DefWithBodyId {
    fn source_to_def_map(&self, _db: &dyn HirDatabase) -> SourceToDefMap {
        // TODO: bodies dont yet contain items themselves
        SourceToDefMap::default()
    }
}

impl SourceToDef for ModuleId {
    fn source_to_def_map(&self, db: &dyn HirDatabase) -> SourceToDefMap {
        let package_defs = db.package_defs(self.package);
        let module_scope = &package_defs[self.local_id];
        module_scope.source_to_def_map(db)
    }
}

impl SourceToDef for ItemScope {
    fn source_to_def_map(&self, db: &dyn HirDatabase) -> SourceToDefMap {
        fn add_module_def(db: &dyn DefDatabase, map: &mut SourceToDefMap, item: ItemDefinitionId) {
            match item {
                ItemDefinitionId::FunctionId(id) => {
                    let src = id.lookup(db).source(db);
                    map.functions.insert(src, id);
                }
                ItemDefinitionId::StructId(id) => {
                    let src = id.lookup(db).source(db);
                    map.structs.insert(src, id);
                }
                ItemDefinitionId::TypeAliasId(id) => {
                    let src = id.lookup(db).source(db);
                    map.type_aliases.insert(src, id);
                }
                _ => {}
            }
        }

        let mut result = SourceToDefMap::default();
        self.declarations()
            .for_each(|item| add_module_def(db.upcast(), &mut result, item));

        result
    }
}

/// Holds conversion from source location to definitions in the HIR.
#[derive(Default)]
pub(crate) struct SourceToDefMap {
    functions: FxHashMap<InFile<ast::FunctionDef>, FunctionId>,
    structs: FxHashMap<InFile<ast::StructDef>, StructId>,
    type_aliases: FxHashMap<InFile<ast::TypeAliasDef>, TypeAliasId>,
}
