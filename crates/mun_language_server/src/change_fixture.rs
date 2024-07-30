use std::sync::Arc;

use mun_hir_input::{FileId, Fixture, PackageSet, SourceRoot, SourceRootId};
use mun_syntax::{TextRange, TextSize};

use crate::change::AnalysisChange;

pub const CURSOR_MARKER: &str = "$0";

/// A `ChangeFixture` is an extended [`Fixture`] that can be used to construct
/// an entire [`AnalysisDatabase`] with. It can also optionally contain a cursor
/// indicated by `$0`.
pub struct ChangeFixture {
    pub file_position: Option<(FileId, RangeOrOffset)>,
    pub files: Vec<FileId>,
    pub change: AnalysisChange,
}

impl ChangeFixture {
    pub fn parse(fixture: &str) -> ChangeFixture {
        let fixture = Fixture::parse(fixture);

        let mut change = AnalysisChange::default();
        let mut source_root = SourceRoot::default();
        let mut package_set = PackageSet::default();

        let mut file_id = FileId(0);
        let mut file_position = None;
        let mut files = Vec::new();

        for entry in fixture {
            let text = if entry.text.contains(CURSOR_MARKER) {
                let (range_or_offset, text) = extract_range_or_offset(&entry.text);
                assert!(
                    file_position.is_none(),
                    "cannot have multiple cursor markers"
                );
                file_position = Some((file_id, range_or_offset));
                text.clone()
            } else {
                entry.text.clone()
            };

            change.change_file(file_id, Some(Arc::from(text)));
            source_root.insert_file(file_id, entry.relative_path);
            files.push(file_id);
            file_id.0 += 1;
        }

        package_set.add_package(SourceRootId(0));

        change.set_roots(vec![source_root]);
        change.set_packages(package_set);

        ChangeFixture {
            file_position,
            files,
            change,
        }
    }
}

/// Returns the offset of the first occurrence of `$0` marker and the copy of
/// `text` without the marker.
fn try_extract_offset(text: &str) -> Option<(TextSize, String)> {
    let cursor_pos = text.find(CURSOR_MARKER)?;
    let mut new_text = String::with_capacity(text.len() - CURSOR_MARKER.len());
    new_text.push_str(&text[..cursor_pos]);
    new_text.push_str(&text[cursor_pos + CURSOR_MARKER.len()..]);
    let cursor_pos = TextSize::from(cursor_pos as u32);
    Some((cursor_pos, new_text))
}

/// Returns `TextRange` between the first two markers `$0...$0` and the copy
/// of `text` without both of these markers.
fn try_extract_range(text: &str) -> Option<(TextRange, String)> {
    let (start, text) = try_extract_offset(text)?;
    let (end, text) = try_extract_offset(&text)?;
    Some((TextRange::new(start, end), text))
}

#[derive(Clone, Copy)]
pub enum RangeOrOffset {
    Range(TextRange),
    Offset(TextSize),
}

impl From<RangeOrOffset> for TextRange {
    fn from(selection: RangeOrOffset) -> Self {
        match selection {
            RangeOrOffset::Range(it) => it,
            RangeOrOffset::Offset(it) => TextRange::empty(it),
        }
    }
}

/// Extracts `TextRange` or `TextSize` depending on the amount of `$0` markers
/// found in `text`.
pub fn extract_range_or_offset(text: &str) -> (RangeOrOffset, String) {
    if let Some((range, text)) = try_extract_range(text) {
        (RangeOrOffset::Range(range), text)
    } else if let Some((offset, text)) = try_extract_offset(text) {
        (RangeOrOffset::Offset(offset), text)
    } else {
        panic!("text should contain a cursor marker")
    }
}
