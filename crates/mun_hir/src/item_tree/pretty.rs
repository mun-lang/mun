use std::{fmt, fmt::Write};

use crate::{
    item_tree::{
        Fields, Function, Impl, Import, ItemTree, LocalItemTreeId, ModItem, Param, RawVisibilityId,
        Struct, TypeAlias,
    },
    path::ImportAlias,
    pretty::{print_path, print_type_ref},
    type_ref::{LocalTypeRefId, TypeRefMap},
    visibility::RawVisibility,
    DefDatabase,
};

/// A helper method to print an `ItemTree` to a string.
pub(super) fn print_item_tree(db: &dyn DefDatabase, tree: &ItemTree) -> Result<String, fmt::Error> {
    let mut p = Printer {
        db,
        tree,
        buf: String::new(),
        indent_level: 0,
        needs_indent: true,
    };

    for item in tree.top_level_items() {
        p.print_mod_item(*item)?;
    }

    let mut s = p.buf.trim_end_matches('\n').to_string();
    s.push('\n');
    Ok(s)
}

/// A helper struct for [`print_item_tree`] that keeps track of the current
/// indentation level.
struct Printer<'a> {
    db: &'a dyn DefDatabase,
    tree: &'a ItemTree,
    buf: String,
    indent_level: usize,
    needs_indent: bool,
}

impl Printer<'_> {
    /// Run the specified closure with an increased indentation level.
    fn indented(&mut self, f: impl FnOnce(&mut Self) -> fmt::Result) -> fmt::Result {
        self.indent_level += 1;
        writeln!(self)?;
        f(self)?;
        self.indent_level -= 1;
        self.buf = self.buf.trim_end_matches('\n').to_string();
        Ok(())
    }

    // Add a whitespace to the end of the buffer if the last character is not a
    // newline or space.
    fn whitespace(&mut self) -> fmt::Result {
        match self.buf.chars().next_back() {
            None | Some('\n' | ' ') => {}
            _ => self.buf.push(' '),
        }
        Ok(())
    }

    /// Print a module item to the buffer.
    fn print_mod_item(&mut self, item: ModItem) -> fmt::Result {
        match item {
            ModItem::Function(it) => self.print_function(it),
            ModItem::Struct(it) => self.print_struct(it),
            ModItem::TypeAlias(it) => self.print_type_alias(it),
            ModItem::Import(it) => self.print_use(it),
            ModItem::Impl(it) => self.print_impl(it),
        }
    }

    /// Prints a use statement to the buffer.
    fn print_use(&mut self, it: LocalItemTreeId<Import>) -> fmt::Result {
        let Import {
            path,
            alias,
            visibility,
            is_glob,
            ast_id: _,
            index: _,
        } = &self.tree[it];
        self.print_visibility(*visibility)?;
        write!(self, "use ")?;
        print_path(self.db, path, self)?;
        if *is_glob {
            write!(self, "::*")?;
        }
        match alias {
            Some(ImportAlias::Alias(name)) => write!(self, " as {name}")?,
            Some(ImportAlias::Underscore) => write!(self, " as _")?,
            None => {}
        }
        writeln!(self, ";")
    }

    /// Prints a type alias to the buffer.
    fn print_type_alias(&mut self, it: LocalItemTreeId<TypeAlias>) -> fmt::Result {
        let TypeAlias {
            name,
            visibility,
            types,
            type_ref,
            ast_id: _,
        } = &self.tree[it];
        self.print_visibility(*visibility)?;
        write!(self, "type {name}")?;
        if let Some(ty) = type_ref {
            write!(self, " = ")?;
            self.print_type_ref(*ty, types)?;
        }
        writeln!(self, ";")
    }

    /// Prints a struct to the buffer.
    fn print_struct(&mut self, it: LocalItemTreeId<Struct>) -> fmt::Result {
        let Struct {
            visibility,
            name,
            types,
            fields,
            ast_id: _,
        } = &self.tree[it];
        self.print_visibility(*visibility)?;
        write!(self, "struct {name}")?;
        match fields {
            Fields::Record(fields) => {
                self.whitespace()?;
                write!(self, "{{")?;
                self.indented(|this| {
                    for field in fields.clone() {
                        let field = &this.tree[field];
                        write!(this, "{}: ", field.name)?;
                        this.print_type_ref(field.type_ref, types)?;
                        writeln!(this, ",")?;
                    }
                    Ok(())
                })?;
                write!(self, "}}")?;
            }
            Fields::Tuple(fields) => {
                write!(self, "(")?;
                self.indented(|this| {
                    for field in fields.clone() {
                        let field = &this.tree[field];
                        this.print_type_ref(field.type_ref, types)?;
                        writeln!(this, ",")?;
                    }
                    Ok(())
                })?;
                write!(self, ")")?;
            }
            Fields::Unit => {}
        };
        if matches!(fields, Fields::Record(_)) {
            writeln!(self)
        } else {
            writeln!(self, ";")
        }
    }

    /// Prints a function to the buffer.
    fn print_function(&mut self, it: LocalItemTreeId<Function>) -> fmt::Result {
        let Function {
            name,
            visibility,
            types,
            params,
            ret_type,
            ast_id: _,
            flags,
        } = &self.tree[it];
        self.print_visibility(*visibility)?;
        if flags.is_extern() {
            write!(self, "extern ")?;
        }
        write!(self, "fn {name}")?;
        write!(self, "(")?;
        if !params.is_empty() {
            self.indented(|this| {
                let mut params = params.clone();
                if flags.has_self_param() {
                    // Skip self parameter
                    params.next();

                    write!(this, "self")?;
                }

                for param in params {
                    let Param {
                        type_ref,
                        ast_id: _,
                    } = &this.tree[param];
                    this.print_type_ref(*type_ref, types)?;
                    writeln!(this, ",")?;
                }
                Ok(())
            })?;
        }
        write!(self, ") -> ")?;
        self.print_type_ref(*ret_type, types)?;
        writeln!(self, ";")
    }

    /// Prints a [`RawVisibilityId`] to the buffer.
    fn print_visibility(&mut self, vis: RawVisibilityId) -> fmt::Result {
        match &self.tree[vis] {
            RawVisibility::This => Ok(()),
            RawVisibility::Super => write!(self, "pub(super) "),
            RawVisibility::Package => write!(self, "pub(package) "),
            RawVisibility::Public => write!(self, "pub "),
        }
    }

    /// Prints a type reference to the buffer.
    fn print_type_ref(&mut self, type_ref: LocalTypeRefId, map: &TypeRefMap) -> fmt::Result {
        print_type_ref(self.db, map, type_ref, self)
    }

    /// Prints an `impl` block to the buffer.
    fn print_impl(&mut self, it: LocalItemTreeId<Impl>) -> fmt::Result {
        let Impl {
            types,
            self_ty,
            items,
            ast_id: _,
        } = &self.tree[it];
        write!(self, "impl ")?;
        self.print_type_ref(*self_ty, types)?;
        self.whitespace()?;
        write!(self, "{{")?;
        self.indented(|this| {
            for item in items.iter().copied() {
                this.print_mod_item(item.into())?;
            }
            Ok(())
        })?;
        write!(self, "}}")
    }
}

impl Write for Printer<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for line in s.split_inclusive('\n') {
            if self.needs_indent {
                match self.buf.chars().last() {
                    Some('\n') | None => {}
                    _ => self.buf.push('\n'),
                }
                self.buf.push_str(&"  ".repeat(self.indent_level));
                self.needs_indent = false;
            }

            self.buf.push_str(line);
            self.needs_indent = line.ends_with('\n');
        }

        Ok(())
    }
}
