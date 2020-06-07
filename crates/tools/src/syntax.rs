use crate::{project_root, reformat, update, Result};
use anyhow::anyhow;
use std::fs;
use std::path::Path;
use teraron::Mode;

pub const GRAMMAR: &str = "crates/mun_syntax/src/grammar.ron";
pub const SYNTAX_KINDS: &str = "crates/mun_syntax/src/syntax_kind/generated.rs.tera";
pub const AST: &str = "crates/mun_syntax/src/ast/generated.rs.tera";

/// Generates the generated.rs for AST and syntax nodes.
pub fn generate(mode: Mode) -> Result<()> {
    let grammar = project_root().join(GRAMMAR);
    let syntax_kinds = project_root().join(SYNTAX_KINDS);
    let ast = project_root().join(AST);
    generate_from_template(&syntax_kinds, &grammar, mode)?;
    generate_from_template(&ast, &grammar, mode)?;
    Ok(())
}

/// Generate file contents from a template
fn generate_from_template(template: &Path, src: &Path, mode: Mode) -> Result<()> {
    let file_name = template.file_stem().unwrap().to_str().unwrap();
    let tgt = template.with_file_name(file_name);
    let template = fs::read_to_string(template)?;
    let src: ron::Value = {
        let text = fs::read_to_string(src)?;
        ron::de::from_str(&text)?
    };
    let content = teraron::render(&template, src).map_err(|e| anyhow!("{}", e))?;
    let content = reformat(content)?;
    update(&tgt, &content, mode)
}
