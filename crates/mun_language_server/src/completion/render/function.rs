use mun_hir::HirDisplay;

use super::{CompletionItem, RenderContext};
use crate::SymbolKind;

/// Similar to [`Render<'a>`] but used to render a completion item for a
/// function
pub(super) struct FunctionRender<'a> {
    ctx: RenderContext<'a>,
    name: String,
    func: mun_hir::Function,
}

impl<'a> FunctionRender<'a> {
    /// Constructs a new `FunctionRender`
    pub fn new(
        ctx: RenderContext<'a>,
        local_name: Option<String>,
        func: mun_hir::Function,
    ) -> Option<FunctionRender<'a>> {
        let name = local_name.unwrap_or_else(|| func.name(ctx.db()).to_string());

        Some(Self { ctx, name, func })
    }

    /// Constructs a [`CompletionItem`] for the wrapped function.
    pub fn render(self) -> CompletionItem {
        CompletionItem::builder(SymbolKind::Function, self.name.clone())
            .with_detail(self.detail())
            .finish()
    }

    /// Returns the detail text to add to the completion. This currently returns
    /// `-> <ret_ty>`.
    fn detail(&self) -> String {
        let ty = self.func.ret_type(self.ctx.db());
        format!("-> {}", ty.display(self.ctx.db()))
    }
}
