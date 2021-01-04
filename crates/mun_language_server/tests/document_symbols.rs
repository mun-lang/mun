mod support;

#[test]
fn test_document_symbols() {
    let mut server = support::Project::with_fixture(
        r#"
    //- /Mun.toml
    [package]
    name = "foo"
    version = "0.0.0"

    //- /src/mod.mun
    fn main() -> i32 {}

    struct Foo {}

    type Bar = Foo;
    "#,
    )
    .server();

    let response = async_std::task::block_on(
        server.send_request::<lsp_types::request::DocumentSymbolRequest>(
            lsp_types::DocumentSymbolParams {
                text_document: server.doc_id("src/mod.mun"),
                work_done_progress_params: Default::default(),
                partial_result_params: Default::default(),
            },
        ),
    );

    insta::assert_debug_snapshot!(response);
}
