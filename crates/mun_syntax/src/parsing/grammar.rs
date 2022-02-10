mod adt;
mod declarations;
mod expressions;
mod params;
mod paths;
mod patterns;
mod types;

use super::{
    parser::{CompletedMarker, Marker, Parser},
    token_set::TokenSet,
    SyntaxKind::{self, *},
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum BlockLike {
    Block,
    NotBlock,
}

impl BlockLike {
    fn is_block(self) -> bool {
        self == BlockLike::Block
    }
}

pub(crate) fn root(p: &mut Parser) {
    let m = p.start();
    declarations::mod_contents(p);
    m.complete(p, SOURCE_FILE);
}

//pub(crate) fn pattern(p: &mut Parser) {
//    patterns::pattern(p)
//}
//
//pub(crate) fn expr(p: &mut Parser) {
//    expressions::expr(p);
//}
//
//pub(crate) fn type_(p: &mut Parser) {
//    types::type_(p)
//}

fn name_recovery(p: &mut Parser, recovery: TokenSet) {
    if p.at(IDENT) {
        let m = p.start();
        p.bump(IDENT);
        m.complete(p, NAME);
    } else {
        p.error_recover("expected a name", recovery)
    }
}

fn name(p: &mut Parser) {
    name_recovery(p, TokenSet::empty())
}

fn name_ref(p: &mut Parser) {
    if p.at(IDENT) {
        let m = p.start();
        p.bump(IDENT);
        m.complete(p, NAME_REF);
    } else {
        p.error_and_bump("expected identifier");
    }
}

fn name_ref_or_index(p: &mut Parser) {
    assert!(p.at(IDENT) || p.at(INT_NUMBER));
    let m = p.start();
    p.bump_any();
    m.complete(p, NAME_REF);
}

fn opt_visibility(p: &mut Parser) -> bool {
    match p.current() {
        T![pub] => {
            let m = p.start();
            p.bump(T![pub]);
            if p.at(T!['(']) {
                match p.nth(1) {
                    T![package] | T![super] => {
                        p.bump_any();
                        p.bump_any();
                        p.expect(T![')']);
                    }
                    _ => (),
                }
            }
            m.complete(p, VISIBILITY);
            true
        }
        _ => false,
    }
}

fn error_block(p: &mut Parser, message: &str) {
    assert!(p.at(T!['{']));
    let m = p.start();
    p.error(message);
    p.bump(T!['{']);
    expressions::expr_block_contents(p);
    p.eat(T!['}']);
    m.complete(p, ERROR);
}
