use mun::run_with_args;
use mun_runtime::Runtime;
use std::ffi::OsString;
use std::path::Path;

const PROJECT_DIR: &str = "mun_projects";
const PROJECT_NAME: &str = "mun_example_project";

/// Creates a new project using `mun init` and then tests that it works.
#[test]
fn mun_init() {
    let project = tempfile::Builder::new()
        .prefix(PROJECT_NAME)
        .tempdir()
        .unwrap();

    let args: Vec<OsString> = vec!["mun".into(), "init".into(), project.path().into()];
    assert_eq!(run_with_args(args).unwrap(), mun::ExitStatus::Success);
    build_and_run(project.path());
}

/// Creates a new project using `mun new` and then tests that it works.
#[test]
fn mun_new() {
    let project = tempfile::Builder::new()
        .prefix(PROJECT_DIR)
        .tempdir()
        .unwrap();

    let project_path = project.as_ref().join(PROJECT_NAME);
    let args: Vec<OsString> = vec!["mun".into(), "new".into(), project_path.as_path().into()];
    assert_eq!(run_with_args(args).unwrap(), mun::ExitStatus::Success);
    build_and_run(&project_path);
}

/// Verifies that a newly created project can be used to emit IR.
#[test]
fn mun_emit_ir() {
    let project_dir = tempfile::Builder::new()
        .prefix(PROJECT_DIR)
        .tempdir()
        .unwrap();

    let project_path = project_dir.path().join(PROJECT_NAME);

    let args: Vec<OsString> = vec!["mun".into(), "new".into(), project_path.as_path().into()];
    assert_eq!(run_with_args(args).unwrap(), mun::ExitStatus::Success);
    assert!(project_path.exists());

    build(&project_path, &["--emit-ir"]);

    let ir_path = project_path.join("target/mod.ll");
    assert!(ir_path.is_file());
}

fn build(project: &Path, args: &[&str]) {
    let args: Vec<OsString> = vec![
        OsString::from("mun"),
        OsString::from("build"),
        OsString::from("--manifest-path"),
        OsString::from(project.join("mun.toml")),
    ]
    .into_iter()
    .chain(args.iter().map(|&arg| arg.into()))
    .collect();
    assert_eq!(run_with_args(args).unwrap(), mun::ExitStatus::Success);
}

/// Builds and runs an newly generated mun project
fn build_and_run(project: &Path) {
    build(project, &[]);

    let library_path = project.join("target/mod.munlib");
    assert!(library_path.is_file());

    // Safety: since we compiled the code ourselves, loading the library should be safe
    let builder = Runtime::builder(&library_path);
    let runtime = unsafe { builder.finish() }.unwrap();
    let result: f64 = runtime.invoke("main", ()).unwrap();
    assert_eq!(result, 3.14159);
}
