pub type Result<T> = std::result::Result<T, failure::Error>;

use std::path::{Path, PathBuf};
pub use teraron::{Mode, Overwrite, Verify};

pub const GRAMMAR: &str = "crates/mun_syntax/src/grammar.ron";
pub const SYNTAX_KINDS: &str = "crates/mun_syntax/src/syntax_kind/generated.rs.tera";
pub const AST: &str = "crates/mun_syntax/src/ast/generated.rs.tera";

pub fn generate(mode: Mode) -> Result<()> {
    let grammar = project_root().join(GRAMMAR);
    let syntax_kinds = project_root().join(SYNTAX_KINDS);
    let ast = project_root().join(AST);
    teraron::generate(&syntax_kinds, &grammar, mode)?;
    teraron::generate(&ast, &grammar, mode)?;
    Ok(())
}

pub fn project_root() -> PathBuf {
    Path::new(&env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf()
}
