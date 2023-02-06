use crate::{
    db::{DefDatabase, SourceDatabase},
    mock::MockDatabase,
    with_fixture::WithFixture,
    PackageId,
};
use std::sync::Arc;

/// This function tests that the ModuleData of a module does not change if the contents of a function
/// is changed.
#[test]
fn check_package_defs_does_not_change() {
    let (mut db, file_id) = MockDatabase::with_single_file(
        r#"
    fn foo()->i32 {
        1+1
    }
    "#,
    );

    {
        let events = db.log_executed(|| {
            db.package_defs(PackageId(0));
        });
        assert!(
            format!("{events:?}").contains("package_defs"),
            "{events:#?}"
        )
    }
    db.set_file_text(
        file_id,
        Arc::from(
            r#"
    fn foo()->i32 {
        90
    }
    "#
            .to_owned(),
        ),
    );
    {
        let events = db.log_executed(|| {
            db.package_defs(PackageId(0));
        });
        assert!(
            !format!("{events:?}").contains("package_defs"),
            "{events:#?}"
        )
    }
}
