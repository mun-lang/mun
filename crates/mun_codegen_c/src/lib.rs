mod code_gen;
mod db;
mod dispatch_table;
mod function;
mod identifier;
mod ty;
mod type_table;

pub use self::db::{CCodegenDatabase, CCodegenDatabaseStorage};

#[derive(Debug, PartialEq, Eq)]
pub struct HeaderAndSourceFiles {
    pub header: String,
    pub source: String,
}
