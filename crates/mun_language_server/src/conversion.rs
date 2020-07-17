use lsp_types::Url;
use mun_syntax::{TextRange, TextUnit};
use std::path::{Component, Path, Prefix};
use std::str::FromStr;

/// Returns a `Url` object from a given path, will lowercase drive letters if present.
/// This will only happen when processing Windows paths.
///
/// When processing non-windows path, this is essentially do the same as `Url::from_file_path`.
pub fn url_from_path_with_drive_lowercasing(path: impl AsRef<Path>) -> anyhow::Result<Url> {
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

pub fn convert_range(
    range: TextRange,
    line_index: &hir::line_index::LineIndex,
) -> lsp_types::Range {
    lsp_types::Range {
        start: convert_unit(range.start(), line_index),
        end: convert_unit(range.end(), line_index),
    }
}

pub fn convert_unit(
    range: TextUnit,
    line_index: &hir::line_index::LineIndex,
) -> lsp_types::Position {
    let line_col = line_index.line_col(range);
    lsp_types::Position {
        line: line_col.line.into(),
        character: line_col.col.into(),
    }
}
