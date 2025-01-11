use itertools::Itertools;
use lsp_types::{DocumentSymbolResponse, PartialResultParams, WorkDoneProgressParams};
use text_trees::FormatCharacters;

use crate::Project;

#[test]
fn test_document_symbols() {
    let server = Project::with_fixture(
        r#"
    //- /mun.toml
    [package]
    name = "foo"
    version = "0.0.0"

    //- /src/mod.mun
    struct Foo {
        a: i32,
    }
    type Bar = Foo;

    impl Foo {
        fn new() -> Self {}

        fn modify(self) -> Self {
            self
        }
    }

    impl Foo {
        fn modify2(self) -> Self {
            self
        }
    }

    fn main() -> i32 {}
    "#,
    )
    .server()
    .wait_until_workspace_is_loaded();

    let symbols = server.send_request::<lsp_types::request::DocumentSymbolRequest>(
        lsp_types::DocumentSymbolParams {
            text_document: server.doc_id("src/mod.mun"),
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        },
    );

    insta::assert_snapshot!(format_document_symbols_response(symbols));
}

fn format_document_symbols_response(response: Option<DocumentSymbolResponse>) -> String {
    let Some(response) = response else {
        return "received empty response".to_string();
    };

    let nodes = match response {
        DocumentSymbolResponse::Flat(_symbols) => {
            unimplemented!("Flat document symbols are not supported")
        }
        DocumentSymbolResponse::Nested(symbols) => symbols
            .iter()
            .map(format_document_symbol)
            .collect::<Vec<_>>(),
    };

    let formatting = text_trees::TreeFormatting::dir_tree(FormatCharacters::ascii());
    format!(
        "{}",
        nodes
            .into_iter()
            .map(|node| node.to_string_with_format(&formatting).unwrap())
            .format("")
    )
}

fn format_document_symbol(symbol: &lsp_types::DocumentSymbol) -> text_trees::StringTreeNode {
    text_trees::StringTreeNode::with_child_nodes(
        format!(
            "{}{}",
            symbol.name,
            symbol
                .detail
                .as_ref()
                .map_or_else(String::new, |s| format!(" ({})", s))
        ),
        symbol.children.iter().flatten().map(format_document_symbol),
    )
}
