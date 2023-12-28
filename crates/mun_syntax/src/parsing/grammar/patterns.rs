use super::{
    expressions, name, paths, CompletedMarker, Parser, TokenSet, BIND_PAT, IDENT, PLACEHOLDER_PAT,
};

pub(super) const PATTERN_FIRST: TokenSet = expressions::LITERAL_FIRST
    .union(paths::PATH_FIRST)
    .union(TokenSet::new(&[T![-], T![_]]));

pub(super) fn pattern(p: &mut Parser<'_>) {
    pattern_r(p, PATTERN_FIRST);
}

pub(super) fn pattern_r(p: &mut Parser<'_>, recovery_set: TokenSet) {
    atom_pat(p, recovery_set);
}

fn atom_pat(p: &mut Parser<'_>, recovery_set: TokenSet) -> Option<CompletedMarker> {
    let t1 = p.nth(0);
    if t1 == IDENT {
        return Some(bind_pat(p));
    }

    #[allow(clippy::single_match_else)]
    let m = match t1 {
        T![_] => placeholder_pat(p),
        _ => {
            p.error_recover("expected pattern", recovery_set);
            return None;
        }
    };
    Some(m)
}

fn placeholder_pat(p: &mut Parser<'_>) -> CompletedMarker {
    assert!(p.at(T![_]));
    let m = p.start();
    p.bump(T![_]);
    m.complete(p, PLACEHOLDER_PAT)
}

fn bind_pat(p: &mut Parser<'_>) -> CompletedMarker {
    let m = p.start();
    name(p);
    m.complete(p, BIND_PAT)
}
