use std::fmt;
use std::path::Path;
use std::str::FromStr;

mod toml;

/// Contains all information of a package. Usually this information is read from a mun.toml file.
#[derive(PartialEq, Clone, Debug)]
pub struct Manifest {
    package_id: PackageId,
    metadata: ManifestMetadata,
}

/// General metadata for a package.
#[derive(PartialEq, Clone, Debug)]
pub struct ManifestMetadata {
    pub authors: Vec<String>,
}

/// Unique identifier of a package and version
#[derive(PartialEq, Clone, Debug)]
pub struct PackageId {
    name: String,
    version: semver::Version,
}

impl Manifest {
    /// Try to read a manifest from a file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Manifest, anyhow::Error> {
        // Load the contents of the file
        let file_contents = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("could not read manifest file: {}", e))?;
        Self::from_str(&file_contents)
    }

    /// Returns the unique identifier of this manifest based on the name and version
    pub fn package_id(&self) -> &PackageId {
        &self.package_id
    }

    /// Returns the name of the package
    pub fn name(&self) -> &str {
        &self.package_id.name()
    }

    /// Returns the version of the package
    pub fn version(&self) -> &semver::Version {
        &self.package_id.version()
    }

    /// Returns the metadata information of the package
    pub fn metadata(&self) -> &ManifestMetadata {
        &self.metadata
    }
}

impl PackageId {
    /// Returns the name of the package
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the version of the package
    pub fn version(&self) -> &semver::Version {
        &self.version
    }
}

impl fmt::Display for PackageId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} v{}", self.name(), self.version())
    }
}

impl std::str::FromStr for Manifest {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Parse the contents of the file to toml manifest
        let manifest = ::toml::from_str::<toml::TomlManifest>(s)
            .map_err(|e| anyhow::anyhow!("could not parse manifest: {}", e))?;
        manifest.into_real_manifest()
    }
}

#[cfg(test)]
mod tests {
    use crate::Manifest;
    use std::str::FromStr;

    #[test]
    fn parse() {
        let manifest = Manifest::from_str(
            r#"
        [package]
        name="test"
        version="0.2.0"
        authors = ["Mun Team"]
        "#,
        )
        .unwrap();

        assert_eq!(manifest.name(), "test");
        assert_eq!(
            manifest.version(),
            &semver::Version::from_str("0.2.0").unwrap()
        );
        assert_eq!(manifest.metadata().authors, vec!["Mun Team"]);
        assert_eq!(format!("{}", manifest.package_id()), "test v0.2.0");
    }
}
