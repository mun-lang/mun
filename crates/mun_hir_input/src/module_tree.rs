use std::sync::Arc;

use itertools::Itertools;
use la_arena::{Arena, Idx};
use mun_paths::RelativePath;
use rustc_hash::FxHashMap;

use self::diagnostics::ModuleTreeDiagnostic;
use crate::{FileId, PackageId, SourceDatabase};

/// Represents the tree of modules of a package.
///
/// The [`ModuleTree`] is built by looking at all the source files of the source
/// root of a package and creating a tree based on their relative paths. See the
/// [`ModuleTree::module_tree_query`] method. When constructing the
/// [`ModuleTree`] extra empty modules may be added for missing files. For
/// instance for the relative path `foo/bar/baz.mun`, besides the module
/// `foo::bar::baz` the modules `foo`, `foo::bar` get created along the way.
///
/// A [`ModuleTree`] represent the inner connections between files. It can be
/// used to query the shortest path for use declarations
#[derive(Debug, PartialEq, Eq)]
pub struct ModuleTree {
    pub root: PackageModuleId,
    pub modules: Arena<ModuleData>,
    pub package: PackageId,

    pub diagnostics: Vec<ModuleTreeDiagnostic>,
}

/// A module in the tree of modules
#[derive(Default, Debug, PartialEq, Eq)]
pub struct ModuleData {
    pub parent: Option<PackageModuleId>,
    pub children: FxHashMap<String, PackageModuleId>,
    pub file: Option<FileId>,
}

/// The ID of a module within a specific package
pub type PackageModuleId = Idx<ModuleData>;

// Using a `LocalModuleId` you can access the `ModuleTree` to get the
// `ModuleData`
impl std::ops::Index<PackageModuleId> for ModuleTree {
    type Output = ModuleData;
    fn index(&self, id: PackageModuleId) -> &ModuleData {
        &self.modules[id]
    }
}

impl ModuleTree {
    /// Constructs the tree of modules from the set of files in a package
    pub(crate) fn module_tree_query(
        db: &dyn SourceDatabase,
        package: PackageId,
    ) -> Arc<ModuleTree> {
        use diagnostics::ModuleTreeDiagnostic::DuplicateModuleFile;

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
            // Iterate over all segments of the relative path and construct modules on the
            // way
            let mut module_id = root;
            for path_segment in path_to_module_path(relative_path) {
                module_id = if let Some(id) = modules[module_id].children.get(&path_segment) {
                    *id
                } else {
                    let child_module_id = modules.alloc(ModuleData {
                        parent: Some(module_id),
                        children: FxHashMap::default(),
                        file: None,
                    });

                    if !is_valid_module_name(&path_segment) {
                        diagnostics.push(ModuleTreeDiagnostic::InvalidModuleName(child_module_id));
                    }

                    modules[module_id]
                        .children
                        .insert(path_segment, child_module_id);

                    child_module_id
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
            root,
            modules,
            package,
            diagnostics,
        })
    }

    /// Returns the module that is defined by the specified `file`
    pub fn module_for_file(&self, file: FileId) -> Option<PackageModuleId> {
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
        .map(str::to_lowercase)
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
    use super::PackageModuleId;
    use crate::FileId;

    #[derive(Debug, PartialEq, Eq)]
    pub enum ModuleTreeDiagnostic {
        DuplicateModuleFile(PackageModuleId, Vec<FileId>),
        InvalidModuleName(PackageModuleId),
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{SourceDatabaseStorage, WithFixture};

    #[test]
    fn valid_module_name() {
        assert!(is_valid_module_name("foo"));
        assert!(is_valid_module_name("bar"));
        assert!(is_valid_module_name("foo_bar"));
        assert!(!is_valid_module_name("3bar"));
        assert!(is_valid_module_name("bar3"));
        assert!(!is_valid_module_name("foo-bar"));
        assert!(!is_valid_module_name(""));
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

    /// A mock implementation of the IR database. It can be used to set up a
    /// simple test case.
    #[salsa::database(SourceDatabaseStorage)]
    #[derive(Default)]
    struct MockDatabase {
        storage: salsa::Storage<Self>,
    }

    impl salsa::Database for MockDatabase {}

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
