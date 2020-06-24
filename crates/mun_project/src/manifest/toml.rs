use super::{Manifest, ManifestMetadata, PackageId};
use serde_derive::{Deserialize, Serialize};

/// A manifest as specified in a mun.toml file.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct TomlManifest {
    package: TomlProject,
}

/// Represents the `package` section of a mun.toml file.
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct TomlProject {
    name: String,
    version: semver::Version,
    authors: Option<Vec<String>>,
}

impl TomlManifest {
    /// Convert this toml manifest into a "real" manifest.
    pub fn into_real_manifest(self) -> Result<Manifest, anyhow::Error> {
        let name = self.package.name.trim();
        if name.is_empty() {
            anyhow::bail!("package name cannot be an empty string");
        }

        Ok(Manifest {
            package_id: PackageId {
                name: name.to_owned(),
                version: self.package.version,
            },
            metadata: ManifestMetadata {
                authors: self.package.authors.unwrap_or_default(),
            },
        })
    }
}
