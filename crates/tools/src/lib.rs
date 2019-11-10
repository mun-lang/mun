pub type Result<T> = std::result::Result<T, failure::Error>;

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
pub use teraron::{Mode, Overwrite, Verify};

pub const GRAMMAR: &str = "crates/mun_syntax/src/grammar.ron";
pub const SYNTAX_KINDS: &str = "crates/mun_syntax/src/syntax_kind/generated.rs.tera";
pub const AST: &str = "crates/mun_syntax/src/ast/generated.rs.tera";

pub fn generate_all(mode: Mode) -> Result<()> {
    let grammar = project_root().join(GRAMMAR);
    let syntax_kinds = project_root().join(SYNTAX_KINDS);
    let ast = project_root().join(AST);
    generate(&syntax_kinds, &grammar, mode)?;
    generate(&ast, &grammar, mode)?;
    Ok(())
}

pub fn generate(template: &Path, src: &Path, mode: Mode) -> Result<()> {
    let file_name = template.file_stem().unwrap().to_str().unwrap();
    let tgt = template.with_file_name(file_name);
    let template = fs::read_to_string(template)?;
    let src: ron::Value = {
        let text = fs::read_to_string(src)?;
        ron::de::from_str(&text)?
    };
    let content = teraron::render(&template, src)?;
    let content = reformat(content)?;
    update(&tgt, &content, mode)
}

/// A helper to update file on disk if it has changed.
/// With verify = false,
fn update(path: &Path, contents: &str, mode: Mode) -> Result<()> {
    match fs::read_to_string(path) {
        Ok(ref old_contents)
            if old_contents.replace("\r\n", "\n") == contents.replace("\r\n", "\n") =>
        {
            return Ok(());
        }
        _ => (),
    }
    if mode == Mode::Verify {
        failure::bail!("`{}` is not up-to-date", path.display());
    }
    eprintln!("updating {}", path.display());
    fs::write(path, contents)?;
    Ok(())
}

fn reformat(text: impl std::fmt::Display) -> Result<String> {
    let mut rustfmt = Command::new("rustfmt")
        //.arg("--config-path")
        //.arg(project_root().join("rustfmt.toml"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    write!(rustfmt.stdin.take().unwrap(), "{}", text)?;
    let output = rustfmt.wait_with_output()?;
    let stdout = String::from_utf8(output.stdout)?;
    let preamble = "Generated file, do not edit by hand, see `crate/ra_tools/src/codegen`";
    Ok(format!("//! {}\n\n{}", preamble, stdout))
}

pub fn project_root() -> PathBuf {
    Path::new(&env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf()
}

#[cfg(test)]
mod tests {
    use crate::Mode;

    #[test]
    fn grammar_is_fresh() {
        if let Err(error) = super::generate_all(Mode::Verify) {
            panic!("{}. Please update it by running `cargo gen-syntax`", error);
        }
    }
}
