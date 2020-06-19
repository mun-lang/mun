use crate::{Manifest, PackageId};
use semver::Version;
use std::fmt;
use std::path::{Path, PathBuf};

#[derive(Clone, PartialEq, Debug)]
pub struct Package {
    // The manifest of the package
    manifest: Manifest,
    // The location of the manifest which marks the root of the package
    manifest_path: PathBuf,
}

impl Package {
    /// Creates a package from a manifest and its location
    pub fn new(manifest: Manifest, manifest_path: &Path) -> Self {
        Self {
            manifest,
            manifest_path: manifest_path.to_path_buf(),
        }
    }

    /// Creates a package by loading the information from a file
    pub fn from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let path = path.as_ref();
        let manifest = Manifest::from_file(path)?;
        Ok(Self::new(manifest, path))
    }

    /// Returns the manifest
    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    /// Returns the path of the manifest
    pub fn manifest_path(&self) -> &Path {
        &self.manifest_path
    }

    /// Returns the name of the package
    pub fn name(&self) -> &str {
        self.manifest().name()
    }

    /// Returns the `PackageId` object for the package
    pub fn package_id(&self) -> &PackageId {
        self.manifest().package_id()
    }

    /// Returns the root folder of the package
    pub fn root(&self) -> &Path {
        self.manifest_path().parent().unwrap()
    }

    /// Returns the version of the package
    pub fn version(&self) -> &Version {
        self.package_id().version()
    }

    /// Returns the source directory of the package, or None if no such directory exists.
    pub fn source_directory(&self) -> Option<PathBuf> {
        let source_dir = self.root().join("src");
        if source_dir.is_dir() {
            Some(source_dir)
        } else {
            None
        }
    }
}

impl fmt::Display for Package {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.package_id())
    }
}
