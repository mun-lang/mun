//! This crate represents all the input of a mun project.

mod db;
mod fixture;
mod line_index;
mod module_tree;
mod package_set;
mod source_root;

pub use db::{SourceDatabase, SourceDatabaseStorage};
pub use fixture::{Fixture, WithFixture};
pub use line_index::{LineCol, LineIndex};
pub use module_tree::{ModuleData, ModuleTree, PackageModuleId};
pub use package_set::{PackageData, PackageId, PackageSet};
pub use source_root::{SourceRoot, SourceRootId};

/// [`FileId`] is an integer which uniquely identifies a file. File paths are
/// messy and system-dependent, so most of the code should work directly with
/// [`FileId`], without inspecting the path.
///
/// [`FileId`]s are logically grouped by a [`SourceRoot`].
///
/// The mapping between [`FileId`] and path and [`SourceRoot`] is constant. A
/// file rename is represented as a pair of deletion/creation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FileId(pub u32);

/// A identifier that unique identifies a single module with the project.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct ModuleId {
    /// The id of the package that contains the module.
    pub package: PackageId,

    /// The id of the module inside the package.
    pub local_id: PackageModuleId,
}
