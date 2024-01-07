use super::{declarations::declaration, error_block, types};
use crate::{
    parsing::parser::{Marker, Parser},
    SyntaxKind::{ASSOCIATED_ITEM_LIST, EOF, IMPL},
};

pub(super) fn impl_(p: &mut Parser<'_>, m: Marker) {
    p.bump(T![impl]);
    types::type_(p);
    if p.at(T!['{']) {
        associated_item_list(p);
    } else {
        p.error("expected `{`");
    }
    m.complete(p, IMPL);
}

fn associated_item_list(p: &mut Parser<'_>) {
    assert!(p.at(T!['{']));
    let m = p.start();
    p.bump(T!['{']);
    while !p.at(EOF) && !p.at(T!['}']) {
        if p.at(T!['{']) {
            error_block(p, "expected an associated item");
            continue;
        }
        declaration(p, true);
    }
    p.expect(T!['}']);
    m.complete(p, ASSOCIATED_ITEM_LIST);
}
