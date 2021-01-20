use crate::{
    conversion::{convert_range, convert_symbol_kind},
    state::LanguageServerSnapshot,
};
use lsp_types::DocumentSymbol;

/// Computes the document symbols for a specific document. Converts the LSP types to internal
/// formats and calls [`LanguageServerSnapshot::file_structure`] to fetch the symbols in the
/// requested document. Once completed, returns the result converted back to LSP types.
pub(crate) fn handle_document_symbol(
    snapshot: LanguageServerSnapshot,
    params: lsp_types::DocumentSymbolParams,
) -> anyhow::Result<Option<lsp_types::DocumentSymbolResponse>> {
    let file_id = snapshot.uri_to_file_id(&params.text_document.uri)?;
    let line_index = snapshot.analysis.file_line_index(file_id)?;

    let mut parents: Vec<(DocumentSymbol, Option<usize>)> = Vec::new();

    for symbol in snapshot.analysis.file_structure(file_id)? {
        #[allow(deprecated)]
        let doc_symbol = DocumentSymbol {
            name: symbol.label,
            detail: symbol.detail,
            kind: convert_symbol_kind(symbol.kind),
            tags: None,
            deprecated: None,
            range: convert_range(symbol.node_range, &line_index),
            selection_range: convert_range(symbol.navigation_range, &line_index),
            children: None,
        };

        parents.push((doc_symbol, symbol.parent));
    }

    Ok(Some(build_hierarchy_from_flat_list(parents).into()))
}

/// Constructs a hierarchy of DocumentSymbols for a list of symbols that specify which index is the
/// parent of a symbol. The parent index must always be smaller than the current index.
fn build_hierarchy_from_flat_list(
    mut symbols_and_parent: Vec<(DocumentSymbol, Option<usize>)>,
) -> Vec<DocumentSymbol> {
    let mut result = Vec::new();

    // Iterate over all elements in the list from back to front.
    while let Some((mut node, parent_index)) = symbols_and_parent.pop() {
        // If this node has children (added by the code below), they are in the reverse order. This
        // is because we iterate the input from back to front.
        if let Some(children) = &mut node.children {
            children.reverse();
        }

        // Get the parent index of the current node.
        let parent = match parent_index {
            // If the parent doesnt have a node, directly use the result vector (its a root).
            None => &mut result,

            // If there is a parent, get a reference to the children vector of that parent.
            Some(i) => symbols_and_parent[i]
                .0
                .children
                .get_or_insert_with(Vec::new),
        };

        parent.push(node);
    }

    // The items where pushed in the reverse order, so reverse it right back
    result.reverse();
    result
}

#[cfg(test)]
mod tests {
    use crate::handlers::build_hierarchy_from_flat_list;
    use lsp_types::{DocumentSymbol, SymbolKind};

    #[test]
    fn test_build_hierarchy_from_flat_list() {
        #[allow(deprecated)]
        let default_symbol = DocumentSymbol {
            name: "".to_string(),
            detail: None,
            kind: SymbolKind::File,
            tags: None,
            deprecated: None,
            range: Default::default(),
            selection_range: Default::default(),
            children: None,
        };

        let mut list = Vec::new();

        list.push((
            DocumentSymbol {
                name: "a".to_string(),
                ..default_symbol.clone()
            },
            None,
        ));

        list.push((
            DocumentSymbol {
                name: "b".to_string(),
                ..default_symbol.clone()
            },
            Some(0),
        ));

        list.push((
            DocumentSymbol {
                name: "c".to_string(),
                ..default_symbol.clone()
            },
            Some(0),
        ));

        list.push((
            DocumentSymbol {
                name: "d".to_string(),
                ..default_symbol.clone()
            },
            Some(1),
        ));

        assert_eq!(
            build_hierarchy_from_flat_list(list),
            vec![DocumentSymbol {
                name: "a".to_string(),
                children: Some(vec![
                    DocumentSymbol {
                        name: "b".to_string(),
                        children: Some(vec![DocumentSymbol {
                            name: "d".to_string(),
                            ..default_symbol.clone()
                        }]),
                        ..default_symbol.clone()
                    },
                    DocumentSymbol {
                        name: "c".to_string(),
                        ..default_symbol.clone()
                    }
                ]),
                ..default_symbol.clone()
            }]
        )
    }
}
