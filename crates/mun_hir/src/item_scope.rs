use crate::module_tree::LocalModuleId;
use crate::primitive_type::PrimitiveType;
use crate::{ids::ItemDefinitionId, visibility::Visibility, Name, PerNs};
use once_cell::sync::Lazy;
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::hash_map::Entry;

/// Defines the type of import. An import can either be a named import (e.g. `use foo::Bar`) or a
/// wildcard import (e.g. `use foo::*`)
#[derive(Copy, Clone)]
pub(crate) enum ImportType {
    /// A wildcard import statement (`use foo::*`)
    Glob,

    /// A named import statement (`use foo::Bar`)
    Named,
}

/// A struct that holds information on which name was imported via a glob import. This information
/// is used by the `PackageDef` collector to keep track of duplicates so that this doesnt result in
/// a duplicate name error:
/// ```mun
/// use foo::{Foo, *};
/// ```
#[derive(Debug, Default)]
pub struct PerNsGlobImports {
    types: FxHashSet<(LocalModuleId, Name)>,
    values: FxHashSet<(LocalModuleId, Name)>,
}

/// Holds all items that are visible from an item as well as by which name and under which
/// visibility they are accessible.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct ItemScope {
    /// All types visible in this scope
    types: FxHashMap<Name, (ItemDefinitionId, Visibility)>,

    /// All values visible in this scope
    values: FxHashMap<Name, (ItemDefinitionId, Visibility)>,

    /// All items that are defined in this scope
    defs: Vec<ItemDefinitionId>,
}

/// A struct that is returned from `add_resolution_from_import`.
#[derive(Debug)]
pub(crate) struct AddResolutionFromImportResult {
    /// Whether or not adding the resolution changed the item scope
    pub changed: bool,

    /// Whether or not adding the resolution will overwrite and existing entry
    pub duplicate: bool,
}

pub(crate) static BUILTIN_SCOPE: Lazy<FxHashMap<Name, PerNs<(ItemDefinitionId, Visibility)>>> =
    Lazy::new(|| {
        PrimitiveType::ALL
            .iter()
            .map(|(name, ty)| {
                (
                    name.clone(),
                    PerNs::types((ty.clone().into(), Visibility::Public)),
                )
            })
            .collect()
    });

impl ItemScope {
    /// Returns all the entries in the scope
    pub fn entries<'a>(
        &'a self,
    ) -> impl Iterator<Item = (&'a Name, PerNs<(ItemDefinitionId, Visibility)>)> + 'a {
        let keys: FxHashSet<_> = self.types.keys().chain(self.values.keys()).collect();
        keys.into_iter().map(move |name| (name, self.get(name)))
    }

    /// Returns an iterator over all declarations with this scope
    pub fn declarations(&self) -> impl Iterator<Item = ItemDefinitionId> + '_ {
        self.defs.iter().copied()
    }

    /// Adds an item definition to the list of definitions
    pub(crate) fn add_definition(&mut self, def: ItemDefinitionId) {
        self.defs.push(def)
    }

    /// Adds a named item resolution into the scope. Returns true if adding the resolution changes
    /// the scope or not.
    pub(crate) fn add_resolution(
        &mut self,
        name: Name,
        def: PerNs<(ItemDefinitionId, Visibility)>,
    ) -> bool {
        let mut changed = false;
        if let Some((types, visibility)) = def.types {
            self.types.entry(name.clone()).or_insert_with(|| {
                changed = true;
                (types, visibility)
            });
        }
        if let Some((values, visibility)) = def.values {
            self.values.entry(name).or_insert_with(|| {
                changed = true;
                (values, visibility)
            });
        }

        changed
    }

    /// Adds a named item resolution into the scope which is the result of a `use` statement.
    /// Returns true if adding the resolution changes the scope or not.
    pub(crate) fn add_resolution_from_import(
        &mut self,
        glob_imports: &mut PerNsGlobImports,
        lookup: (LocalModuleId, Name),
        def: PerNs<(ItemDefinitionId, Visibility)>,
        def_import_type: ImportType,
    ) -> AddResolutionFromImportResult {
        let mut changed = false;
        let mut duplicate = false;

        macro_rules! check_changed {
            (
                $changed:ident,
                ( $this:ident / $def:ident ) . $field:ident,
                $glob_imports:ident [ $lookup:ident ],
                $def_import_type:ident
            ) => {{
                let existing = $this.$field.entry($lookup.1.clone());
                match (existing, $def.$field) {
                    (Entry::Vacant(entry), Some(_)) => {
                        match $def_import_type {
                            ImportType::Glob => {
                                $glob_imports.$field.insert($lookup.clone());
                            }
                            ImportType::Named => {
                                $glob_imports.$field.remove(&$lookup);
                            }
                        }

                        if let Some(fld) = $def.$field {
                            entry.insert(fld);
                        }
                        $changed = true;
                    }
                    // If there is already an entry for this resolution, but it came from a glob
                    // pattern, overwrite it and mark it as not included from the glob pattern.
                    (Entry::Occupied(mut entry), Some(_))
                        if $glob_imports.$field.contains(&$lookup)
                            && matches!($def_import_type, ImportType::Named) =>
                    {
                        $glob_imports.$field.remove(&$lookup);
                        if let Some(fld) = $def.$field {
                            entry.insert(fld);
                        }
                        $changed = true;
                    }
                    (Entry::Occupied(_), Some(_)) => {
                        let is_previous_from_glob = $glob_imports.$field.contains(&$lookup);
                        let is_explicit_import = matches!($def_import_type, ImportType::Named);
                        if is_explicit_import && !is_previous_from_glob {
                            duplicate = true;
                        }
                    }
                    _ => {}
                }
            }};
        }

        check_changed!(
            changed,
            (self / def).types,
            glob_imports[lookup],
            def_import_type
        );
        check_changed!(
            changed,
            (self / def).values,
            glob_imports[lookup],
            def_import_type
        );

        AddResolutionFromImportResult { changed, duplicate }
    }

    /// Gets a name from the current module scope
    pub(crate) fn get(&self, name: &Name) -> PerNs<(ItemDefinitionId, Visibility)> {
        PerNs {
            types: self.types.get(name).copied(),
            values: self.values.get(name).copied(),
        }
    }
}

impl PerNs<(ItemDefinitionId, Visibility)> {
    pub(crate) fn from_definition(
        def: ItemDefinitionId,
        vis: Visibility,
        has_constructor: bool,
    ) -> PerNs<(ItemDefinitionId, Visibility)> {
        match def {
            ItemDefinitionId::FunctionId(_) => PerNs::values((def, vis)),
            ItemDefinitionId::StructId(_) => {
                if has_constructor {
                    PerNs::both((def, vis), (def, vis))
                } else {
                    PerNs::types((def, vis))
                }
            }
            ItemDefinitionId::TypeAliasId(_) => PerNs::types((def, vis)),
            ItemDefinitionId::PrimitiveType(_) => PerNs::types((def, vis)),
            ItemDefinitionId::ModuleId(_) => PerNs::types((def, vis)),
        }
    }
}
