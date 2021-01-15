//! This modules contains several helper functions to convert from types defined in the Language
//! Server Protocol to our own datatypes.

use crate::state::LanguageServerSnapshot;
use hir::line_index::LineIndex;
use lsp_types::Url;
use mun_syntax::{TextRange, TextSize};
use paths::AbsPathBuf;
use std::convert::TryFrom;

/// Converts the specified `uri` to an absolute path. Returns an error if the url could not be
/// converted to an absolute path.
pub(crate) fn abs_path(uri: &Url) -> anyhow::Result<AbsPathBuf> {
    uri.to_file_path()
        .ok()
        .and_then(|path| AbsPathBuf::try_from(path).ok())
        .ok_or_else(|| anyhow::anyhow!("invalid uri: {}", uri))
}

/// Returns the `hir::FileId` associated with the given `Url`
pub(crate) fn file_id(
    snapshot: &LanguageServerSnapshot,
    url: &lsp_types::Url,
) -> anyhow::Result<hir::FileId> {
    abs_path(url).and_then(|path| {
        snapshot
            .vfs
            .read()
            .file_id(&path)
            .ok_or_else(|| anyhow::anyhow!("url does not refer to a file: {}", url))
            .map(|id| hir::FileId(id.0))
    })
}

/// Converts the specified `offset` to our own `TextSize` structure
pub(crate) fn offset(line_index: &LineIndex, position: lsp_types::Position) -> TextSize {
    let line_col = hir::line_index::LineCol {
        line: position.line as u32,
        col_utf16: position.character as u32,
    };
    line_index.offset(line_col)
}

/// Converts the given lsp range to a `TextRange`. This requires a `LineIndex` to convert lines to
/// offsets.
pub(crate) fn text_range(line_index: &LineIndex, range: lsp_types::Range) -> TextRange {
    let start = offset(line_index, range.start);
    let end = offset(line_index, range.end);
    TextRange::new(start, end)
}
