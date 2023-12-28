use super::{paths, Parser, TokenSet, ARRAY_TYPE, NEVER_TYPE, PATH_TYPE};

pub(super) const TYPE_FIRST: TokenSet =
    paths::PATH_FIRST.union(TokenSet::new(&[T![never], T!['[']]));

pub(super) const TYPE_RECOVERY_SET: TokenSet = TokenSet::new(&[T!['('], T![,], T![pub]]);

pub(super) fn ascription(p: &mut Parser<'_>) {
    p.expect(T![:]);
    type_(p);
}

pub(super) fn type_(p: &mut Parser<'_>) {
    match p.current() {
        T!['['] => array_type(p),
        T![never] => never_type(p),
        _ if paths::is_path_start(p) => path_type(p),
        _ => {
            p.error_recover("expected type", TYPE_RECOVERY_SET);
        }
    }
}

pub(super) fn path_type(p: &mut Parser<'_>) {
    let m = p.start();
    paths::type_path(p);
    m.complete(p, PATH_TYPE);
}

fn never_type(p: &mut Parser<'_>) {
    assert!(p.at(T![never]));
    let m = p.start();
    p.bump(T![never]);
    m.complete(p, NEVER_TYPE);
}

fn array_type(p: &mut Parser<'_>) {
    assert!(p.at(T!['[']));
    let m = p.start();
    p.bump(T!['[']);
    type_(p);
    p.expect(T![']']);
    m.complete(p, ARRAY_TYPE);
}
