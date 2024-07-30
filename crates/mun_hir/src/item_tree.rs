mod lower;
mod pretty;
#[cfg(test)]
mod tests;

use std::{
    any::type_name,
    fmt,
    fmt::Formatter,
    hash::{Hash, Hasher},
    marker::PhantomData,
    ops::{Index, Range},
    sync::Arc,
};

use la_arena::{Arena, Idx};
use mun_hir_input::FileId;
use mun_syntax::ast;

use crate::{
    path::ImportAlias,
    source_id::{AstIdNode, FileAstId},
    type_ref::{LocalTypeRefId, TypeRefMap},
    visibility::RawVisibility,
    DefDatabase, InFile, Name, Path,
};

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct RawVisibilityId(u32);

impl RawVisibilityId {
    pub const PUB: Self = RawVisibilityId(u32::max_value());
    pub const PRIV: Self = RawVisibilityId(u32::max_value() - 1);
    pub const PUB_PACKAGE: Self = RawVisibilityId(u32::max_value() - 2);
    pub const PUB_SUPER: Self = RawVisibilityId(u32::max_value() - 3);
}

impl fmt::Debug for RawVisibilityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut f = f.debug_tuple("RawVisibilityId");
        match *self {
            Self::PUB => f.field(&"pub"),
            Self::PRIV => f.field(&"pub(self)"),
            Self::PUB_PACKAGE => f.field(&"pub(package)"),
            Self::PUB_SUPER => f.field(&"pub(super)"),
            _ => f.field(&self.0),
        };
        f.finish()
    }
}

/// An `ItemTree` is a derivative of an AST that only contains the items defined
/// in the AST.
///
/// Examples of items are: functions, structs, use statements.
#[derive(Debug, Eq, PartialEq)]
pub struct ItemTree {
    file_id: FileId,
    top_level: Vec<ModItem>,
    data: ItemTreeData,

    pub diagnostics: Vec<diagnostics::ItemTreeDiagnostic>,
}

impl ItemTree {
    /// Constructs a new `ItemTree` for the specified `file_id`
    pub fn item_tree_query(db: &dyn DefDatabase, file_id: FileId) -> Arc<ItemTree> {
        let syntax = db.parse(file_id);
        let item_tree = lower::Context::new(db, file_id).lower_module_items(&syntax.tree());
        Arc::new(item_tree)
    }

    /// Returns a slice over all items located at the top level of the `FileId`
    /// for which this `ItemTree` was constructed.
    pub fn top_level_items(&self) -> &[ModItem] {
        &self.top_level
    }

    /// Returns the source location of the specified item. Note that the
    /// `file_id` of the item must be the same `file_id` that was used to
    /// create this `ItemTree`.
    pub fn source<S: ItemTreeNode>(
        &self,
        db: &dyn DefDatabase,
        item: LocalItemTreeId<S>,
    ) -> S::Source {
        let root = db.parse(self.file_id);

        let id = self[item].ast_id();
        let map = db.ast_id_map(self.file_id);
        let ptr = map.get(id);
        ptr.to_node(&root.syntax_node())
    }
}

#[derive(Default, Debug, Eq, PartialEq)]
struct ItemVisibilities {
    arena: Arena<RawVisibility>,
}

impl ItemVisibilities {
    fn alloc(vis: RawVisibility) -> RawVisibilityId {
        match &vis {
            RawVisibility::Public => RawVisibilityId::PUB,
            RawVisibility::This => RawVisibilityId::PRIV,
            RawVisibility::Package => RawVisibilityId::PUB_PACKAGE,
            RawVisibility::Super => RawVisibilityId::PUB_SUPER,
        }
    }
}

#[derive(Default, Debug, Eq, PartialEq)]
struct ItemTreeData {
    imports: Arena<Import>,
    functions: Arena<Function>,
    params: Arena<Param>,
    structs: Arena<Struct>,
    fields: Arena<Field>,
    type_aliases: Arena<TypeAlias>,
    impls: Arena<Impl>,

    visibilities: ItemVisibilities,
}

/// Trait implemented by all item nodes in the item tree.
pub trait ItemTreeNode: Clone {
    type Source: AstIdNode + Into<ast::ModuleItem>;

    /// Returns the AST id for this instance
    fn ast_id(&self) -> FileAstId<Self::Source>;

    /// Looks up an instance of `Self` in an item tree.
    fn lookup(tree: &ItemTree, index: Idx<Self>) -> &Self;

    /// Downcasts a `ModItem` to a `FileItemTreeId` specific to this type
    fn id_from_mod_item(mod_item: ModItem) -> Option<LocalItemTreeId<Self>>;

    /// Upcasts a `FileItemTreeId` to a generic [`ModItem`].
    fn id_to_mod_item(id: LocalItemTreeId<Self>) -> ModItem;
}

/// The typed Id of an item in an `ItemTree`
#[derive(Clone)]
pub struct LocalItemTreeId<N: ItemTreeNode> {
    index: Idx<N>,
    _p: PhantomData<N>,
}

impl<N: ItemTreeNode> Copy for LocalItemTreeId<N> {}

impl<N: ItemTreeNode> PartialEq for LocalItemTreeId<N> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index
    }
}

impl<N: ItemTreeNode> Eq for LocalItemTreeId<N> {}

impl<N: ItemTreeNode> Hash for LocalItemTreeId<N> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.index.hash(state);
    }
}

impl<N: ItemTreeNode> fmt::Debug for LocalItemTreeId<N> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.index.fmt(f)
    }
}

/// Represents the Id of an item in the [`ItemTree`] of a file.
pub type ItemTreeId<N> = InFile<LocalItemTreeId<N>>;

macro_rules! mod_items {
    ( $( $typ:ident in $fld:ident -> $ast:ty ),+ $(,)?) => {
        #[derive(Debug,Copy,Clone,Eq,PartialEq,Hash)]
        pub enum ModItem {
            $(
                $typ(LocalItemTreeId<$typ>),
            )+
        }

        $(
            impl From<LocalItemTreeId<$typ>> for ModItem {
                fn from(id: LocalItemTreeId<$typ>) -> ModItem {
                    ModItem::$typ(id)
                }
            }
        )+

        $(
            impl ItemTreeNode for $typ {
                type Source = $ast;

                fn ast_id(&self) -> FileAstId<Self::Source> {
                    self.ast_id
                }

                fn lookup(tree: &ItemTree, index: Idx<Self>) -> &Self {
                    &tree.data.$fld[index]
                }

                fn id_from_mod_item(mod_item: ModItem) -> Option<LocalItemTreeId<Self>> {
                    if let ModItem::$typ(id) = mod_item {
                        Some(id)
                    } else {
                        None
                    }
                }

                fn id_to_mod_item(id: LocalItemTreeId<Self>) -> ModItem {
                    ModItem::$typ(id)
                }
            }

            impl Index<Idx<$typ>> for ItemTree {
                type Output = $typ;

                fn index(&self, index: Idx<$typ>) -> &Self::Output {
                    &self.data.$fld[index]
                }
            }
        )+
    };
}

mod_items! {
    Function in functions -> ast::FunctionDef,
    Struct in structs -> ast::StructDef,
    TypeAlias in type_aliases -> ast::TypeAliasDef,
    Import in imports -> ast::Use,
    Impl in impls -> ast::Impl,
}

macro_rules! impl_index {
    ( $($fld:ident: $t:ty),+ $(,)? ) => {
        $(
            impl Index<Idx<$t>> for ItemTree {
                type Output = $t;

                fn index(&self, index: Idx<$t>) -> &Self::Output {
                    &self.data.$fld[index]
                }
            }
        )+
    };
}

impl_index!(fields: Field, params: Param);

static VIS_PUB: RawVisibility = RawVisibility::Public;
static VIS_PRIV: RawVisibility = RawVisibility::This;
static VIS_PUB_PACKAGE: RawVisibility = RawVisibility::Package;
static VIS_PUB_SUPER: RawVisibility = RawVisibility::Super;

impl Index<RawVisibilityId> for ItemTree {
    type Output = RawVisibility;
    fn index(&self, index: RawVisibilityId) -> &Self::Output {
        match index {
            RawVisibilityId::PRIV => &VIS_PRIV,
            RawVisibilityId::PUB => &VIS_PUB,
            RawVisibilityId::PUB_PACKAGE => &VIS_PUB_PACKAGE,
            RawVisibilityId::PUB_SUPER => &VIS_PUB_SUPER,
            _ => &self.data.visibilities.arena[Idx::from_raw(index.0.into())],
        }
    }
}

impl<N: ItemTreeNode> Index<LocalItemTreeId<N>> for ItemTree {
    type Output = N;
    fn index(&self, id: LocalItemTreeId<N>) -> &N {
        N::lookup(self, id.index)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Import {
    /// The path of the import (e.g. foo::Bar). Note that group imports have
    /// been desugared, each item in the import tree is a seperate import.
    pub path: Path,

    /// An optional alias for this import statement (e.g. `use foo as bar`)
    pub alias: Option<ImportAlias>,

    /// The visibility of the import statement as seen from the file that
    /// contains the import statement.
    pub visibility: RawVisibilityId,

    /// Whether or not this is a wildcard import.
    pub is_glob: bool,

    /// AST Id of the `use` item this import was derived from. Note that
    /// multiple `Import`s can map to the same `use` item.
    pub ast_id: FileAstId<ast::Use>,

    /// Index of this `Import` when the containing `Use` is visited with
    /// `Path::expand_use_item`.
    pub index: usize,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Function {
    pub name: Name,
    pub visibility: RawVisibilityId,
    pub types: TypeRefMap,
    pub params: IdRange<Param>,
    pub ret_type: LocalTypeRefId,
    pub ast_id: FileAstId<ast::FunctionDef>,
    pub(crate) flags: FunctionFlags,
}

bitflags::bitflags! {
    #[doc = "Flags that are used to store additional information about a function"]
    #[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
    pub(crate) struct FunctionFlags: u8 {
        const HAS_SELF_PARAM = 1 << 0;
        const HAS_BODY = 1 << 1;
        const IS_EXTERN = 1 << 2;
    }
}

impl FunctionFlags {
    /// Whether the function has a self parameter.
    pub fn has_self_param(self) -> bool {
        self.contains(Self::HAS_SELF_PARAM)
    }

    /// Whether the function has a body.
    pub fn has_body(self) -> bool {
        self.contains(Self::HAS_BODY)
    }

    /// Whether the function is extern.
    pub fn is_extern(self) -> bool {
        self.contains(Self::IS_EXTERN)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Param {
    pub type_ref: LocalTypeRefId,
    pub ast_id: ParamAstId,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ParamAstId {
    Param(FileAstId<ast::Param>),
    SelfParam(FileAstId<ast::SelfParam>),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Struct {
    pub name: Name,
    pub visibility: RawVisibilityId,
    pub types: TypeRefMap,
    pub fields: Fields,
    pub ast_id: FileAstId<ast::StructDef>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Impl {
    pub types: TypeRefMap,
    pub self_ty: LocalTypeRefId,
    pub items: Box<[AssociatedItem]>,
    pub ast_id: FileAstId<ast::Impl>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeAlias {
    pub name: Name,
    pub visibility: RawVisibilityId,
    pub types: TypeRefMap,
    pub type_ref: Option<LocalTypeRefId>,
    pub ast_id: FileAstId<ast::TypeAliasDef>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum AssociatedItem {
    Function(LocalItemTreeId<Function>),
}

impl From<LocalItemTreeId<Function>> for AssociatedItem {
    fn from(value: LocalItemTreeId<Function>) -> Self {
        AssociatedItem::Function(value)
    }
}

impl From<AssociatedItem> for ModItem {
    fn from(item: AssociatedItem) -> Self {
        match item {
            AssociatedItem::Function(it) => it.into(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum StructDefKind {
    /// `struct S { ... }` - type namespace only.
    Record,
    /// `struct S(...);`
    Tuple,
    /// `struct S;`
    Unit,
}

/// A set of fields
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fields {
    Record(IdRange<Field>),
    Tuple(IdRange<Field>),
    Unit,
}

/// A single field of an enum variant or struct
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub name: Name,
    pub type_ref: LocalTypeRefId,
}

/// A range of Ids
pub struct IdRange<T> {
    range: Range<u32>,
    _p: PhantomData<T>,
}

impl<T> IdRange<T> {
    fn new(range: Range<Idx<T>>) -> Self {
        Self {
            range: range.start.into_raw().into()..range.end.into_raw().into(),
            _p: PhantomData,
        }
    }

    /// Returns true if the index range is empty
    pub fn is_empty(&self) -> bool {
        self.range.is_empty()
    }
}

impl<T> Iterator for IdRange<T> {
    type Item = Idx<T>;
    fn next(&mut self) -> Option<Self::Item> {
        self.range.next().map(|raw| Idx::from_raw(raw.into()))
    }
}

impl<T> fmt::Debug for IdRange<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple(&format!("IdRange::<{}>", type_name::<T>()))
            .field(&self.range)
            .finish()
    }
}

impl<T> Clone for IdRange<T> {
    fn clone(&self) -> Self {
        Self {
            range: self.range.clone(),
            _p: PhantomData,
        }
    }
}

impl<T> PartialEq for IdRange<T> {
    fn eq(&self, other: &Self) -> bool {
        self.range == other.range
    }
}

impl<T> Eq for IdRange<T> {}

mod diagnostics {
    use mun_syntax::{AstNode, SyntaxNodePtr};

    use super::{ItemTree, ModItem};
    use crate::{
        diagnostics::DuplicateDefinition, DefDatabase, DiagnosticSink, HirDatabase, InFile, Name,
        Path,
    };

    #[derive(Clone, Debug, Eq, PartialEq)]
    pub enum ItemTreeDiagnostic {
        DuplicateDefinition {
            name: Name,
            first: ModItem,
            second: ModItem,
        },
    }

    impl ItemTreeDiagnostic {
        pub(crate) fn add_to(
            &self,
            db: &dyn HirDatabase,
            item_tree: &ItemTree,
            sink: &mut DiagnosticSink<'_>,
        ) {
            fn ast_ptr_from_mod(
                db: &dyn DefDatabase,
                item_tree: &ItemTree,
                item: ModItem,
            ) -> InFile<SyntaxNodePtr> {
                match item {
                    ModItem::Function(item) => InFile::new(
                        item_tree.file_id,
                        SyntaxNodePtr::new(item_tree.source(db, item).syntax()),
                    ),
                    ModItem::Struct(item) => InFile::new(
                        item_tree.file_id,
                        SyntaxNodePtr::new(item_tree.source(db, item).syntax()),
                    ),
                    ModItem::TypeAlias(item) => InFile::new(
                        item_tree.file_id,
                        SyntaxNodePtr::new(item_tree.source(db, item).syntax()),
                    ),
                    ModItem::Import(it) => {
                        let import = &item_tree[it];
                        let import_src = item_tree.source(db, it);
                        let mut use_item = None;
                        let mut index = 0;
                        Path::expand_use_item(&import_src, |_, tree, _, _| {
                            if index == import.index {
                                use_item = Some(tree.clone());
                            }
                            index += 1;
                        });
                        InFile::new(
                            item_tree.file_id,
                            SyntaxNodePtr::new(use_item.expect("cannot find use item").syntax()),
                        )
                    }
                    ModItem::Impl(_) => unreachable!("impls cannot be duplicated"),
                }
            }

            match self {
                ItemTreeDiagnostic::DuplicateDefinition {
                    name,
                    first,
                    second,
                } => sink.push(DuplicateDefinition {
                    name: name.to_string(),
                    first_definition: ast_ptr_from_mod(db.upcast(), item_tree, *first),
                    definition: ast_ptr_from_mod(db.upcast(), item_tree, *second),
                }),
            };
        }
    }
}
