use std::{collections::HashMap, fs, path::Path};

use anyhow::anyhow;
use heck::{ToShoutySnakeCase, ToSnakeCase, ToUpperCamelCase};
use tera::{Context, Value};

use crate::{project_root, reformat, update};

pub const GRAMMAR: &str = "crates/mun_syntax/src/grammar.ron";
pub const SYNTAX_KINDS: &str = "crates/mun_syntax/src/syntax_kind/generated.rs.tera";
pub const AST: &str = "crates/mun_syntax/src/ast/generated.rs.tera";

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Overwrite,
    Verify,
}

/// Generates the generated.rs for AST and syntax nodes.
pub fn generate(mode: Mode) -> anyhow::Result<()> {
    let grammar = project_root().join(GRAMMAR);
    let syntax_kinds = project_root().join(SYNTAX_KINDS);
    let ast = project_root().join(AST);
    generate_from_template(&syntax_kinds, &grammar, mode)?;
    generate_from_template(&ast, &grammar, mode)?;
    Ok(())
}

/// Generate file contents from a template
fn generate_from_template(template: &Path, src: &Path, mode: Mode) -> anyhow::Result<()> {
    let file_name = template.file_stem().unwrap().to_str().unwrap();
    let tgt = template.with_file_name(file_name);
    let template = fs::read_to_string(template)?;
    let src: ron::Value = {
        let text = fs::read_to_string(src)?;
        ron::de::from_str(&text)?
    };
    let mut tera = create_tera();
    tera.add_raw_template("_src", &template)
        .map_err(|e| anyhow!("template parsing error: {:?}", e))?;
    let content = tera
        .render("_src", &Context::from_serialize(src)?)
        .map_err(|e| anyhow!("template rendering error: {:?}", e))?;
    let content = reformat(content)?;
    update(&tgt, &content, mode)
}

/// Creates a new `tera::Tera` instance with some default filters and functions
/// used in our templates.
fn create_tera() -> tera::Tera {
    /// Convert value to CamelCase
    fn camel(value: &Value, _: &HashMap<String, Value>) -> tera::Result<Value> {
        Ok(value.as_str().unwrap().to_upper_camel_case().into())
    }

    /// Convert value to `snake_case`
    fn snake(value: &Value, _: &HashMap<String, Value>) -> tera::Result<Value> {
        Ok(value.as_str().unwrap().to_snake_case().into())
    }

    /// Convert value to `SCREAM_CASE`
    fn scream(value: &Value, _: &HashMap<String, Value>) -> tera::Result<Value> {
        Ok(value.as_str().unwrap().to_shouty_snake_case().into())
    }

    let mut res = tera::Tera::default();
    res.register_filter("camel", camel);
    res.register_filter("snake", snake);
    res.register_filter("SCREAM", scream);
    res.register_function("concat", |args: &HashMap<String, Value>| {
        let mut elements = Vec::new();
        for &key in ["a", "b", "c"].iter() {
            let val = match args.get(key) {
                Some(val) => val,
                None => continue,
            };
            let val = val.as_array().unwrap();
            elements.extend(val.iter().cloned());
        }
        Ok(tera::Value::Array(elements))
    });

    res
}
