use crate::{AstNode, SyntaxNode, TextSize};
use itertools::Itertools;

/// Returns ancestors of the node at the offset, sorted by length. This should do the right thing at
/// an edge, e.g. when searching for expressions at `{ $0foo }` we will get the name reference
/// instead of the whole block, which we would get if we just did `find_token_at_offset(...).
/// flat_map(|t| t.parent().ancestors())`.
pub fn ancestors_at_offset(
    node: &SyntaxNode,
    offset: TextSize,
) -> impl Iterator<Item = SyntaxNode> {
    node.token_at_offset(offset)
        .map(|token| token.ancestors())
        .kmerge_by(|node1, node2| node1.text_range().len() < node2.text_range().len())
}

/// Finds a node of specific Ast type at offset. Note that this is slightly imprecise: if the cursor
/// is strictly between two nodes of the desired type, as in
///
/// ```mun
/// struct Foo {}|struct Bar;
/// ```
///
/// then the shorter node will be silently preferred.
pub fn find_node_at_offset<N: AstNode>(syntax: &SyntaxNode, offset: TextSize) -> Option<N> {
    ancestors_at_offset(syntax, offset).find_map(N::cast)
}
