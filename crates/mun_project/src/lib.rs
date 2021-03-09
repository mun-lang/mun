pub use manifest::{Manifest, ManifestMetadata, PackageId};
pub use package::Package;
pub use project_manifest::ProjectManifest;

mod manifest;
mod package;
mod project_manifest;

pub const MANIFEST_FILENAME: &str = "mun.toml";
pub const LOCKFILE_NAME: &str = ".munlock";
