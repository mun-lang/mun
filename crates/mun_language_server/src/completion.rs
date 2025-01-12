//! A module that provides completions based on the position of the cursor
//! (indicated as `$0` in the documentation).
//! The [`completions`] function is the main entry point for computing the
//! completions.

mod context;
// mod dot;
mod item;
mod render;
// mod unqualified_path;

mod dot;
mod expr;
mod name_ref;
#[cfg(test)]
mod test_utils;

use context::{
    CompletionAnalysis, CompletionContext, DotAccess, NameRefContext, NameRefKind,
    PathCompletionContext, PathExprContext, PathKind, Qualified,
};
pub use item::{CompletionItem, CompletionItemKind};
use mun_hir::semantics::ScopeDef;

use crate::{
    completion::render::{render_field, render_fn, render_resolution, RenderContext},
    db::AnalysisDatabase,
    FilePosition,
};

/// This is the main entry point for computing completions. This is a two step
/// process.
///
/// The first step is to determine the context of where the completion is
/// requested. This information is captured in the [`CompletionContext`]. The
/// context captures things like which type of syntax node is before the cursor
/// or the current scope.
///
/// Second is to compute a set of completions based on the previously computed
/// context. We provide several methods for computing completions based on
/// different syntax contexts. For instance when writing `foo.$0` you want to
/// complete the fields of `foo` and don't want the local variables of
/// the active scope.
pub(crate) fn completions(db: &AnalysisDatabase, position: FilePosition) -> Option<Completions> {
    let (context, analysis) = CompletionContext::new(db, position)?;

    let mut result = Completions::default();

    match analysis {
        CompletionAnalysis::NameRef(name_ref_ctx) => {
            name_ref::complete_name_ref(&mut result, &context, &name_ref_ctx);
        }
    }

    // unqualified_path::complete_unqualified_path(&mut result, &context);
    // dot::complete_dot(&mut result, &context);
    Some(result)
}

/// Represents an in-progress set of completions being built. Use the `add_..`
/// functions to quickly add completion items.
#[derive(Debug, Default)]
pub(crate) struct Completions {
    buf: Vec<CompletionItem>,
}

impl From<Completions> for Vec<CompletionItem> {
    fn from(completions: Completions) -> Self {
        completions.buf
    }
}

impl Completions {
    /// Adds a raw `CompletionItem`
    fn add(&mut self, item: CompletionItem) {
        self.buf.push(item);
    }

    /// Adds a completion item for a resolved name
    fn add_resolution(
        &mut self,
        ctx: &CompletionContext<'_>,
        local_name: String,
        resolution: &ScopeDef,
    ) {
        if let Some(item) = render_resolution(RenderContext::new(ctx), local_name, resolution) {
            self.add(item);
        }
    }

    /// Adds a completion item for a field
    fn add_field(&mut self, ctx: &CompletionContext<'_>, field: mun_hir::Field) {
        let item = render_field(RenderContext::new(ctx), field);
        self.add(item);
    }

    fn add_function(
        &mut self,
        ctx: &CompletionContext<'_>,
        func: mun_hir::Function,
        local_name: Option<String>,
    ) {
        if let Some(item) = render_fn(RenderContext::new(ctx), local_name, func) {
            self.add(item);
        }
    }
}
