use crate::conversion::{convert_range, convert_symbol_kind};
use crate::state::LanguageServerSnapshot;
use lsp_types::DocumentSymbol;

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

    // Builds hierarchy from a flat list, in reverse order (so that indices
    // makes sense)
    let document_symbols = {
        let mut acc = Vec::new();
        while let Some((mut node, parent_idx)) = parents.pop() {
            if let Some(children) = &mut node.children {
                children.reverse();
            }
            let parent = match parent_idx {
                None => &mut acc,
                Some(i) => parents[i].0.children.get_or_insert_with(Vec::new),
            };
            parent.push(node);
        }
        acc.reverse();
        acc
    };

    Ok(Some(document_symbols.into()))
}
