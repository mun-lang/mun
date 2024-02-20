use std::ops::Index;

use rustc_hash::FxHashMap;

use crate::SourceRootId;

/// Information regarding a package
#[derive(Debug, Clone)]
pub struct PackageData {
    /// The source root that holds the source files
    pub source_root: SourceRootId,
}

/// Represents the id of a single package, all packages have a unique id, the
/// main package and all dependent packages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PackageId(pub u32);

/// Represents information about a set of packages in a compilation
#[derive(Debug, Clone, Default)]
pub struct PackageSet {
    arena: FxHashMap<PackageId, PackageData>,
}

impl PackageSet {
    /// Adds a new package to the package set
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
