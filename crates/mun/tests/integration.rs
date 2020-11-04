use mun::run_with_args;
use mun_runtime::{invoke_fn, RuntimeBuilder};
use serial_test::serial;
use std::env::set_current_dir;
use std::ffi::OsString;
use std::path::Path;

/// Creates a new project using `mun init` and then tests that it works.
#[test]
#[serial] // This test must be run in serial as files may conflict.
fn mun_init() {
    let project = tempfile::Builder::new()
        .prefix("mun_project_example")
        .tempdir()
        .unwrap();

    set_current_dir(&project).unwrap();

    let args: Vec<OsString> = vec!["mun".into(), "init".into()];
    assert_eq!(run_with_args(args).unwrap(), mun::ExitStatus::Success);
    build_and_run(project);
}

/// Creates a new project using `mun new` and then tests that it works.
#[test]
#[serial] // This test must be run in serial as files may conflict.
fn mun_new() {
    let project = tempfile::Builder::new()
        .prefix("mun_projects")
        .tempdir()
        .unwrap();

    set_current_dir(&project).unwrap();

    let args: Vec<OsString> = vec!["mun".into(), "new".into(), "mun_project_example".into()];
    assert_eq!(run_with_args(args).unwrap(), mun::ExitStatus::Success);
    dbg!(project.as_ref().join("mun_project_example").ancestors());
    build_and_run(project.as_ref().join("mun_project_example"));
}

/// Builds and runs an newly generated mun project
fn build_and_run(project: impl AsRef<Path>) {
    let args: Vec<OsString> = vec![
        "mun".into(),
        "build".into(),
        "--manifest-path".into(),
        project.as_ref().join("mun.toml").into(),
    ];
    assert_eq!(run_with_args(args).unwrap(), mun::ExitStatus::Success);

    let library_path = project.as_ref().join("target/mod.munlib");
    assert!(library_path.is_file());

    let runtime = RuntimeBuilder::new(&library_path).spawn().unwrap();
    let runtime_ref = runtime.borrow();
    let result: f64 = invoke_fn!(runtime_ref, "main").unwrap();
    assert_eq!(result, 3.14159);
}
