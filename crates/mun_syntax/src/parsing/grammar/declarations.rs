use super::{
    adt, error_block, expressions, name, name_recovery, opt_visibility, params, paths, traits,
    types, Marker, Parser, TokenSet, EOF, ERROR, EXTERN, FUNCTION_DEF, RENAME, RET_TYPE, USE,
    USE_TREE, USE_TREE_LIST,
};
use crate::{parsing::grammar::paths::is_use_path_start, T};

pub(super) const DECLARATION_RECOVERY_SET: TokenSet = TokenSet::new(&[
    T![enum],
    T![fn],
    T![impl],
    T![pub],
    T![struct],
    T![use],
    T![;],
]);

pub(super) fn mod_contents(p: &mut Parser<'_>) {
    while !p.at(EOF) {
        declaration(p, false);
    }
}

pub(super) fn declaration(p: &mut Parser<'_>, stop_on_r_curly: bool) {
    let m = p.start();
    let m = match maybe_declaration(p, m) {
        Ok(()) => return,
        Err(m) => m,
    };

    m.abandon(p);
    match p.current() {
        T!['{'] => error_block(p, "expected a declaration"),
        T!['}'] if !stop_on_r_curly => {
            let e = p.start();
            p.error("unmatched }");
            p.bump(T!['}']);
            e.complete(p, ERROR);
        }
        EOF | T!['}'] => p.error("expected a declaration"),
        _ => p.error_and_bump("expected a declaration"),
    }
}

pub(super) fn maybe_declaration(p: &mut Parser<'_>, m: Marker) -> Result<(), Marker> {
    opt_visibility(p);

    let m = match declarations_without_modifiers(p, m) {
        Ok(()) => return Ok(()),
        Err(m) => m,
    };

    if p.at(T![extern]) {
        abi(p);
    }

    match p.current() {
        T![fn] => {
            fn_def(p);
            m.complete(p, FUNCTION_DEF);
        }
        _ => return Err(m),
    }
    Ok(())
}

fn abi(p: &mut Parser<'_>) {
    assert!(p.at(T![extern]));
    let abi = p.start();
    p.bump(T![extern]);
    abi.complete(p, EXTERN);
}

fn declarations_without_modifiers(p: &mut Parser<'_>, m: Marker) -> Result<(), Marker> {
    match p.current() {
        T![enum] => {
            adt::enum_def(p, m);
        }
        T![use] => {
            use_(p, m);
        }
        T![struct] => {
            adt::struct_def(p, m);
        }
        T![type] => {
            adt::type_alias_def(p, m);
        }
        T![impl] => {
            traits::impl_(p, m);
        }
        _ => return Err(m),
    };
    Ok(())
}

pub(super) fn fn_def(p: &mut Parser<'_>) {
    assert!(p.at(T![fn]));
    p.bump(T![fn]);

    name_recovery(p, DECLARATION_RECOVERY_SET.union(TokenSet::new(&[T![')']])));

    if p.at(T!['(']) {
        params::param_list(p);
    } else {
        p.error("expected function arguments");
    }

    opt_fn_ret_type(p);

    if p.at(T![;]) {
        p.bump(T![;]);
    } else {
        expressions::block(p);
    }
}

fn opt_fn_ret_type(p: &mut Parser<'_>) -> bool {
    if p.at(T![->]) {
        let m = p.start();
        p.bump(T![->]);
        types::type_(p);
        m.complete(p, RET_TYPE);
        true
    } else {
        false
    }
}

fn use_(p: &mut Parser<'_>, m: Marker) {
    assert!(p.at(T![use]));
    p.bump(T![use]);
    use_tree(p, true);
    p.expect(T![;]);
    m.complete(p, USE);
}

/// Parses a use "tree", such as `foo::bar` in `use foo::bar;`.
fn use_tree(p: &mut Parser<'_>, top_level: bool) {
    let m = p.start();

    match p.current() {
        T![*] if !top_level => p.bump(T![*]),
        _ if is_use_path_start(p, top_level) => {
            paths::use_path(p, top_level);
            match p.current() {
                T![as] => {
                    opt_rename(p);
                }
                T![:] if p.at(T![::]) => {
                    p.bump(T![::]);
                    match p.current() {
                        T![*] => {
                            p.bump(T![*]);
                        }
                        T!['{'] => use_tree_list(p),
                        _ => {
                            p.error("expected `{` or `*`");
                        }
                    }
                }
                _ => (),
            }
        }
        _ => {
            m.abandon(p);
            let msg = "expected one of `self`, `super`, `package` or an identifier";
            if top_level {
                p.error_recover(msg, DECLARATION_RECOVERY_SET);
            } else {
                // if we are parsing a nested tree, we have to eat a token to remain balanced
                // `{}`
                p.error_and_bump(msg);
            }
            return;
        }
    }

    m.complete(p, USE_TREE);
}

fn use_tree_list(p: &mut Parser<'_>) {
    assert!(p.at(T!['{']));
    let m = p.start();
    p.bump(T!['{']);
    while !p.at(EOF) && !p.at(T!['}']) {
        use_tree(p, false);
        if !p.at(T!['}']) {
            p.expect(T![,]);
        }
    }
    p.expect(T!['}']);
    m.complete(p, USE_TREE_LIST);
}

fn opt_rename(p: &mut Parser<'_>) {
    if p.at(T![as]) {
        let m = p.start();
        p.bump(T![as]);
        if !p.eat(T![_]) {
            name(p);
        }
        m.complete(p, RENAME);
    }
}
