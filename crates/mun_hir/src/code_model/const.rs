use crate::{ids::ConstId, DiagnosticSink, HirDatabase};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Const {
    pub(crate) id: ConstId,
}
