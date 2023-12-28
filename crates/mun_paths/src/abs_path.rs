use std::borrow::Borrow;
use std::convert::{TryFrom, TryInto};
use std::ops::Deref;
use std::path::{Path, PathBuf};

/// Represents an absolute path, internally simply wraps a `PathBuf`.
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct AbsPathBuf(PathBuf);

impl From<AbsPathBuf> for PathBuf {
    fn from(abs_path_buf: AbsPathBuf) -> Self {
        abs_path_buf.0
    }
}

impl Deref for AbsPathBuf {
    type Target = AbsPath;

    fn deref(&self) -> &Self::Target {
        self.as_path()
    }
}

impl AsRef<Path> for AbsPathBuf {
    fn as_ref(&self) -> &Path {
        self.0.as_path()
    }
}

impl AsRef<PathBuf> for AbsPathBuf {
    fn as_ref(&self) -> &PathBuf {
        &self.0
    }
}

impl AsRef<AbsPath> for AbsPathBuf {
    fn as_ref(&self) -> &AbsPath {
        AbsPath::assert_new(self.0.as_path())
    }
}

impl TryFrom<PathBuf> for AbsPathBuf {
    type Error = PathBuf;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        if path.is_absolute() {
            Ok(AbsPathBuf(path))
        } else {
            Err(path)
        }
    }
}

impl PartialEq<AbsPath> for AbsPathBuf {
    fn eq(&self, other: &AbsPath) -> bool {
        self.as_path() == other
    }
}

impl Borrow<AbsPath> for AbsPathBuf {
    fn borrow(&self) -> &AbsPath {
        self.as_path()
    }
}

impl AbsPathBuf {
    /// Coerces to a [`AbsPath`] slice.
    pub fn as_path(&self) -> &AbsPath {
        AbsPath::assert_new(self.0.as_path())
    }
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
#[repr(transparent)]
pub struct AbsPath(Path);

impl Deref for AbsPath {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<Path> for AbsPath {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

impl<'a> TryFrom<&'a Path> for &'a AbsPath {
    type Error = &'a Path;

    fn try_from(path: &'a Path) -> Result<Self, Self::Error> {
        if path.is_absolute() {
            Ok(AbsPath::assert_new(path))
        } else {
            Err(path)
        }
    }
}

impl AbsPath {
    /// Constructs a new `AbsPath` from a `Path`.
    pub fn assert_new(path: &Path) -> &AbsPath {
        assert!(path.is_absolute());
        // This is a safe operation because `AbsPath` is a transparent wrapper around `Path`
        unsafe { &*(path as *const Path as *const AbsPath) }
    }

    /// Returns the `AbsPath` without its final component, if there is one.
    pub fn parent(&self) -> Option<&AbsPath> {
        self.0.parent().map(AbsPath::assert_new)
    }

    /// Creates an owned [`AbsPathBuf`] with `path` adjoined to `self`.
    pub fn join(&self, path: impl AsRef<Path>) -> AbsPathBuf {
        self.as_ref().join(path).try_into().unwrap()
    }

    /// Converts a `AbsPath` to an owned [`AbsPathBuf`].
    pub fn to_path_buf(&self) -> AbsPathBuf {
        AbsPathBuf::try_from(self.0.to_path_buf()).unwrap()
    }
}
