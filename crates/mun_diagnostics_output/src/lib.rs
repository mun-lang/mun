//! This library contains the code required to emit compiler diagnostics.

mod display_color;
mod emit;

pub use self::display_color::DisplayColor;
pub use self::emit::{
    emit_diagnostics, emit_diagnostics_to_string, emit_hir_diagnostic, emit_syntax_error,
};
