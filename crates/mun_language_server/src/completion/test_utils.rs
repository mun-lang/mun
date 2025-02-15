use itertools::Itertools;

use crate::{
    change_fixture::{ChangeFixture, RangeOrOffset},
    completion::{CompletionItem, CompletionItemKind},
    db::AnalysisDatabase,
    FilePosition,
};

/// Creates an analysis database from a multi-file fixture and a position marked
/// with `$0`.
pub(crate) fn position(fixture: &str) -> (AnalysisDatabase, FilePosition) {
    let change_fixture = ChangeFixture::parse(fixture);
    let mut database = AnalysisDatabase::default();
    database.apply_change(change_fixture.change);
    let (file_id, range_or_offset) = change_fixture
        .file_position
        .expect("expected a marker ($0)");
    let offset = match range_or_offset {
        RangeOrOffset::Range(_) => panic!(),
        RangeOrOffset::Offset(it) => it,
    };
    (database, FilePosition { file_id, offset })
}

/// Creates a list of completions for the specified code. The code must contain
/// a cursor in the text indicated by `$0`
pub(crate) fn completion_list(code: &str) -> Vec<CompletionItem> {
    let (db, position) = position(code);
    let completions = super::completions(&db, position).unwrap();
    completions
        .buf
        .into_iter()
        .filter(|item| item.kind != CompletionItemKind::BuiltinType)
        .sorted_by_key(|it| (it.kind, it.label.clone()))
        .collect()
}

/// Constructs a string representation of all the completions for the specified
/// code. The code must contain a cursor in the text indicated by `$0`.
pub(crate) fn completion_string(code: &str) -> String {
    let completions = completion_list(code);
    completions_to_string(completions)
}

/// Similar to [`completion_string`] but the items are sorted by relevance.
pub(crate) fn completion_relevance_string(code: &str) -> String {
    let completions = completion_list(code)
        .into_iter()
        .sorted_by_key(|it| it.relevance.score())
        .rev()
        .collect();
    completions_to_string(completions)
}

fn completions_to_string(completions: Vec<CompletionItem>) -> String {
    let label_width = completions
        .iter()
        .map(|it| it.label.chars().count())
        .max()
        .unwrap_or_default()
        .min(16);
    itertools::Itertools::intersperse(
        completions.into_iter().map(|item| {
            let mut result = format!("{} {}", item.kind.tag(), &item.label);
            if let Some(detail) = item.detail {
                result = format!("{:width$} {}", result, detail, width = label_width + 3);
            }
            result
        }),
        String::from("\n"),
    )
    .collect()
}
