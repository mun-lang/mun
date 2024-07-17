use std::ops::Index;

use rustc_hash::FxHashMap;

use crate::SourceRootId;

/// Represents the id of a single package, all packages have a unique id, the
/// main package and all dependent packages.
///
/// The [`PackageId`] is used to uniquely identify a package in the project. To
/// gain access to the data of a package, the [`PackageSet`] is used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PackageId(pub u32);

/// Top-level information for a single package in the project.
///
/// Packages are usually represented by a [`PackageId`]. The [`PackageSet`]
/// stores a mapping from [`PackageId`] to [`PackageData`].
#[derive(Debug, Clone)]
pub struct PackageData {
    /// The source root which groups together all the source files of a package.
    pub source_root: SourceRootId,
}

/// Contains information about all the packages in the project.
#[derive(Debug, Clone, Default)]
pub struct PackageSet {
    arena: FxHashMap<PackageId, PackageData>,
}

impl PackageSet {
    /// Returns an empty package set
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a new package to the package set with the source files located add
    /// the specified root. Returns the `PackageId` associated with the package.
    pub fn add_package(&mut self, source_root: SourceRootId) -> PackageId {
        let data = PackageData { source_root };
        let package_id = PackageId(self.arena.len() as u32);
        self.arena.insert(package_id, data);
        package_id
    }

    /// Iterates over all packages
    pub fn iter(&self) -> impl Iterator<Item = PackageId> + '_ {
        self.arena.keys().copied()
    }
}

impl Index<PackageId> for PackageSet {
    type Output = PackageData;

    fn index(&self, index: PackageId) -> &Self::Output {
        &self.arena[&index]
    }
}
