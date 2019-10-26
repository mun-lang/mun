use crate::{
    parsing::{event::Event, token_set::TokenSet, ParseError, TokenSource},
    SyntaxKind::{self, *},
};
use drop_bomb::DropBomb;
use std::cell::Cell;

/// `Parser` struct provides the low-level API for navigating through the stream of tokens and
/// constructing the parse tree. The actual parsing happens in the `grammar` module.
///
/// However, the result of this `Parser` is not a real tree, but rather a flat stream of
/// events of the form 'start expression, consume number literal, finish espression'. See `Event`
/// docs for more info.
pub(crate) struct Parser<'t> {
    token_source: &'t mut dyn TokenSource,
    events: Vec<Event>,
    steps: Cell<u32>,
}

impl<'t> Parser<'t> {
    pub(super) fn new(token_source: &'t mut dyn TokenSource) -> Parser<'t> {
        Parser {
            token_source,
            events: Vec::new(),
            steps: Cell::new(0),
        }
    }

    pub(crate) fn finish(self) -> Vec<Event> {
        self.events
    }

    /// Returns the kind of the current token.
    /// If the parser has already reach the end of the input the special `EOF` kind is returned.
    pub(crate) fn current(&self) -> SyntaxKind {
        self.nth(0)
    }

    /// Lookahead operation: returns the kind of the next nth
    /// token.
    pub(crate) fn nth(&self, n: usize) -> SyntaxKind {
        assert!(n <= 3);

        let steps = self.steps.get();
        assert!(steps <= 10_000_000, "the parser seems stuck");
        self.steps.set(steps + 1);

        self.token_source.lookahead_nth(n).kind
    }

    // Checks if the current token is `kind`.
    pub(crate) fn at(&self, kind: SyntaxKind) -> bool {
        self.nth_at(0, kind)
    }

    pub(crate) fn nth_at(&self, n: usize, kind: SyntaxKind) -> bool {
        match kind {
            T![-=] => self.at_composite2(n, T![-], T![=]),
            //T![->] => self.at_composite2(n, T![-], T![>]),
            T![::] => self.at_composite2(n, T![:], T![:]),
            T![!=] => self.at_composite2(n, T![!], T![=]),
            T![..] => self.at_composite2(n, T![.], T![.]),
            T![*=] => self.at_composite2(n, T![*], T![=]),
            T![/=] => self.at_composite2(n, T![/], T![=]),
            //T![&&] => self.at_composite2(n, T![&], T![&]),
            //T![&=] => self.at_composite2(n, T![&], T![=]),
            //T![%=] => self.at_composite2(n, T![%], T![=]),
            //T![^=] => self.at_composite2(n, T![^], T![=]),
            T![+=] => self.at_composite2(n, T![+], T![=]),
            //T![<<] => self.at_composite2(n, T![<], T![<]),
            T![<=] => self.at_composite2(n, T![<], T![=]),
            T![==] => self.at_composite2(n, T![=], T![=]),
            //T![=>] => self.at_composite2(n, T![=], T![>]),
            T![>=] => self.at_composite2(n, T![>], T![=]),
            //T![>>] => self.at_composite2(n, T![>], T![>]),
            //T![|=] => self.at_composite2(n, T![|], T![=]),
            //T![||] => self.at_composite2(n, T![|], T![|]),
            T![...] => self.at_composite3(n, T![.], T![.], T![.]),
            //T![..=] => self.at_composite3(n, T![.], T![.], T![=]),
            //T![<<=] => self.at_composite3(n, T![<], T![<], T![=]),
            //T![>>=] => self.at_composite3(n, T![>], T![>], T![=]),
            _ => self.token_source.lookahead_nth(n).kind == kind,
        }
    }

    fn at_composite2(&self, n: usize, k1: SyntaxKind, k2: SyntaxKind) -> bool {
        let t1 = self.token_source.lookahead_nth(n + 0);
        let t2 = self.token_source.lookahead_nth(n + 1);
        t1.kind == k1 && t1.is_jointed_to_next && t2.kind == k2
    }

    fn at_composite3(&self, n: usize, k1: SyntaxKind, k2: SyntaxKind, k3: SyntaxKind) -> bool {
        let t1 = self.token_source.lookahead_nth(n + 0);
        let t2 = self.token_source.lookahead_nth(n + 1);
        let t3 = self.token_source.lookahead_nth(n + 2);
        (t1.kind == k1 && t1.is_jointed_to_next)
            && (t2.kind == k2 && t2.is_jointed_to_next)
            && t3.kind == k3
    }

    /// Checks if the current token is in `kinds`.
    pub(crate) fn at_ts(&self, kinds: TokenSet) -> bool {
        kinds.contains(self.current())
    }

    //    /// Checks if the current token is contextual keyword with text `t`.
    //    pub(crate) fn at_contextual_kw(&self, kw: &str) -> bool {
    //        self.token_source.is_keyword(kw)
    //    }

    /// Starts a new node in the syntax tree. All nodes and tokens consumed between the `start` and
    /// the corresponding `Marker::complete` belong to the same node.
    pub(crate) fn start(&mut self) -> Marker {
        let pos = self.events.len() as u32;
        self.push_event(Event::tombstone());
        Marker::new(pos)
    }

    /// Consume the next token if `kind` matches.
    pub(crate) fn bump(&mut self, kind: SyntaxKind) {
        assert!(self.eat(kind), "kind != {:?}", kind);
    }

    /// Advances the parser by one token with composite puncts handled
    pub(crate) fn bump_any(&mut self) {
        let kind = self.nth(0);
        if kind == EOF {
            return;
        }
        self.do_bump(kind, 1)
    }

    fn do_bump(&mut self, kind: SyntaxKind, n_raw_tokens: u8) {
        for _ in 0..n_raw_tokens {
            self.token_source.bump();
        }
        self.push_event(Event::Token { kind, n_raw_tokens });
    }

    /// Emit error with the `message`
    pub(crate) fn error<T: Into<String>>(&mut self, message: T) {
        let msg = ParseError(message.into());
        self.push_event(Event::Error { msg });
    }

    /// Create an error node and consume the next token.
    pub(crate) fn error_and_bump(&mut self, message: &str) {
        self.error_recover(message, TokenSet::empty())
    }

    /// Create an error node and consume the next token.
    pub(crate) fn error_recover(&mut self, message: &str, recovery: TokenSet) {
        if self.at(T!['{']) || self.at(T!['{']) || self.at_ts(recovery) {
            self.error(message);
        } else {
            let m = self.start();
            self.error(message);
            self.bump_any();
            m.complete(self, ERROR);
        }
    }

    fn push_event(&mut self, event: Event) {
        self.events.push(event)
    }

    /// Consume the next token if `kind` matches.
    pub(crate) fn eat(&mut self, kind: SyntaxKind) -> bool {
        if !self.at(kind) {
            return false;
        }
        let n_raw_tokens = match kind {
            T![-=]
            //| T![->]
            | T![::]
            | T![!=]
            | T![..]
            | T![*=]
            | T![/=]
            //| T![&&]
            //| T![&=]
            //| T![%=]
            //| T![^=]
            | T![+=]
            //| T![<<]
            | T![<=]
            | T![==]
            //| T![=>]
            | T![>=]
            //| T![>>]
            //| T![|=]
            //| T![||]
            => 2,

            T![...]
            //| T![..=]
            //| T![<<=]
            //| T![>>=]
            => 3,
            _ => 1,
        };
        self.do_bump(kind, n_raw_tokens);
        true
    }

    /// Consume the next token if it is `kind` or emit an error otherwise.
    pub(crate) fn expect(&mut self, kind: SyntaxKind) -> bool {
        if self.eat(kind) {
            return true;
        }
        self.error(format!("expected {:?}", kind));
        false
    }
}

/// See `Parser::start`
pub(crate) struct Marker {
    pos: u32,
    bomb: DropBomb,
}

impl Marker {
    fn new(pos: u32) -> Marker {
        Marker {
            pos,
            bomb: DropBomb::new("Marker must be either completed or abandoned"),
        }
    }

    /// Finishes the syntax tree node and assigns `kind` to it, and create a `CompletedMarker` for
    /// possible future operation like `.precede()` to deal with forward_parent.
    pub(crate) fn complete(mut self, p: &mut Parser, kind: SyntaxKind) -> CompletedMarker {
        self.bomb.defuse();
        let idx = self.pos as usize;
        match p.events[idx] {
            Event::Start {
                kind: ref mut slot, ..
            } => {
                *slot = kind;
            }
            _ => unreachable!(),
        }
        let finish_pos = p.events.len() as u32;
        p.push_event(Event::Finish);
        CompletedMarker::new(self.pos, finish_pos, kind)
    }

    /// Abandons the syntax tree node. All its children are attached to its parent instead.
    pub(crate) fn abandon(mut self, p: &mut Parser) {
        self.bomb.defuse();
        let idx = self.pos as usize;
        if idx == p.events.len() - 1 {
            match p.events.pop() {
                Some(Event::Start {
                    kind: TOMBSTONE,
                    forward_parent: None,
                }) => (),
                _ => unreachable!(),
            }
        }
    }
}

pub(crate) struct CompletedMarker {
    start_pos: u32,
    finish_pos: u32,
    kind: SyntaxKind,
}

impl CompletedMarker {
    fn new(start_pos: u32, finish_pos: u32, kind: SyntaxKind) -> Self {
        CompletedMarker {
            start_pos,
            finish_pos,
            kind,
        }
    }

    /// This method allows to create a new node which starts *before* the current one. That is,
    /// the parser could start node `A`, then complete it, and then after parsing the whole `A`,
    /// decide that it should have started some node `B` before starting `A`. `precede` allows to
    /// do exactly that. See also docs about `forward_parent` in `Event::Start`.
    ///
    /// Given completed events `[START, FINISH]` and its corresponding `CompletedMarker(pos: 0, _)`,
    /// append a new `START` event as `[START, FINISH, NEWSTART]`, then mark `NEWSTART` as `START`'s
    /// parent with saving its relative distance to `NEWSTART` into forward_parent(=2 in this case).
    pub(crate) fn precede(self, p: &mut Parser) -> Marker {
        let new_pos = p.start();
        let idx = self.start_pos as usize;
        match p.events[idx] {
            Event::Start {
                ref mut forward_parent,
                ..
            } => {
                *forward_parent = Some(new_pos.pos - self.start_pos);
            }
            _ => unreachable!(),
        }
        new_pos
    }

    /// Undo this completion and turns into a `Marker`
    pub(crate) fn undo_completion(self, p: &mut Parser) -> Marker {
        let start_idx = self.start_pos as usize;
        let finish_idx = self.finish_pos as usize;
        match p.events[start_idx] {
            Event::Start {
                ref mut kind,
                forward_parent: None,
            } => *kind = TOMBSTONE,
            _ => unreachable!(),
        }
        match p.events[finish_idx] {
            ref mut slot @ Event::Finish => *slot = Event::tombstone(),
            _ => unreachable!(),
        }
        Marker::new(self.start_pos)
    }

    pub(crate) fn kind(&self) -> SyntaxKind {
        self.kind
    }
}
