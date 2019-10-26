use crate::db::SourceDatabase;
use crate::expr::BodySourceMap;
use crate::ids::LocationCtx;
use crate::mock::MockDatabase;
use crate::{Function, HirDisplay, InferenceResult};
use mun_syntax::{ast, AstNode};
use std::fmt::Write;
use std::sync::Arc;

#[test]
fn infer_basics() {
    infer_snapshot(
        r#"
    fn test(a:int, b:float, c:bool): bool {
        a;
        b;
        c
    }
    "#,
    )
}

fn infer_snapshot(text: &str) {
    let text = text.trim().replace("\n    ", "\n");
    insta::assert_snapshot!(insta::_macro_support::AutoName, infer(&text), &text);
}

fn infer(content: &str) -> String {
    let (db, file_id) = MockDatabase::with_single_file(content);
    let source_file = db.parse(file_id).ok().unwrap();

    let mut acc = String::new();

    let mut infer_def = |infer_result: Arc<InferenceResult>,
                         body_source_map: Arc<BodySourceMap>| {
        let mut types = Vec::new();

        for (pat, ty) in infer_result.type_of_pat.iter() {
            let syntax_ptr = match body_source_map.pat_syntax(pat) {
                Some(sp) => sp.map(|ast| ast.syntax_node_ptr()),
                None => continue,
            };
            types.push((syntax_ptr, ty));
        }

        for (expr, ty) in infer_result.type_of_expr.iter() {
            let syntax_ptr = match body_source_map.expr_syntax(expr) {
                Some(sp) => sp.map(|ast| ast.syntax_node_ptr()),
                None => continue,
            };
            types.push((syntax_ptr, ty));
        }

        // Sort ranges for consistency
        types.sort_by_key(|(src_ptr, _)| (src_ptr.ast.range().start(), src_ptr.ast.range().end()));
        for (src_ptr, ty) in &types {
            let node = src_ptr.ast.to_node(&src_ptr.file_syntax(&db));

            let (range, text) = (
                src_ptr.ast.range(),
                node.text().to_string().replace("\n", " "),
            );
            write!(
                acc,
                "{} '{}': {}\n",
                range,
                ellipsize(text, 15),
                ty.display(&db)
            )
            .unwrap();
        }
    };

    let ctx = LocationCtx::new(&db, file_id);
    for node in source_file.syntax().descendants() {
        if let Some(def) = ast::FunctionDef::cast(node.clone()) {
            let fun = Function {
                id: ctx.to_def(&def),
            };
            let source_map = fun.body_source_map(&db);
            let infer_result = fun.infer(&db);
            infer_def(infer_result, source_map);
        }
    }

    acc.truncate(acc.trim_end().len());
    acc
}

fn ellipsize(mut text: String, max_len: usize) -> String {
    if text.len() <= max_len {
        return text;
    }
    let ellipsis = "...";
    let e_len = ellipsis.len();
    let mut prefix_len = (max_len - e_len) / 2;
    while !text.is_char_boundary(prefix_len) {
        prefix_len += 1;
    }
    let mut suffix_len = max_len - e_len - prefix_len;
    while !text.is_char_boundary(text.len() - suffix_len) {
        suffix_len += 1;
    }
    text.replace_range(prefix_len..text.len() - suffix_len, ellipsis);
    text
}
