use mun::run_with_args;
use mun_runtime::{invoke_fn, RuntimeBuilder};
use std::ffi::OsString;
use tempdir::TempDir;

const TEST_VAL: i32 = 567;

/// Creates a simple test project in a temporary directory and returns the directory.
fn create_project() -> tempdir::TempDir {
    let project_dir = TempDir::new("mun_project_example").unwrap();
    let project_path = project_dir.path();

    std::fs::write(
        project_path.join("mun.toml"),
        r#"
[package]
name="test"
authors=["Mun Team"]
version="0.1.0"
    "#,
    )
    .unwrap();

    std::fs::create_dir_all(project_path.join("src")).unwrap();

    std::fs::write(
        project_path.join("src/main.mun"),
        format!(
            r#"
pub fn main() -> i32 {{
    {}
}}"#,
            TEST_VAL
        ),
    )
    .unwrap();

    project_dir
}

#[test]
fn build_and_run() {
    pretty_env_logger::env_logger::Builder::from_default_env()
        .is_test(true)
        .init();

    let project = create_project();

    let args: Vec<OsString> = vec![
        "mun".into(),
        "build".into(),
        "--manifest-path".into(),
        project.path().join("mun.toml").into(),
    ];
    assert_eq!(run_with_args(args).unwrap(), mun::ExitStatus::Success);

    let library_path = project.path().join("target/main.munlib");
    assert!(library_path.is_file());

    let runtime = RuntimeBuilder::new(&library_path).spawn().unwrap();
    let result: i32 = invoke_fn!(runtime, "main").unwrap();
    assert_eq!(result, TEST_VAL);
}
