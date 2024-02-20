use lsp_types::{CompletionContext, CompletionItem, DocumentSymbol};
use mun_syntax::{AstNode, TextSize};

use crate::{from_lsp, state::LanguageServerSnapshot, to_lsp, FilePosition};

/// Computes the document symbols for a specific document. Converts the LSP
/// types to internal formats and calls
/// [`LanguageServerSnapshot::file_structure`] to fetch the symbols in the
/// requested document. Once completed, returns the result converted back to LSP
/// types.
pub(crate) fn handle_document_symbol(
    snapshot: LanguageServerSnapshot,
    params: lsp_types::DocumentSymbolParams,
) -> anyhow::Result<Option<lsp_types::DocumentSymbolResponse>> {
    let file_id = from_lsp::file_id(&snapshot, &params.text_document.uri)?;
    let line_index = snapshot.analysis.file_line_index(file_id)?;

    let mut parents: Vec<(DocumentSymbol, Option<usize>)> = Vec::new();

    for symbol in snapshot.analysis.file_structure(file_id)? {
        #[allow(deprecated)]
        let doc_symbol = DocumentSymbol {
            name: symbol.label,
            detail: symbol.detail,
            kind: to_lsp::symbol_kind(symbol.kind),
            tags: None,
            deprecated: None,
            range: to_lsp::range(symbol.node_range, &line_index),
            selection_range: to_lsp::range(symbol.navigation_range, &line_index),
            children: None,
        };

        parents.push((doc_symbol, symbol.parent));
    }

    Ok(Some(build_hierarchy_from_flat_list(parents).into()))
}

/// Computes completion items that should be presented to the user when the
/// cursor is at a specific location.
pub(crate) fn handle_completion(
    snapshot: LanguageServerSnapshot,
    params: lsp_types::CompletionParams,
) -> anyhow::Result<Option<lsp_types::CompletionResponse>> {
    /// Helper function to check if the given position is preceded by a single
    /// colon.
    fn is_position_at_single_colon(
        snapshot: &LanguageServerSnapshot,
        position: FilePosition,
        context: Option<CompletionContext>,
    ) -> anyhow::Result<bool> {
        if let Some(ctx) = context {
            if ctx.trigger_character.unwrap_or_default() == ":" {
                let source_file = snapshot.analysis.parse(position.file_id)?;
                let syntax = source_file.syntax();
                let text = syntax.text();
                if let Some(next_char) = text.char_at(position.offset) {
                    let diff = TextSize::of(next_char) + TextSize::of(':');
                    let prev_char = position.offset - diff;
                    if text.char_at(prev_char) != Some(':') {
                        return Ok(true);
                    }
                }
            }
        }
        Ok(false)
    }

    let position = from_lsp::file_position(&snapshot, params.text_document_position)?;

    // If the completion was triggered after a single colon there is nothing to do.
    // We only want completion after a *double* colon (::) or after a dot (.).
    if is_position_at_single_colon(&snapshot, position, params.context)? {
        return Ok(None);
    }

    // Get all completions from the analysis database
    let items = match snapshot.analysis.completions(position)? {
        None => return Ok(None),
        Some(items) => items,
    };

    // Convert all the items to the LSP protocol type
    let items: Vec<CompletionItem> = items.into_iter().map(to_lsp::completion_item).collect();

    Ok(Some(items.into()))
}

/// Constructs a hierarchy of `DocumentSymbols` for a list of symbols that
/// specify which index is the parent of a symbol. The parent index must always
/// be smaller than the current index.
fn build_hierarchy_from_flat_list(
    mut symbols_and_parent: Vec<(DocumentSymbol, Option<usize>)>,
) -> Vec<DocumentSymbol> {
    let mut result = Vec::new();

    // Iterate over all elements in the list from back to front.
    while let Some((mut node, parent_index)) = symbols_and_parent.pop() {
        // If this node has children (added by the code below), they are in the reverse
        // order. This is because we iterate the input from back to front.
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
    use lsp_types::{DocumentSymbol, Range, SymbolKind};

    use crate::handlers::build_hierarchy_from_flat_list;

    #[test]
    fn test_build_hierarchy_from_flat_list() {
        #[allow(deprecated)]
        let default_symbol = DocumentSymbol {
            name: "".to_string(),
            detail: None,
            kind: SymbolKind::FILE,
            tags: None,
            deprecated: None,
            range: Range::default(),
            selection_range: Range::default(),
            children: None,
        };

        let list = vec![
            (
                DocumentSymbol {
                    name: "a".to_string(),
                    ..default_symbol.clone()
                },
                None,
            ),
            (
                DocumentSymbol {
                    name: "b".to_string(),
                    ..default_symbol.clone()
                },
                Some(0),
            ),
            (
                DocumentSymbol {
                    name: "c".to_string(),
                    ..default_symbol.clone()
                },
                Some(0),
            ),
            (
                DocumentSymbol {
                    name: "d".to_string(),
                    ..default_symbol.clone()
                },
                Some(1),
            ),
        ];

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
                ..default_symbol
            }]
        );
    }
}
