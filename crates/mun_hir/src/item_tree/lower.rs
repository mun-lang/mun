//! This module implements the logic to convert an AST to an `ItemTree`.

use std::{collections::HashMap, convert::TryInto, marker::PhantomData, sync::Arc};

use mun_syntax::ast::{
    self, ExternOwner, ModuleItemOwner, NameOwner, StructKind, TypeAscriptionOwner,
};
use smallvec::SmallVec;

use super::{
    diagnostics, AssociatedItem, Field, Fields, Function, IdRange, Impl, ItemTree, ItemTreeData,
    ItemTreeNode, ItemVisibilities, LocalItemTreeId, ModItem, Param, ParamAstId, RawVisibilityId,
    Struct, TypeAlias,
};
use crate::{
    arena::{Idx, RawId},
    item_tree::Import,
    name::AsName,
    source_id::AstIdMap,
    type_ref::{TypeRefMap, TypeRefMapBuilder},
    visibility::RawVisibility,
    DefDatabase, FileId, Name, Path,
};

struct ModItems(SmallVec<[ModItem; 1]>);

impl<T> From<T> for ModItems
where
    T: Into<ModItem>,
{
    fn from(t: T) -> Self {
        ModItems(SmallVec::from_buf([t.into(); 1]))
    }
}

impl<N: ItemTreeNode> From<Idx<N>> for LocalItemTreeId<N> {
    fn from(index: Idx<N>) -> Self {
        LocalItemTreeId {
            index,
            _p: PhantomData,
        }
    }
}

pub(super) struct Context {
    file: FileId,
    source_ast_id_map: Arc<AstIdMap>,
    data: ItemTreeData,
    diagnostics: Vec<diagnostics::ItemTreeDiagnostic>,
}

impl Context {
    /// Constructs a new `Context` for the specified file
    pub(super) fn new(db: &dyn DefDatabase, file: FileId) -> Self {
        Self {
            file,
            source_ast_id_map: db.ast_id_map(file),
            data: ItemTreeData::default(),
            diagnostics: Vec::new(),
        }
    }

    /// Lowers all the items in the specified `ModuleItemOwner` and returns an
    /// `ItemTree`
    pub(super) fn lower_module_items(mut self, item_owner: &impl ModuleItemOwner) -> ItemTree {
        let top_level = item_owner
            .items()
            .filter_map(|item| self.lower_mod_item(&item))
            .flat_map(|items| items.0)
            .collect::<Vec<_>>();

        // Check duplicates
        let mut set = HashMap::<Name, &ModItem>::new();
        for item in top_level.iter() {
            let name = match item {
                ModItem::Function(item) => Some(&self.data.functions[item.index].name),
                ModItem::Struct(item) => Some(&self.data.structs[item.index].name),
                ModItem::TypeAlias(item) => Some(&self.data.type_aliases[item.index].name),
                ModItem::Import(item) => {
                    let import = &self.data.imports[item.index];
                    if import.is_glob {
                        None
                    } else {
                        import
                            .alias
                            .as_ref()
                            .map_or_else(|| import.path.last_segment(), |alias| alias.as_name())
                    }
                }
                ModItem::Impl(_) => None,
            };
            if let Some(name) = name {
                if let Some(first_item) = set.get(name) {
                    self.diagnostics
                        .push(diagnostics::ItemTreeDiagnostic::DuplicateDefinition {
                            name: name.clone(),
                            first: **first_item,
                            second: *item,
                        });
                } else {
                    set.insert(name.clone(), item);
                }
            }
        }

        ItemTree {
            file_id: self.file,
            top_level,
            data: self.data,
            diagnostics: self.diagnostics,
        }
    }

    /// Lowers a single module item
    fn lower_mod_item(&mut self, item: &ast::ModuleItem) -> Option<ModItems> {
        match item.kind() {
            ast::ModuleItemKind::FunctionDef(ast) => self.lower_function(&ast).map(Into::into),
            ast::ModuleItemKind::StructDef(ast) => self.lower_struct(&ast).map(Into::into),
            ast::ModuleItemKind::TypeAliasDef(ast) => self.lower_type_alias(&ast).map(Into::into),
            ast::ModuleItemKind::Use(ast) => Some(ModItems(
                self.lower_use(&ast).into_iter().map(Into::into).collect(),
            )),
            ast::ModuleItemKind::Impl(ast) => self.lower_impl(&ast).map(Into::into),
        }
    }

    /// Lowers a `use` statement
    fn lower_use(&mut self, use_item: &ast::Use) -> Vec<LocalItemTreeId<Import>> {
        let visibility = lower_visibility(use_item);
        let ast_id = self.source_ast_id_map.ast_id(use_item);

        // Every use item can expand to many `Import`s.
        let mut imports = Vec::new();
        let tree = &mut self.data;
        Path::expand_use_item(use_item, |path, _use_tree, is_glob, alias| {
            imports.push(
                tree.imports
                    .alloc(Import {
                        path,
                        alias,
                        visibility,
                        is_glob,
                        ast_id,
                        index: imports.len(),
                    })
                    .into(),
            );
        });

        imports
    }

    /// Lowers a function
    fn lower_function(&mut self, func: &ast::FunctionDef) -> Option<LocalItemTreeId<Function>> {
        let name = func.name()?.as_name();
        let visibility = lower_visibility(func);
        let mut types = TypeRefMap::builder();

        // Lower all the params
        let start_param_idx = self.next_param_idx();
        if let Some(param_list) = func.param_list() {
            for param in param_list.params() {
                let ast_id = self.source_ast_id_map.ast_id(&param);
                let type_ref = types.alloc_from_node_opt(param.ascribed_type().as_ref());
                self.data.params.alloc(Param {
                    type_ref,
                    ast_id: ParamAstId::Param(ast_id),
                });
            }
        }
        let end_param_idx = self.next_param_idx();
        let params = IdRange::new(start_param_idx..end_param_idx);

        // Lowers the return type
        let ret_type = match func.ret_type().and_then(|rt| rt.type_ref()) {
            None => types.unit(),
            Some(ty) => types.alloc_from_node(&ty),
        };

        let is_extern = func.is_extern();

        let (types, _types_source_map) = types.finish();
        let ast_id = self.source_ast_id_map.ast_id(func);
        let res = Function {
            name,
            visibility,
            is_extern,
            types,
            params,
            ret_type,
            ast_id,
        };

        Some(self.data.functions.alloc(res).into())
    }

    /// Lowers a struct
    fn lower_struct(&mut self, strukt: &ast::StructDef) -> Option<LocalItemTreeId<Struct>> {
        let name = strukt.name()?.as_name();
        let visibility = lower_visibility(strukt);
        let mut types = TypeRefMap::builder();
        let fields = self.lower_fields(&strukt.kind(), &mut types);
        let ast_id = self.source_ast_id_map.ast_id(strukt);

        let (types, _types_source_map) = types.finish();
        let res = Struct {
            name,
            visibility,
            types,
            fields,
            ast_id,
        };
        Some(self.data.structs.alloc(res).into())
    }

    /// Lowers the fields of a struct or enum
    fn lower_fields(
        &mut self,
        struct_kind: &ast::StructKind,
        types: &mut TypeRefMapBuilder,
    ) -> Fields {
        match struct_kind {
            StructKind::Record(it) => {
                let range = self.lower_record_fields(it, types);
                Fields::Record(range)
            }
            StructKind::Tuple(it) => {
                let range = self.lower_tuple_fields(it, types);
                Fields::Tuple(range)
            }
            StructKind::Unit => Fields::Unit,
        }
    }

    /// Lowers records fields (e.g. `{ a: i32, b: i32 }`)
    fn lower_record_fields(
        &mut self,
        fields: &ast::RecordFieldDefList,
        types: &mut TypeRefMapBuilder,
    ) -> IdRange<Field> {
        let start = self.next_field_idx();
        for field in fields.fields() {
            if let Some(data) = lower_record_field(&field, types) {
                let _idx = self.data.fields.alloc(data);
            }
        }
        let end = self.next_field_idx();
        IdRange::new(start..end)
    }

    /// Lowers tuple fields (e.g. `(i32, u8)`)
    fn lower_tuple_fields(
        &mut self,
        fields: &ast::TupleFieldDefList,
        types: &mut TypeRefMapBuilder,
    ) -> IdRange<Field> {
        let start = self.next_field_idx();
        for (i, field) in fields.fields().enumerate() {
            let data = lower_tuple_field(i, &field, types);
            let _idx = self.data.fields.alloc(data);
        }
        let end = self.next_field_idx();
        IdRange::new(start..end)
    }

    /// Lowers a type alias (e.g. `type Foo = Bar`)
    fn lower_type_alias(
        &mut self,
        type_alias: &ast::TypeAliasDef,
    ) -> Option<LocalItemTreeId<TypeAlias>> {
        let name = type_alias.name()?.as_name();
        let visibility = lower_visibility(type_alias);
        let mut types = TypeRefMap::builder();
        let type_ref = type_alias.type_ref().map(|ty| types.alloc_from_node(&ty));
        let ast_id = self.source_ast_id_map.ast_id(type_alias);
        let (types, _types_source_map) = types.finish();
        let res = TypeAlias {
            name,
            visibility,
            types,
            type_ref,
            ast_id,
        };
        Some(self.data.type_aliases.alloc(res).into())
    }

    fn lower_impl(&mut self, impl_def: &ast::Impl) -> Option<LocalItemTreeId<Impl>> {
        let ast_id = self.source_ast_id_map.ast_id(impl_def);
        let mut types = TypeRefMap::builder();
        let self_ty = impl_def.type_ref().map(|ty| types.alloc_from_node(&ty))?;

        let items = impl_def
            .associated_item_list()
            .into_iter()
            .flat_map(|it| it.associated_items())
            .filter_map(|item| self.lower_associated_item(&item))
            .collect();

        let (types, _types_source_map) = types.finish();

        let res = Impl {
            types,
            self_ty,
            items,
            ast_id,
        };

        Some(self.data.impls.alloc(res).into())
    }

    fn lower_associated_item(&mut self, item: &ast::AssociatedItem) -> Option<AssociatedItem> {
        let item: AssociatedItem = match item.kind() {
            ast::AssociatedItemKind::FunctionDef(ast) => self.lower_function(&ast).map(Into::into),
        }?;
        Some(item)
    }

    /// Returns the `Idx` of the next `Field`
    fn next_field_idx(&self) -> Idx<Field> {
        let idx: u32 = self.data.fields.len().try_into().expect("too many fields");
        Idx::from_raw(RawId::from(idx))
    }

    /// Returns the `Idx` of the next `Param`
    fn next_param_idx(&self) -> Idx<Param> {
        let idx: u32 = self.data.params.len().try_into().expect("too many params");
        Idx::from_raw(RawId::from(idx))
    }
}

/// Lowers a record field (e.g. `a:i32`)
fn lower_record_field(field: &ast::RecordFieldDef, types: &mut TypeRefMapBuilder) -> Option<Field> {
    let name = field.name()?.as_name();
    let type_ref = types.alloc_from_node_opt(field.ascribed_type().as_ref());
    let res = Field { name, type_ref };
    Some(res)
}

/// Lowers a tuple field (e.g. `i32`)
fn lower_tuple_field(
    idx: usize,
    field: &ast::TupleFieldDef,
    types: &mut TypeRefMapBuilder,
) -> Field {
    let name = Name::new_tuple_field(idx);
    let type_ref = types.alloc_from_node_opt(field.type_ref().as_ref());
    Field { name, type_ref }
}

/// Lowers an `ast::VisibilityOwner`
fn lower_visibility(item: &impl ast::VisibilityOwner) -> RawVisibilityId {
    let vis = RawVisibility::from_ast(item.visibility());
    ItemVisibilities::alloc(vis)
}
