use crate::MANIFEST_FILENAME;
use anyhow::bail;
use mun_paths::{AbsPath, AbsPathBuf};
use rustc_hash::FxHashSet;
use std::{convert::TryFrom, fs::read_dir, io};

/// A wrapper around a path to a mun project
#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct ProjectManifest {
    pub path: AbsPathBuf,
}

impl ProjectManifest {
    /// Constructs a new [`ProjectManifest`] from a path
    pub fn from_manifest_path(path: impl AsRef<AbsPath>) -> anyhow::Result<Self> {
        let path = path.as_ref();
        if path.ends_with(MANIFEST_FILENAME) {
            Ok(Self {
                path: path.to_path_buf(),
            })
        } else {
            bail!(
                "project root must point to {}: {}",
                MANIFEST_FILENAME,
                path.display()
            );
        }
    }

    /// Find all project manifests in the given directory
    pub fn discover(path: impl AsRef<AbsPath>) -> io::Result<Vec<ProjectManifest>> {
        Ok(read_dir(path.as_ref())?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.is_file()
                    && path
                        .file_name()
                        .map(|file_name| file_name == MANIFEST_FILENAME)
                        .unwrap_or(false)
            })
            .map(|path| ProjectManifest {
                path: AbsPathBuf::try_from(path).expect(
                    "read_dir does not return absolute path when iterating an absolute path",
                ),
            })
            .collect())
    }

    /// Find all project manifests in a collection of paths
    pub fn discover_all(paths: impl Iterator<Item = impl AsRef<AbsPath>>) -> Vec<ProjectManifest> {
        let mut project_manifests = paths
            .filter_map(|path| ProjectManifest::discover(path).ok())
            .flatten()
            .collect::<FxHashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        project_manifests.sort();
        project_manifests
    }
}
