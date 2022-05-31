///! A monitor is a trait that reads and monitors files in a given set of directories. Changes are
///! read to memory and communicated.
mod notify_monitor;

pub use notify_monitor::NotifyMonitor;

use crate::{AbsPath, AbsPathBuf};
use std::fmt;

/// Describes something to be monitored by a `Monitor`.
#[derive(Debug, Clone)]
pub enum MonitorEntry {
    /// A set of files
    Files(Vec<AbsPathBuf>),

    /// A dynamic set of files and directories
    Directories(MonitorDirectories),
}

/// Describes a set of files to monitor. A file is included if:
/// * it has included `extension`
/// * it is under an `include` path
/// * it is not under an `exclude` path
///
/// If many include/exclude paths match, the longest one wins.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonitorDirectories {
    /// File extensions to monitor (e.g. "mun")
    pub extensions: Vec<String>,

    /// The directories or files to monitor
    pub include: Vec<AbsPathBuf>,

    /// Paths to ignore
    pub exclude: Vec<AbsPathBuf>,
}

/// Describes the configuration of the monitor. This can be updated with the `set_config` method on
/// a [`Monitor`]
#[derive(Debug, Clone)]
pub struct MonitorConfig {
    /// The set of entries to load
    pub load: Vec<MonitorEntry>,

    /// Indicates which entries in `load` should also continuously be monitored.
    pub watch: Vec<usize>,
}

/// A message that might be communicated from a [`Monitor`]
pub enum MonitorMessage {
    /// A message that indicates the progress status of the monitor
    Progress { total: usize, done: usize },

    /// A message that indicates files has been loaded or modified. If the contents of a file is
    /// `None` it has been removed.
    Loaded {
        files: Vec<(AbsPathBuf, Option<Vec<u8>>)>,
    },
}

pub type Sender = Box<dyn Fn(MonitorMessage) + Send>;

/// A trait to monitor a set of directories and files
/// TODO: In the future it would be nice to do this with a Future (no pun intended).
pub trait Monitor {
    /// Instantiates a new instance of `Self`
    fn new(sender: Sender) -> Self
    where
        Self: Sized;

    /// Updates the configuration of things to monitor.
    fn set_config(&mut self, config: MonitorConfig);

    /// Reload the content of the specified file. This will trigger a new `Loaded` message to be
    /// send.
    fn reload(&mut self, path: &AbsPath);
}

impl MonitorDirectories {
    /// Returns true if, according to this instance, the file at the given `path` is contained in
    /// this set.
    pub fn contains_file(&self, path: impl AsRef<AbsPath>) -> bool {
        let ext = path.as_ref().extension().unwrap_or_default();
        if !self
            .extensions
            .iter()
            .any(|include_ext| include_ext.as_str() == ext)
        {
            false
        } else {
            self.includes_path(path)
        }
    }

    /// Returns true if, according to this instance, the directory at the given `path` is contained
    /// in this set.
    pub fn contains_dir(&self, path: impl AsRef<AbsPath>) -> bool {
        self.includes_path(path)
    }

    /// Returns true if the given path is considered part of this set.
    fn includes_path(&self, path: impl AsRef<AbsPath>) -> bool {
        let path = path.as_ref();

        // Find the include path with the longest path that includes the specified path
        let mut include: Option<&AbsPathBuf> = None;
        for incl in &self.include {
            if path.starts_with(incl) {
                include = Some(match include {
                    Some(prev) if prev.starts_with(incl) => prev,
                    _ => incl,
                })
            }
        }

        // If there is no include path, we're done quickly
        let include = match include {
            Some(incl) => incl,
            None => return false,
        };

        // Filter based on exclude paths
        for excl in &self.exclude {
            if path.starts_with(excl) && excl.starts_with(include) {
                return false;
            }
        }

        true
    }
}

impl MonitorEntry {
    /// Returns true if, according to this instance, the file at the given `path` is contained in
    /// this entry.
    pub fn contains_file(&self, path: impl AsRef<AbsPath>) -> bool {
        match self {
            MonitorEntry::Files(files) => {
                let path = path.as_ref();
                files.iter().any(|entry| entry == path)
            }
            MonitorEntry::Directories(dirs) => dirs.contains_file(path),
        }
    }

    /// Returns true if, according to this instance, the directory at the given `path` is contained
    /// in this set.
    pub fn contains_dir(&self, path: impl AsRef<AbsPath>) -> bool {
        match self {
            MonitorEntry::Files(_) => false,
            MonitorEntry::Directories(dirs) => dirs.contains_dir(path),
        }
    }
}

impl fmt::Debug for MonitorMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MonitorMessage::Loaded { files } => f
                .debug_struct("Loaded")
                .field("files", &files.len())
                .finish(),
            MonitorMessage::Progress { total, done } => f
                .debug_struct("Progress")
                .field("total", total)
                .field("done", done)
                .finish(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AbsPathBuf, Monitor, MonitorDirectories};
    use std::convert::TryInto;
    use std::path::PathBuf;

    #[test]
    fn monitor_is_object_safe() {
        fn _assert(_: &dyn Monitor) {}
    }

    #[test]
    fn test_config() {
        let abs_manifest_dir: AbsPathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .try_into()
            .unwrap();

        let config = MonitorDirectories {
            extensions: vec!["mun".to_owned()],
            include: vec![
                abs_manifest_dir.join("src"),
                abs_manifest_dir.join("src/.git/special_case"),
            ],
            exclude: vec![
                abs_manifest_dir.join(".git"),
                abs_manifest_dir.join("src/.git"),
            ],
        };

        assert!(!config.contains_file(abs_manifest_dir.join("mod.mun")));
        assert!(config.contains_file(abs_manifest_dir.join("src/mod.mun")));
        assert!(!config.contains_file(abs_manifest_dir.join("src/mod.rs")));
        assert!(!config.contains_file(abs_manifest_dir.join(".git/src/mod.mun")));
        assert!(!config.contains_file(abs_manifest_dir.join("src/.git/mod.mun")));
        assert!(config.contains_file(abs_manifest_dir.join("src/.git/special_case/mod.mun")));
        assert!(config.contains_dir(abs_manifest_dir.join("src")));
    }
}
