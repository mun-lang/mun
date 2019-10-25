use super::*;

pub(super) const TYPE_RECOVERY_SET: TokenSet = token_set![R_PAREN, COMMA];

pub(super) fn ascription(p: &mut Parser) {
    p.expect(T![:]);
    type_(p);
}

pub(super) fn type_(p: &mut Parser) {
    match p.current() {
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
