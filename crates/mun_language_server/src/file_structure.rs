use crate::SymbolKind;
use mun_syntax::{
    ast::{self, NameOwner},
    match_ast, AstNode, SourceFile, SyntaxNode, TextRange, WalkEvent,
};

/// A description of a symbol in a source file.
#[derive(Debug, Clone)]
pub struct StructureNode {
    /// An optional parent of this symbol. Refers to the index of the symbol in the collection that
    /// this instance resides in.
    pub parent: Option<usize>,

    /// The text label
    pub label: String,

    /// The range to navigate to if selected
    pub navigation_range: TextRange,

    /// The entire range of the node in the file
    pub node_range: TextRange,

    /// The type of symbol
    pub kind: SymbolKind,

    /// Optional detailed information
    pub detail: Option<String>,
}

/// Provides a tree of symbols defined in a `SourceFile`.
pub(crate) fn file_structure(file: &SourceFile) -> Vec<StructureNode> {
    let mut result = Vec::new();
    let mut stack = Vec::new();

    for event in file.syntax().preorder() {
        match event {
            WalkEvent::Enter(node) => {
                if let Some(mut symbol) = try_convert_to_structure_node(&node) {
                    symbol.parent = stack.last().copied();
                    stack.push(result.len());
                    result.push(symbol);
                }
            }
            WalkEvent::Leave(node) => {
                if try_convert_to_structure_node(&node).is_some() {
                    stack.pop().unwrap();
                }
            }
        }
    }

    result
}

/// Tries to convert an ast node to something that would reside in the hierarchical file structure.
fn try_convert_to_structure_node(node: &SyntaxNode) -> Option<StructureNode> {
    /// Create a `StructureNode` from a declaration
    fn decl<N: NameOwner>(node: N, kind: SymbolKind) -> Option<StructureNode> {
        decl_with_detail(&node, None, kind)
    }

    /// Create a `StructureNode` from a declaration with extra text detail
    fn decl_with_detail<N: NameOwner>(
        node: &N,
        detail: Option<String>,
        kind: SymbolKind,
    ) -> Option<StructureNode> {
        let name = node.name()?;

        Some(StructureNode {
            parent: None,
            label: name.text().to_string(),
            navigation_range: name.syntax().text_range(),
            node_range: node.syntax().text_range(),
            kind,
            detail,
        })
    }

    /// Given a `SyntaxNode` get the text without any whitespaces
    fn collapse_whitespaces(node: &SyntaxNode, output: &mut String) {
        let mut can_insert_ws = false;
        node.text().for_each_chunk(|chunk| {
            for line in chunk.lines() {
                let line = line.trim();
                if line.is_empty() {
                    if can_insert_ws {
                        output.push(' ');
                        can_insert_ws = false;
                    }
                } else {
                    output.push_str(line);
                    can_insert_ws = true;
                }
            }
        })
    }

    /// Given a `SyntaxNode` construct a `StructureNode` by referring to the type of a node.
    fn decl_with_type_ref<N: NameOwner>(
        node: &N,
        type_ref: Option<ast::TypeRef>,
        kind: SymbolKind,
    ) -> Option<StructureNode> {
        let detail = type_ref.map(|type_ref| {
            let mut detail = String::new();
            collapse_whitespaces(type_ref.syntax(), &mut detail);
            detail
        });
        decl_with_detail(node, detail, kind)
    }

    match_ast! {
        match node {
            ast::FunctionDef(it) => {
                let mut detail = String::from("fn");
                if let Some(param_list) = it.param_list() {
                    collapse_whitespaces(param_list.syntax(), &mut detail);
                }
                if let Some(ret_type) = it.ret_type() {
                    detail.push(' ');
                    collapse_whitespaces(ret_type.syntax(), &mut detail);
                }

                decl_with_detail(&it, Some(detail), SymbolKind::Function)
            },
            ast::StructDef(it) => decl(it, SymbolKind::Struct),
            ast::TypeAliasDef(it) => decl_with_type_ref(&it, it.type_ref(), SymbolKind::TypeAlias),
            _ => None
        }
    }
}
