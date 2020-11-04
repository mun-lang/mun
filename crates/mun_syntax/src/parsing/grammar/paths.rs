use super::*;

pub(super) const PATH_FIRST: TokenSet =
    TokenSet::new(&[IDENT, T![super], T![self], T![package], T![::]]);

pub(super) fn is_path_start(p: &Parser) -> bool {
    matches!(p.current(), IDENT | T![self] | T![super] | T![package])
}

pub(super) fn type_path(p: &mut Parser) {
    path(p, Mode::Type)
}
pub(super) fn expr_path(p: &mut Parser) {
    path(p, Mode::Expr)
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum Mode {
    Type,
    Expr,
}

fn path(p: &mut Parser, mode: Mode) {
    let path = p.start();
    path_segment(p, mode);
    let mut qualifier = path.complete(p, PATH);
    loop {
        let import_tree = matches!(p.nth(1), T![*] | T!['{']);
        if p.at(T![::]) && !import_tree {
            let path = qualifier.precede(p);
            p.bump(T![::]);
            path_segment(p, mode);
            let path = path.complete(p, PATH);
            qualifier = path;
        } else {
            break;
        }
    }
}

fn path_segment(p: &mut Parser, _mode: Mode) {
    let m = p.start();
    match p.current() {
        IDENT => {
            name_ref(p);
        }
        T![self] | T![super] | T![package] => p.bump_any(),
        _ => p.error_recover(
            "expected identifier",
            declarations::DECLARATION_RECOVERY_SET,
        ),
    }
    m.complete(p, PATH_SEGMENT);
}
