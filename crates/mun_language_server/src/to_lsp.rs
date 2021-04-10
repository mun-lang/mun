use crate::completion::{CompletionItem, CompletionItemKind};
use crate::state::LanguageServerSnapshot;
use crate::symbol_kind::SymbolKind;
use lsp_types::Url;
use mun_syntax::{TextRange, TextSize};
use std::{
    path::{Component, Path, Prefix},
    str::FromStr,
};

/// Returns a `Url` object from a given path, will lowercase drive letters if present.
/// This will only happen when processing Windows paths.
///
/// When processing non-windows path, this is essentially do the same as `Url::from_file_path`.
fn url_from_path_with_drive_lowercasing(path: impl AsRef<Path>) -> anyhow::Result<Url> {
    let component_has_windows_drive = path.as_ref().components().any(|comp| {
        if let Component::Prefix(c) = comp {
            match c.kind() {
                Prefix::Disk(_) | Prefix::VerbatimDisk(_) => return true,
                _ => return false,
            }
        }
        false
    });

    // VSCode expects drive letters to be lowercased, whereas rust will uppercase the drive letters.
    if component_has_windows_drive {
        let url_original = Url::from_file_path(&path).map_err(|_| {
            anyhow::anyhow!("can't convert path to url: {}", path.as_ref().display())
        })?;

        let drive_partition: Vec<&str> = url_original.as_str().rsplitn(2, ':').collect();

        // There is a drive partition, but we never found a colon.
        // This should not happen, but in this case we just pass it through.
        if drive_partition.len() == 1 {
            return Ok(url_original);
        }

        let joined = drive_partition[1].to_ascii_lowercase() + ":" + drive_partition[0];
        let url = Url::from_str(&joined).expect("This came from a valid `Url`");

        Ok(url)
    } else {
        Ok(Url::from_file_path(&path).map_err(|_| {
            anyhow::anyhow!("can't convert path to url: {}", path.as_ref().display())
        })?)
    }
}

pub(crate) fn range(range: TextRange, line_index: &hir::line_index::LineIndex) -> lsp_types::Range {
    lsp_types::Range {
        start: position(range.start(), line_index),
        end: position(range.end(), line_index),
    }
}

pub(crate) fn position(
    range: TextSize,
    line_index: &hir::line_index::LineIndex,
) -> lsp_types::Position {
    let line_col = line_index.line_col(range);
    lsp_types::Position {
        line: line_col.line,
        character: line_col.col_utf16,
    }
}

/// Converts a symbol kind from this crate to one for the LSP protocol.
pub(crate) fn symbol_kind(symbol_kind: SymbolKind) -> lsp_types::SymbolKind {
    match symbol_kind {
        SymbolKind::Function => lsp_types::SymbolKind::Function,
        SymbolKind::Struct => lsp_types::SymbolKind::Struct,
        SymbolKind::TypeAlias => lsp_types::SymbolKind::TypeParameter,
        SymbolKind::Field => lsp_types::SymbolKind::Field,
        SymbolKind::Local => lsp_types::SymbolKind::Variable,
        SymbolKind::Module => lsp_types::SymbolKind::Module,
    }
}

/// Returns the `Url` associated with the specified `FileId`.
pub(crate) fn url(snapshot: &LanguageServerSnapshot, file_id: hir::FileId) -> anyhow::Result<Url> {
    let vfs = snapshot.vfs.read();
    let path = vfs.file_path(vfs::FileId(file_id.0));
    let url = url_from_path_with_drive_lowercasing(path)?;
    Ok(url)
}

/// Converts from our `CompletionItem` to an LSP `CompletionItem`
pub(crate) fn completion_item(completion_item: CompletionItem) -> lsp_types::CompletionItem {
    lsp_types::CompletionItem {
        label: completion_item.label,
        kind: completion_item.kind.map(completion_item_kind),
        detail: completion_item.detail,
        documentation: None,
        deprecated: None,
        preselect: None,
        sort_text: None,
        filter_text: None,
        insert_text: None,
        insert_text_format: None,
        insert_text_mode: None,
        text_edit: None,
        additional_text_edits: None,
        command: None,
        data: None,
        tags: None,
    }
}

pub(crate) fn completion_item_kind(
    completion_item_kind: CompletionItemKind,
) -> lsp_types::CompletionItemKind {
    match completion_item_kind {
        CompletionItemKind::Binding => lsp_types::CompletionItemKind::Variable,
        CompletionItemKind::BuiltinType => lsp_types::CompletionItemKind::Struct,
        CompletionItemKind::Keyword => lsp_types::CompletionItemKind::Keyword,
        CompletionItemKind::Method => lsp_types::CompletionItemKind::Method,
        CompletionItemKind::Snippet => lsp_types::CompletionItemKind::Snippet,
        CompletionItemKind::UnresolvedReference => lsp_types::CompletionItemKind::Reference,
        CompletionItemKind::SymbolKind(symbol) => match symbol {
            SymbolKind::Field => lsp_types::CompletionItemKind::Field,
            SymbolKind::Function => lsp_types::CompletionItemKind::Function,
            SymbolKind::Local => lsp_types::CompletionItemKind::Variable,
            SymbolKind::Module => lsp_types::CompletionItemKind::Module,
            SymbolKind::Struct => lsp_types::CompletionItemKind::Struct,
            SymbolKind::TypeAlias => lsp_types::CompletionItemKind::Struct,
        },
        CompletionItemKind::Attribute => lsp_types::CompletionItemKind::EnumMember,
    }
}
