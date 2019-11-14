use mun_errors::Diagnostic;
use mun_hir::{FileId, SourceDatabase};
use std::io;
use termcolor::{Color, ColorSpec, WriteColor};

pub trait Emit {
    fn emit(
        &self,
        writer: &mut impl WriteColor,
        db: &impl SourceDatabase,
        file_id: FileId,
    ) -> io::Result<()>;
}

impl Emit for Diagnostic {
    fn emit(
        &self,
        writer: &mut impl WriteColor,
        db: &impl SourceDatabase,
        file_id: FileId,
    ) -> io::Result<()> {
        let line_index = db.line_index(file_id);
        let text = db.file_text(file_id).to_string();
        let path = db.file_relative_path(file_id);
        let line_col = line_index.line_col(self.loc.offset());
        let line_col_end = line_index.line_col(self.loc.end_offset());

        let header = ColorSpec::new()
            .set_fg(Some(Color::White))
            .set_bold(true)
            .set_intense(true)
            .clone();
        let error = header.clone().set_fg(Some(Color::Red)).clone();
        let snippet_gutter = ColorSpec::new()
            .set_fg(Some(Color::Cyan))
            .set_bold(true)
            .set_intense(true)
            .clone();
        let snippet_text = ColorSpec::new();

        // Write severity name
        writer.set_color(&error)?;
        write!(writer, "error")?;

        // Write diagnostics message
        writer.set_color(&header)?;
        writeln!(writer, ": {}", self.message)?;

        if let Some(snippet) = line_index.line_str(line_col.line, &text) {
            // Determine gutter offset
            let line_str = format!("{}", line_col.line + 1);
            let gutter_indent = " ".to_string().repeat(line_str.len());

            writer.set_color(&snippet_gutter)?;
            write!(writer, "{}-->", gutter_indent)?;
            writer.set_color(&snippet_text)?;
            writeln!(
                writer,
                " {}:{}:{}",
                path.as_str(),
                line_col.line + 1,
                line_col.col
            )?;

            // Snippet
            writer.set_color(&snippet_gutter)?;
            writeln!(writer, "{} |", gutter_indent)?;
            write!(writer, "{} | ", line_str)?;
            writer.set_color(&snippet_text)?;
            writeln!(writer, "{}", snippet)?;

            writer.set_color(&snippet_gutter)?;
            write!(writer, "{} |", gutter_indent)?;
            writer.set_color(&error)?;

            if line_col.line == line_col_end.line {
                // single-line diagnostics
                writeln!(
                    writer,
                    " {}{}",
                    " ".to_string().repeat(line_col.col as usize),
                    "^".to_string()
                        .repeat((line_col_end.col - line_col.col) as usize)
                )?;
            }
        }

        //        // Write the start location
        //        println!("  {} {}:{}:{}",
        //            "-->".cyan(),
        //            path.as_str(),
        //            line_col.line + 1,
        //            line_col.col);

        writer.reset()?;
        Ok(())
    }
}
