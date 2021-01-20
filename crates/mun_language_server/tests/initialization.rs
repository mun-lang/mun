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

#[test]
fn test_document_symbols() {
    let server = Project::with_fixture(
        r#"
    //- /mun.toml
    [package]
    name = "foo"
    version = "0.0.0"

    //- /src/mod.mun
    fn main() -> i32 {}
    struct Foo {}
    type Bar = Foo;
    "#,
    )
    .server()
    .wait_until_workspace_is_loaded();

    let symbols = server.send_request::<lsp_types::request::DocumentSymbolRequest>(
        lsp_types::DocumentSymbolParams {
            text_document: server.doc_id("src/mod.mun"),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        },
    );

    insta::assert_debug_snapshot!(symbols);
}
