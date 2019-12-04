use super::*;
use crate::parsing::grammar::paths::PATH_FIRST;

pub(crate) const LITERAL_FIRST: TokenSet =
    token_set![TRUE_KW, FALSE_KW, INT_NUMBER, FLOAT_NUMBER, STRING];

const EXPR_RECOVERY_SET: TokenSet = token_set![LET_KW];

const ATOM_EXPR_FIRST: TokenSet = LITERAL_FIRST.union(PATH_FIRST).union(token_set![
    IDENT,
    T!['('],
    T!['{'],
    T![if],
    T![loop],
    T![return],
    T![break],
    T![while],
]);

const LHS_FIRST: TokenSet = ATOM_EXPR_FIRST.union(token_set![EXCLAMATION, MINUS]);

const EXPR_FIRST: TokenSet = LHS_FIRST;

#[derive(Clone, Copy)]
struct Restrictions {
    /// Indicates that parsing of structs is not valid in the current context. For instance:
    /// ```mun
    /// if break { 3 }
    /// if break 4 { 3 }
    /// ```
    /// In the first if expression we do not want the `break` expression to capture the block as an
    /// expression. However, in the second statement we do want the break to capture the 4.
    forbid_structs: bool,
}

pub(crate) fn expr_block_contents(p: &mut Parser) {
    while !p.at(EOF) && !p.at(T!['}']) {
        if p.eat(T![;]) {
            continue;
        }

        stmt(p);
    }
}

/// Parses a block statement
pub(crate) fn block(p: &mut Parser) {
    if !p.at(T!['{']) {
        p.error("expected a block");
        return;
    }
    block_expr(p);
}

fn block_expr(p: &mut Parser) -> CompletedMarker {
    assert!(p.at(T!['{']));
    let m = p.start();
    p.bump(T!['{']);
    expr_block_contents(p);
    p.expect(T!['}']);
    m.complete(p, BLOCK_EXPR)
}

/// Parses a general statement: (let, expr, etc.)
pub(super) fn stmt(p: &mut Parser) {
    let m = p.start();

    // Encounters let keyword, so we know it's a let stmt
    if p.at(T![let]) {
        let_stmt(p, m);
        return;
    }

    let (cm, _blocklike) = expr_stmt(p);
    let kind = cm.as_ref().map(|cm| cm.kind()).unwrap_or(ERROR);

    if p.at(T!['}']) {
        if let Some(cm) = cm {
            cm.undo_completion(p).abandon(p);
            m.complete(p, kind);
        } else {
            m.abandon(p);
        }
    } else {
        p.eat(T![;]);
        m.complete(p, EXPR_STMT);
    }
}

fn let_stmt(p: &mut Parser, m: Marker) {
    assert!(p.at(T![let]));
    p.bump(T![let]);
    patterns::pattern(p);
    if p.at(T![:]) {
        types::ascription(p);
    }
    if p.eat(T![=]) {
        expressions::expr(p);
    }

    p.eat(T![;]); // Semicolon at the end of statement belongs to the statement
    m.complete(p, LET_STMT);
}

pub(super) fn expr(p: &mut Parser) {
    let r = Restrictions {
        forbid_structs: false,
    };
    expr_bp(p, r, 1);
}

fn expr_no_struct(p: &mut Parser) {
    let r = Restrictions {
        forbid_structs: true,
    };
    expr_bp(p, r, 1);
}

fn expr_stmt(p: &mut Parser) -> (Option<CompletedMarker>, BlockLike) {
    let r = Restrictions {
        forbid_structs: false,
    };
    expr_bp(p, r, 1)
}

fn expr_bp(p: &mut Parser, r: Restrictions, bp: u8) -> (Option<CompletedMarker>, BlockLike) {
    // Parse left hand side of the expression
    let mut lhs = match lhs(p, r) {
        Some((lhs, blocklike)) => {
            if blocklike.is_block() {
                return (Some(lhs), BlockLike::Block);
            }
            lhs
        }
        None => return (None, BlockLike::NotBlock),
    };

    loop {
        let (op_bp, op) = current_op(p);
        if op_bp < bp {
            break;
        }

        let m = lhs.precede(p);
        p.bump(op);

        expr_bp(p, r, op_bp + 1);
        lhs = m.complete(p, BIN_EXPR);
    }

    (Some(lhs), BlockLike::NotBlock)
}

fn current_op(p: &Parser) -> (u8, SyntaxKind) {
    match p.current() {
        T![+] if p.at(T![+=]) => (1, T![+=]),
        T![+] => (10, T![+]),
        T![-] if p.at(T![-=]) => (1, T![-=]),
        T![-] => (10, T![-]),
        T![*] if p.at(T![*=]) => (1, T![*=]),
        T![*] => (11, T![*]),
        T![/] if p.at(T![/=]) => (1, T![/=]),
        T![/] => (11, T![/]),
        T![=] if p.at(T![==]) => (5, T![==]),
        T![=] => (1, T![=]),
        T![!] if p.at(T![!=]) => (5, T![!=]),
        T![>] if p.at(T![>=]) => (5, T![>=]),
        T![>] => (5, T![>]),
        T![<] if p.at(T![<=]) => (5, T![<=]),
        T![<] => (5, T![<]),
        _ => (0, T![_]),
    }
}

fn lhs(p: &mut Parser, r: Restrictions) -> Option<(CompletedMarker, BlockLike)> {
    let m;
    let kind = match p.current() {
        T![-] | T![!] => {
            m = p.start();
            p.bump_any();
            PREFIX_EXPR
        }
        _ => {
            let (lhs, blocklike) = atom_expr(p, r)?;
            return Some(postfix_expr(p, lhs, blocklike));
        }
    };
    expr_bp(p, r, 255);
    Some((m.complete(p, kind), BlockLike::NotBlock))
}

fn postfix_expr(
    p: &mut Parser,
    mut lhs: CompletedMarker,
    _blocklike: BlockLike,
) -> (CompletedMarker, BlockLike) {
    loop {
        lhs = match p.current() {
            T!['('] => call_expr(p, lhs),
            _ => break,
        }
    }
    (lhs, BlockLike::NotBlock)
}

fn call_expr(p: &mut Parser, lhs: CompletedMarker) -> CompletedMarker {
    assert!(p.at(T!['(']));
    let m = lhs.precede(p);
    arg_list(p);
    m.complete(p, CALL_EXPR)
}

fn arg_list(p: &mut Parser) {
    assert!(p.at(T!['(']));
    let m = p.start();
    p.bump(T!['(']);
    while !p.at(T![')']) && !p.at(EOF) {
        if !p.at_ts(EXPR_FIRST) {
            p.error("expected expression");
            break;
        }

        expr(p);
        if !p.at(T![')']) && !p.expect(T![,]) {
            break;
        }
    }
    p.eat(T![')']);
    m.complete(p, ARG_LIST);
}

fn atom_expr(p: &mut Parser, r: Restrictions) -> Option<(CompletedMarker, BlockLike)> {
    if let Some(m) = literal(p) {
        return Some((m, BlockLike::NotBlock));
    }

    if paths::is_path_start(p) {
        return Some(path_expr(p, r));
    }

    let marker = match p.current() {
        T!['('] => paren_expr(p),
        T!['{'] => block_expr(p),
        T![if] => if_expr(p),
        T![loop] => loop_expr(p),
        T![return] => ret_expr(p),
        T![while] => while_expr(p),
        T![break] => break_expr(p, r),
        _ => {
            p.error_recover("expected expression", EXPR_RECOVERY_SET);
            return None;
        }
    };
    let blocklike = match marker.kind() {
        IF_EXPR | WHILE_EXPR | LOOP_EXPR | BLOCK_EXPR => BlockLike::Block,
        _ => BlockLike::NotBlock,
    };
    Some((marker, blocklike))
}

fn path_expr(p: &mut Parser, r: Restrictions) -> (CompletedMarker, BlockLike) {
    assert!(paths::is_path_start(p));
    let m = p.start();
    paths::expr_path(p);
    match p.current() {
        T!['{'] if !r.forbid_structs => {
            record_field_list(p);
            (m.complete(p, RECORD_LIT), BlockLike::NotBlock)
        }
        _ => (m.complete(p, PATH_EXPR), BlockLike::NotBlock),
    }
}

fn literal(p: &mut Parser) -> Option<CompletedMarker> {
    if !p.at_ts(LITERAL_FIRST) {
        return None;
    }
    let m = p.start();
    p.bump_any();
    Some(m.complete(p, LITERAL))
}

fn paren_expr(p: &mut Parser) -> CompletedMarker {
    assert!(p.at(T!['(']));
    let m = p.start();
    p.bump(T!['(']);
    expr(p);
    p.expect(T![')']);
    m.complete(p, PAREN_EXPR)
}

fn if_expr(p: &mut Parser) -> CompletedMarker {
    assert!(p.at(T![if]));
    let m = p.start();
    p.bump(T![if]);
    cond(p);
    block(p);
    if p.at(T![else]) {
        p.bump(T![else]);
        if p.at(T![if]) {
            if_expr(p);
        } else {
            block(p);
        }
    }
    m.complete(p, IF_EXPR)
}

fn loop_expr(p: &mut Parser) -> CompletedMarker {
    assert!(p.at(T![loop]));
    let m = p.start();
    p.bump(T![loop]);
    block(p);
    m.complete(p, LOOP_EXPR)
}

fn cond(p: &mut Parser) {
    let m = p.start();
    expr_no_struct(p);
    m.complete(p, CONDITION);
}

fn ret_expr(p: &mut Parser) -> CompletedMarker {
    assert!(p.at(T![return]));
    let m = p.start();
    p.bump(T![return]);
    if p.at_ts(EXPR_FIRST) {
        expr(p);
    }
    m.complete(p, RETURN_EXPR)
}

fn break_expr(p: &mut Parser, r: Restrictions) -> CompletedMarker {
    assert!(p.at(T![break]));
    let m = p.start();
    p.bump(T![break]);
    if p.at_ts(EXPR_FIRST) && !(r.forbid_structs && p.at(T!['{'])) {
        expr(p);
    }
    m.complete(p, BREAK_EXPR)
}

fn while_expr(p: &mut Parser) -> CompletedMarker {
    assert!(p.at(T![while]));
    let m = p.start();
    p.bump(T![while]);
    cond(p);
    block(p);
    m.complete(p, WHILE_EXPR)
}

fn record_field_list(p: &mut Parser) {
    assert!(p.at(T!['{']));
    let m = p.start();
    p.bump(T!['{']);
    while !p.at(EOF) && !p.at(T!['}']) {
        match p.current() {
            IDENT | INT_NUMBER => {
                let m = p.start();
                name_ref_or_index(p);
                if p.eat(T![:]) {
                    expr(p);
                }
                m.complete(p, RECORD_FIELD);
            }
            T!['{'] => error_block(p, "expected a field"),
            _ => p.error_and_bump("expected an identifier"),
        }
        if !p.at(T!['}']) {
            p.expect(T![,]);
        }
    }
    p.expect(T!['}']);
    m.complete(p, RECORD_FIELD_LIST);
}
