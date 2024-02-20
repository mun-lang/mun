mod function;

use function::FunctionRender;
use mun_hir::{semantics::ScopeDef, HirDisplay};

use super::{CompletionContext, CompletionItem, CompletionItemKind};
use crate::{completion::CompletionKind, db::AnalysisDatabase, SymbolKind};

pub(super) fn render_field(ctx: RenderContext<'_>, field: mun_hir::Field) -> CompletionItem {
    Render::new(ctx).render_field(field)
}

pub(super) fn render_fn(
    ctx: RenderContext<'_>,
    local_name: Option<String>,
    func: mun_hir::Function,
) -> Option<CompletionItem> {
    Some(FunctionRender::new(ctx, local_name, func)?.render())
}

pub(super) fn render_resolution(
    ctx: RenderContext<'_>,
    local_name: String,
    resolution: &ScopeDef,
) -> Option<CompletionItem> {
    Render::new(ctx).render_resolution(local_name, resolution)
}

/// Interface for data and methods required for items rendering.
pub(super) struct RenderContext<'a> {
    completion: &'a CompletionContext<'a>,
}

impl<'a> RenderContext<'a> {
    pub(super) fn new(completion: &'a CompletionContext<'a>) -> RenderContext<'a> {
        RenderContext { completion }
    }

    pub(super) fn db(&self) -> &'a AnalysisDatabase {
        self.completion.db
    }
}

/// Generic renderer for completion items.
struct Render<'a> {
    ctx: RenderContext<'a>,
}

impl<'a> Render<'a> {
    fn new(ctx: RenderContext<'a>) -> Render<'a> {
        Render { ctx }
    }

    /// Constructs a `CompletionItem` for a resolved name.
    fn render_resolution(
        self,
        local_name: String,
        resolution: &ScopeDef,
    ) -> Option<CompletionItem> {
        use mun_hir::ModuleDef::{Function, Module, PrimitiveType, Struct, TypeAlias};

        let completion_kind = match resolution {
            ScopeDef::ModuleDef(PrimitiveType(..)) => CompletionKind::BuiltinType,
            _ => CompletionKind::Reference,
        };

        let kind = match resolution {
            ScopeDef::ModuleDef(Module(_)) => CompletionItemKind::SymbolKind(SymbolKind::Module),
            ScopeDef::ModuleDef(Function(func)) => {
                return render_fn(self.ctx, Some(local_name), *func)
            }
            ScopeDef::ModuleDef(PrimitiveType(_)) => CompletionItemKind::BuiltinType,
            ScopeDef::ModuleDef(Struct(_)) => CompletionItemKind::SymbolKind(SymbolKind::Struct),
            ScopeDef::ModuleDef(TypeAlias(_)) => {
                CompletionItemKind::SymbolKind(SymbolKind::TypeAlias)
            }
            ScopeDef::Local(_) => CompletionItemKind::SymbolKind(SymbolKind::Local),
            ScopeDef::Unknown => {
                let item = CompletionItem::builder(CompletionKind::Reference, local_name)
                    .kind(CompletionItemKind::UnresolvedReference)
                    .finish();
                return Some(item);
            }
        };

        let mut item = CompletionItem::builder(completion_kind, local_name);

        // Add the type for locals
        if let ScopeDef::Local(local) = resolution {
            let ty = local.ty(self.ctx.db());
            if !ty.is_unknown() {
                item = item.detail(ty.display(self.ctx.db()).to_string());
            }
        };

        Some(item.kind(kind).finish())
    }

    /// Constructs a `CompletionItem` for a field.
    fn render_field(&mut self, field: mun_hir::Field) -> CompletionItem {
        let name = field.name(self.ctx.db());
        CompletionItem::builder(CompletionKind::Reference, name.to_string())
            .kind(SymbolKind::Field)
            .detail(field.ty(self.ctx.db()).display(self.ctx.db()).to_string())
            .finish()
    }
}
