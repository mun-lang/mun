use super::*;

pub(super) const TYPE_RECOVERY_SET: TokenSet = token_set![R_PAREN, COMMA];

pub(super) fn ascription(p: &mut Parser) {
    p.expect(T![:]);
    type_(p);
}

pub(super) fn type_(p: &mut Parser) {
    match p.current() {
        T![never] => never_type(p),
        _ if paths::is_path_start(p) => path_type(p),
        _ => {
            p.error_recover("expected type", TYPE_RECOVERY_SET);
        }
    }
}

pub(super) fn path_type(p: &mut Parser) {
    let m = p.start();
    paths::type_path(p);
    m.complete(p, PATH_TYPE);
}

fn never_type(p: &mut Parser) {
    assert!(p.at(T![never]));
    let m = p.start();
    p.bump(T![never]);
    m.complete(p, NEVER_TYPE);
}
