use crate::{
    arena::{Arena, Idx},
    ids::ModuleId,
    module_tree::diagnostics::ModuleTreeDiagnostic,
    visibility::RawVisibility,
    DefDatabase, FileId, Name, PackageId, SourceDatabase, Visibility,
};
use itertools::Itertools;
use paths::RelativePath;
use rustc_hash::FxHashMap;
use std::sync::Arc;

/// Represents the tree of modules in a package.
#[derive(Debug, PartialEq, Eq)]
pub struct ModuleTree {
    pub root: LocalModuleId,
    pub modules: Arena<ModuleData>,
    pub package: PackageId,

    pub diagnostics: Vec<diagnostics::ModuleTreeDiagnostic>,
}

/// A module in the tree of modules
#[derive(Default, Debug, PartialEq, Eq)]
pub struct ModuleData {
    pub parent: Option<LocalModuleId>,
    pub children: FxHashMap<Name, LocalModuleId>,
    pub file: Option<FileId>,
}

/// The ID of a module within a specific package
pub(crate) type LocalModuleId = Idx<ModuleData>;

// Using a `LocalModuleId` you can access the `ModuleTree` to get the `ModuleData`
impl std::ops::Index<LocalModuleId> for ModuleTree {
    type Output = ModuleData;
    fn index(&self, id: LocalModuleId) -> &ModuleData {
        &self.modules[id]
    }
}

impl ModuleTree {
    /// Constructs the tree of modules from the set of files in a package
    pub(crate) fn module_tree_query(
        db: &dyn SourceDatabase,
        package: PackageId,
    ) -> Arc<ModuleTree> {
        use diagnostics::ModuleTreeDiagnostic::*;

        let mut diagnostics = Vec::new();

        // Get the sources for the package
        let source_root_id = db.packages().as_ref()[package].source_root;
        let source_root = db.source_root(source_root_id);

        let mut modules = Arena::default();
        let root = modules.alloc(ModuleData::default());

        // Iterate over all files and add them to the module tree
        for (file_id, relative_path) in source_root
            .files()
            .map(|file_id| (file_id, source_root.relative_path(file_id)))
            .sorted_by(|(_, a), (_, b)| a.cmp(b))
        {
            // Iterate over all segments of the relative path and construct modules on the way
            let mut module_id = root;
            for path_segment in path_to_module_path(&relative_path)
                .into_iter()
                .map(Name::new)
            {
                module_id = match modules[module_id].children.get(&path_segment) {
                    Some(id) => *id,
                    None => {
                        let child_module_id = modules.alloc(ModuleData {
                            parent: Some(module_id),
                            children: Default::default(),
                            file: None,
                        });

                        if !is_valid_module_name(path_segment.to_string()) {
                            diagnostics
                                .push(ModuleTreeDiagnostic::InvalidModuleName(child_module_id))
                        }

                        modules[module_id]
                            .children
                            .insert(path_segment, child_module_id);

                        child_module_id
                    }
                };
            }

            // Mark the found module with the current file id
            let module = &mut modules[module_id];
            if let Some(other_file) = module.file {
                diagnostics.push(DuplicateModuleFile(module_id, vec![other_file, file_id]));
            }

            module.file = Some(file_id);
        }

        Arc::new(ModuleTree {
            package,
            diagnostics,
            modules,
            root,
        })
    }

    /// Given a `RawVisibility` which describes the visibility of an item relative to a module into
    /// a `Visibility` which describes the absolute visibility within the module tree.
    pub(crate) fn resolve_visibility(
        &self,
        _db: &dyn DefDatabase,
        original_module: LocalModuleId,
        visibility: &RawVisibility,
    ) -> Visibility {
        match visibility {
            RawVisibility::This => Visibility::Module(ModuleId {
                package: self.package,
                local_id: original_module,
            }),
            RawVisibility::Super => {
                let parent_module_id = self[original_module].parent.unwrap_or(original_module);
                Visibility::Module(ModuleId {
                    package: self.package,
                    local_id: parent_module_id,
                })
            }
            RawVisibility::Package => Visibility::Module(ModuleId {
                package: self.package,
                local_id: self.root,
            }),
            RawVisibility::Public => Visibility::Public,
        }
    }

    /// Returns the module that is defined by the specified `file`
    pub fn module_for_file(&self, file: FileId) -> Option<LocalModuleId> {
        self.modules.iter().find_map(|(idx, data)| {
            if data.file == Some(file) {
                Some(idx)
            } else {
                None
            }
        })
    }
}

/// Given a relative path, returns a Vec with all the module names
fn path_to_module_path(path: &RelativePath) -> Vec<String> {
    if path.extension().is_none() {
        path.components().map(|c| c.as_str().to_owned()).collect()
    } else if path
        .file_stem()
        .map(|stem| stem.to_lowercase())
        .expect("the file has an extension so it must also have a file stem")
        == "mod"
    {
        // The parent directory is the module path
        path_to_module_path(
            path.parent()
                .expect("path has a filename so it must also have a parent"),
        )
    } else {
        // Simply strip the extension and use that as the module path
        path_to_module_path(&path.with_extension(""))
    }
}

/// Given a module name returns true if it is a valid name
fn is_valid_module_name(name: impl AsRef<str>) -> bool {
    let mut chars = name.as_ref().chars();
    if let Some(first_char) = chars.next() {
        first_char.is_alphabetic() && chars.all(|c| c.is_alphanumeric() || c == '_')
    } else {
        false
    }
}

mod diagnostics {
    use super::LocalModuleId;
    use crate::FileId;

    #[derive(Debug, PartialEq, Eq)]
    pub enum ModuleTreeDiagnostic {
        DuplicateModuleFile(LocalModuleId, Vec<FileId>),
        InvalidModuleName(LocalModuleId),
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{fixture::WithFixture, mock::MockDatabase};

    #[test]
    fn valid_module_name() {
        assert_eq!(is_valid_module_name("foo"), true);
        assert_eq!(is_valid_module_name("bar"), true);
        assert_eq!(is_valid_module_name("foo_bar"), true);
        assert_eq!(is_valid_module_name("3bar"), false);
        assert_eq!(is_valid_module_name("bar3"), true);
        assert_eq!(is_valid_module_name("foo-bar"), false);
        assert_eq!(is_valid_module_name(""), false);
    }

    #[test]
    fn module_path() {
        assert_eq!(
            path_to_module_path(RelativePath::new("foo/bar/baz.mun")),
            vec!["foo", "bar", "baz"]
        );
        assert_eq!(
            path_to_module_path(RelativePath::new("foo/bar/mod.mun")),
            vec!["foo", "bar"]
        );
        assert_eq!(
            path_to_module_path(RelativePath::new("foo/mod.mun")),
            vec!["foo"]
        );
        assert_eq!(
            path_to_module_path(RelativePath::new("foo.mun")),
            vec!["foo"]
        );
        assert_eq!(
            path_to_module_path(RelativePath::new("mod.mun")),
            Vec::<String>::new()
        );
    }

    #[test]
    fn module_tree() {
        let mock_db = MockDatabase::with_files(
            r#"
        //- /mod.mun
        //- /foo.mun
        //- /foo/mod.mun
        //- /foo/bar.mun
        //- /foo/baz/mod.mun
        //- /baz/foo.mun
        "#,
        );
        let module_tree = mock_db.module_tree(PackageId(0));
        insta::assert_debug_snapshot!(module_tree);
    }
}
