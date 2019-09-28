use super::*;

pub(super) const PATH_FIRST: TokenSet = token_set![IDENT, SELF_KW, SUPER_KW, COLONCOLON];

pub(super) fn is_path_start(p: &Parser) -> bool {
    match p.current() {
        IDENT | COLONCOLON => true,
        _ => false,
    }
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
    path_segment(p, mode, true);
    let mut qualifier = path.complete(p, PATH);
    loop {
        let import_tree = match p.nth(1) {
            STAR | L_CURLY => true,
            _ => false,
        };
        if p.matches(COLONCOLON) && !import_tree {
            let path = qualifier.precede(p);
            p.bump();
            path_segment(p, mode, false);
            let path = path.complete(p, PATH);
            qualifier = path;
        } else {
            break;
        }
    }
}

fn path_segment(p: &mut Parser, _mode: Mode, first: bool) {
    let m = p.start();
    if first {
        p.eat(COLONCOLON);
    }
    match p.current() {
        IDENT => {
            name_ref(p);
        }
        SELF_KW | SUPER_KW => p.bump(),
        _ => p.error_recover(
            "expected identifier",
            declarations::DECLARATION_RECOVERY_SET,
        ),
    }
    m.complete(p, PATH_SEGMENT);
}
