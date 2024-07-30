//! This crate provides utilitiy functions to be used alongside salsa databases
//! used by mun.

pub trait Upcast<T: ?Sized> {
    fn upcast(&self) -> &T;
}
