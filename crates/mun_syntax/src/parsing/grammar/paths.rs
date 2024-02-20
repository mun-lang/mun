use super::{declarations, name_ref, Parser, TokenSet, IDENT, PATH, PATH_SEGMENT};
use crate::SyntaxKind::NAME_REF;

pub(super) const PATH_FIRST: TokenSet =
    TokenSet::new(&[IDENT, T![super], T![self], T![package], T![Self], T![::]]);

pub(super) fn is_path_start(p: &Parser<'_>) -> bool {
    is_use_path_start(p, true) || p.at(T![Self])
}

pub(super) fn is_use_path_start(p: &Parser<'_>, top_level: bool) -> bool {
    if top_level {
        matches!(p.current(), IDENT | T![self] | T![super] | T![package])
    } else {
        matches!(p.current(), IDENT | T![self])
    }
}

pub(super) fn type_path(p: &mut Parser<'_>) {
    path(p, Mode::Type, true);
}
pub(super) fn expr_path(p: &mut Parser<'_>) {
    path(p, Mode::Expr, true);
}
pub(super) fn use_path(p: &mut Parser<'_>, top_level: bool) {
    path(p, Mode::Use, top_level);
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum Mode {
    Type,
    Expr,
    Use,
}

fn path(p: &mut Parser<'_>, mode: Mode, top_level: bool) {
    let path = p.start();
    path_segment(p, mode, top_level);
    let mut qualifier = path.complete(p, PATH);
    loop {
        let use_tree = matches!(p.nth(2), T![*] | T!['{']);
        if p.at(T![::]) && !use_tree {
            let path = qualifier.precede(p);
            p.bump(T![::]);
            path_segment(p, mode, false);
            let path = path.complete(p, PATH);
            qualifier = path;
        } else {
            break;
        }
    }
}

fn path_segment(p: &mut Parser<'_>, _mode: Mode, top_level: bool) {
    let m = p.start();
    match p.current() {
        IDENT => {
            name_ref(p);
        }
        T![super] | T![package] | T![Self] if top_level => {
            let m = p.start();
            p.bump_any();
            m.complete(p, NAME_REF);
        }
        T![self] => p.bump(T![self]),
        _ => p.error_recover(
            "expected identifier",
            declarations::DECLARATION_RECOVERY_SET,
        ),
    }
    m.complete(p, PATH_SEGMENT);
}
