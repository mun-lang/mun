use crate::item_tree::{
    Fields, Function, Import, ItemTree, ModItem, RawVisibilityId, Struct, TypeAlias,
};
use crate::path::ImportAlias;
use crate::pretty::{print_path, print_type_ref};
use crate::type_ref::{LocalTypeRefId, TypeRefMap};
use crate::visibility::RawVisibility;
use crate::DefDatabase;
use std::fmt;
use std::fmt::Write;

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

struct Printer<'a> {
    db: &'a dyn DefDatabase,
    tree: &'a ItemTree,
    buf: String,
    indent_level: usize,
    needs_indent: bool,
}

impl Printer<'_> {
    fn indented(&mut self, f: impl FnOnce(&mut Self) -> fmt::Result) -> fmt::Result {
        self.indent_level += 1;
        writeln!(self)?;
        f(self)?;
        self.indent_level -= 1;
        self.buf = self.buf.trim_end_matches('\n').to_string();
        Ok(())
    }

    fn whitespace(&mut self) -> fmt::Result {
        match self.buf.chars().next_back() {
            None | Some('\n' | ' ') => {}
            _ => self.buf.push(' '),
        }
        Ok(())
    }

    fn print_mod_item(&mut self, item: ModItem) -> fmt::Result {
        match item {
            ModItem::Function(it) => {
                let Function {
                    name,
                    visibility,
                    is_extern,
                    types,
                    params,
                    ret_type,
                    ast_id: _,
                } = &self.tree[it];
                self.print_visibility(*visibility)?;
                if *is_extern {
                    write!(self, "extern ")?;
                }
                write!(self, "fn {}", name)?;
                write!(self, "(")?;
                if !params.is_empty() {
                    self.indented(|this| {
                        for param in params.iter().copied() {
                            this.print_type_ref(param, types)?;
                            writeln!(this, ",")?;
                        }
                        Ok(())
                    })?;
                }
                write!(self, ") -> ")?;
                self.print_type_ref(*ret_type, types)?;
                writeln!(self, ";")
            }
            ModItem::Struct(it) => {
                let Struct {
                    visibility,
                    name,
                    types,
                    fields,
                    ast_id: _,
                } = &self.tree[it];
                self.print_visibility(*visibility)?;
                write!(self, "struct {}", name)?;
                match fields {
                    Fields::Record(fields) => {
                        write!(self, " {{")?;
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
                        write!(self, " (")?;
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
            ModItem::TypeAlias(it) => {
                let TypeAlias {
                    name,
                    visibility,
                    types,
                    type_ref,
                    ast_id: _,
                } = &self.tree[it];
                self.print_visibility(*visibility)?;
                write!(self, "type {}", name)?;
                if let Some(ty) = type_ref {
                    write!(self, " = ")?;
                    self.print_type_ref(*ty, types)?;
                }
                writeln!(self, ";")
            }
            ModItem::Import(it) => {
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
                    Some(ImportAlias::Alias(name)) => write!(self, " as {}", name)?,
                    Some(ImportAlias::Underscore) => write!(self, " as _")?,
                    None => {}
                }
                writeln!(self, ";")
            }
        }
    }

    fn print_visibility(&mut self, vis: RawVisibilityId) -> fmt::Result {
        match &self.tree[vis] {
            RawVisibility::This => Ok(()),
            RawVisibility::Super => write!(self, "pub(super) "),
            RawVisibility::Package => write!(self, "pub(package) "),
            RawVisibility::Public => write!(self, "pub "),
        }
    }

    fn print_type_ref(&mut self, type_ref: LocalTypeRefId, map: &TypeRefMap) -> fmt::Result {
        print_type_ref(self.db, map, type_ref, self)
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
