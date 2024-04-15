use std::fmt;

use crate::{
    code_model::AssocItem,
    db::HirDatabase,
    type_ref::{LocalTypeRefId, TypeRef, TypeRefMap},
    Function, HasVisibility, ModuleId, Visibility,
};

pub struct HirFormatter<'a, 'b> {
    pub db: &'a dyn HirDatabase,
    fmt: &'a mut fmt::Formatter<'b>,
}

pub trait HirDisplay {
    fn hir_fmt(&self, f: &mut HirFormatter<'_, '_>) -> fmt::Result;
    fn display<'a>(&'a self, db: &'a dyn HirDatabase) -> HirDisplayWrapper<'a, Self>
    where
        Self: Sized,
    {
        HirDisplayWrapper(db, self)
    }
}

impl<'a, 'b> HirFormatter<'a, 'b> {
    pub fn write_joined<T: HirDisplay>(
        &mut self,
        iter: impl IntoIterator<Item = T>,
        sep: &str,
    ) -> fmt::Result {
        let mut first = true;
        for e in iter {
            if !first {
                write!(self, "{sep}")?;
            }
            first = false;
            e.hir_fmt(self)?;
        }
        Ok(())
    }

    /// This allows using the `write!` macro directly with a `HirFormatter`.
    pub fn write_fmt(&mut self, args: fmt::Arguments<'_>) -> fmt::Result {
        fmt::write(self.fmt, args)
    }
}

pub struct HirDisplayWrapper<'a, T>(&'a dyn HirDatabase, &'a T);

impl<'a, T> fmt::Display for HirDisplayWrapper<'a, T>
where
    T: HirDisplay,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.1.hir_fmt(&mut HirFormatter { db: self.0, fmt: f })
    }
}

impl HirDisplay for AssocItem {
    fn hir_fmt(&self, f: &mut HirFormatter<'_, '_>) -> fmt::Result {
        match self {
            AssocItem::Function(fun) => fun.hir_fmt(f),
        }
    }
}

impl HirDisplay for Function {
    fn hir_fmt(&self, f: &mut HirFormatter<'_, '_>) -> fmt::Result {
        let db = f.db;
        let data = db.fn_data(self.id);
        let module = self.module(db);
        write_visiblity(module.id, self.visibility(db), f)?;
        if data.is_extern() {
            write!(f, "extern ")?;
        }
        write!(f, "fn {}(", self.name(db))?;

        let type_map = data.type_ref_map();
        for (idx, (&type_ref_id, param)) in data.params().iter().zip(self.params(db)).enumerate() {
            let name = param.name(db);
            if idx != 0 {
                write!(f, ", ")?;
            }
            match name {
                Some(name) => write!(f, "{name}: ")?,
                None => write!(f, "_: ")?,
            }
            write_type_ref(type_ref_id, type_map, f)?;
        }

        write!(f, ")")?;

        let ret_type_id = *data.ret_type();
        match &type_map[ret_type_id] {
            TypeRef::Tuple(elems) if elems.is_empty() => {}
            _ => {
                write!(f, " -> ")?;
                write_type_ref(ret_type_id, type_map, f)?;
            }
        }

        Ok(())
    }
}

fn write_type_ref(
    type_ref_id: LocalTypeRefId,
    container: &TypeRefMap,
    f: &mut HirFormatter<'_, '_>,
) -> fmt::Result {
    let type_ref = &container[type_ref_id];
    match type_ref {
        TypeRef::Path(path) => write!(f, "{path}"),
        TypeRef::Array(element_ty) => {
            write!(f, "[")?;
            write_type_ref(*element_ty, container, f)?;
            write!(f, "]")
        }
        TypeRef::Never => write!(f, "!"),
        TypeRef::Tuple(elems) => {
            write!(f, "(")?;
            for (idx, elem) in elems.iter().enumerate() {
                if idx != 0 {
                    write!(f, ", ")?;
                }
                write_type_ref(*elem, container, f)?;
            }
            write!(f, ")")
        }
        TypeRef::Error => write!(f, "{{error}}"),
    }
}

/// Writes the visibility of an item to the formatter.
fn write_visiblity(
    module_id: ModuleId,
    vis: Visibility,
    f: &mut HirFormatter<'_, '_>,
) -> fmt::Result {
    match vis {
        Visibility::Public => write!(f, "pub "),
        Visibility::Module(vis_id) => {
            let module_tree = f.db.module_tree(module_id.package);
            if module_id == vis_id {
                // Only visible to self
                Ok(())
            } else if vis_id.local_id == module_tree.root {
                write!(f, "pub(package) ")
            } else if module_tree[module_id.local_id].parent == Some(vis_id.local_id) {
                write!(f, "pub(super) ")
            } else {
                write!(f, "pub(in ...) ")
            }
        }
    }
}
