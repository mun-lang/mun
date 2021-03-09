pub use crate::fixture::Fixture;
use crate::{FileId, PackageSet, SourceDatabase, SourceRoot, SourceRootId};
use std::{convert::TryInto, sync::Arc};

impl<DB: SourceDatabase + Default + 'static> WithFixture for DB {}

/// Enables the creation of an instance from a [`Fixture`]
pub trait WithFixture: Default + SourceDatabase + 'static {
    /// Constructs an instance from a fixture
    fn with_files(fixture: impl AsRef<str>) -> Self {
        let mut db = Self::default();
        with_files(&mut db, fixture.as_ref());
        db
    }

    /// Constructs an instance from a fixture
    fn with_single_file(text: impl AsRef<str>) -> (Self, FileId) {
        let mut db = Self::default();
        let files = with_files(&mut db, text.as_ref());
        assert_eq!(files.len(), 1);
        (db, files[0])
    }
}

/// Fills the specified database with all the files from the specified `fixture`
fn with_files(db: &mut dyn SourceDatabase, fixture: &str) -> Vec<FileId> {
    let fixture = Fixture::parse(fixture);

    let mut source_root = SourceRoot::default();
    let source_root_id = SourceRootId(0);
    let mut files = Vec::new();

    for (idx, entry) in fixture.into_iter().enumerate() {
        let file_id = FileId(idx.try_into().expect("too many files"));
        db.set_file_text(file_id, Arc::from(entry.text));
        db.set_file_source_root(file_id, source_root_id);
        source_root.insert_file(file_id, entry.relative_path);
        files.push(file_id);
    }

    db.set_source_root(source_root_id, Arc::new(source_root));

    let mut packages = PackageSet::default();
    packages.add_package(source_root_id);
    db.set_packages(Arc::new(packages));

    files
}
