use crate::line_index::{LineCol, LineIndex};
use colored::*;
use mun_errors::Diagnostic;
use std::fmt;

pub trait Emit {
    fn emit(&self, line_index: &LineIndex);
}

impl Emit for Diagnostic {
    fn emit(&self, line_index: &LineIndex) {
        let line_col = line_index.line_col(self.loc.offset());
        println!("{} ({}:{}): {}",
                 "error".red(),
                 line_col.line + 1,
                 line_col.col,
                 self.message);
    }
}