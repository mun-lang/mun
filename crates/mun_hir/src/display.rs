use std::fmt;

use crate::db::HirDatabase;

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
