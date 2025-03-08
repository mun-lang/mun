mod code_gen;
pub mod db;
mod dispatch_table;
mod function;
mod ty;

#[derive(Debug, PartialEq, Eq)]
pub struct HeaderAndSourceFiles {
    pub header: String,
    pub source: String,
}
