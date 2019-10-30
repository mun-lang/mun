use crate::db::SourceDatabase;
use crate::diagnostics::DiagnosticSink;
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
    fn test(a:int, b:float, c:never, d:bool): bool {
        a;
        b;
        c;
        d
    }
    "#,
    )
}

#[test]
fn infer_branching() {
    infer_snapshot(
        r#"
    fn test() {
        let a = if true { 3 } else { 4 }
        let b = if true { 3 }               // Missing else branch
        let c = if true { 3; }
        let d = if true { 5 } else if false { 3 } else { 4 }
        let e = if true { 5.0 } else { 5 }  // Mismatched branches
    }
    "#,
    )
}

#[test]
fn void_return() {
    infer_snapshot(
        r#"
    fn bar() {
        let a = 3;
    }
    fn foo(a:int) {
        let c = bar()
    }
    "#,
    )
}

#[test]
fn place_expressions() {
    infer_snapshot(
        r#"
    fn foo(a:int) {
        a += 3;
        3 = 5; // error: invalid left hand side of expression
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

    let mut diags = String::new();

    let mut diag_sink = DiagnosticSink::new(|diag| {
        write!(diags, "{}: {}\n", diag.highlight_range(), diag.message()).unwrap();
    });

    let ctx = LocationCtx::new(&db, file_id);
    for node in source_file.syntax().descendants() {
        if let Some(def) = ast::FunctionDef::cast(node.clone()) {
            let fun = Function {
                id: ctx.to_def(&def),
            };
            let source_map = fun.body_source_map(&db);
            let infer_result = fun.infer(&db);

            for diag in infer_result.diagnostics.iter() {
                diag.add_to(&db, fun, &mut diag_sink);
            }

            infer_def(infer_result, source_map);
        }
    }

    drop(diag_sink);

    acc.truncate(acc.trim_end().len());
    diags.truncate(diags.trim_end().len());
    [diags, acc].join("\n").trim().to_string()
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
