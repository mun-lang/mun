mod manifest;
mod package;

pub use manifest::{Manifest, ManifestMetadata, PackageId};
pub use package::Package;

pub const MANIFEST_FILENAME: &str = "mun.toml";
