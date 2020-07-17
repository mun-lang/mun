use crate::db::{DefDatabase, SourceDatabase};
use crate::mock::MockDatabase;
use std::sync::Arc;

/// This function tests that the ModuleData of a module does not change if the contents of a function
/// is changed.
#[test]
fn check_module_data_does_not_change() {
    let (mut db, file_id) = MockDatabase::with_single_file(
        r#"
    fn foo()->i32 {
        1+1
    }
    "#,
    );

    {
        let events = db.log_executed(|| {
            db.module_data(file_id);
        });
        assert!(
            format!("{:?}", events).contains("module_data"),
            "{:#?}",
            events
        )
    }
    db.set_file_text(
        file_id,
        Arc::new(
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
            db.module_data(file_id);
        });
        assert!(
            !format!("{:?}", events).contains("module_data"),
            "{:#?}",
            events
        )
    }
}
