mod support;

use support::Project;

#[test]
fn test_server() {
    let _server = Project::with_fixture(
        r#"
//- /mun.toml
[package]
name = "foo"
version = "0.0.0"

//- /src/mod.mun
fn add(a: i32, b: i32) -> i32 {
    a + b
}
"#,
    )
    .server()
    .wait_until_workspace_is_loaded();
}
