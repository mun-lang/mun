mod myers;
mod primitives;
mod structs;
mod util;

use crate::fake_struct;
use mun_memory::{
    diff::{diff, Diff},
    type_table::TypeTable,
};
use util::*;

#[test]
fn add() {
    let type_table = TypeTable::default();

    let int = type_table.find_type_info_by_name("core::i64").unwrap();
    let float = type_table.find_type_info_by_name("core::f64").unwrap();
    let struct1 = fake_struct!(type_table, "struct1", "a" => i64, "b" => f64);

    let old = &[int.clone(), struct1.clone()];
    let new = &[int.clone(), struct1.clone(), float.clone()];

    let diff = diff(old, new);
    assert_eq!(diff, vec![Diff::Insert { index: 2 }]);
    assert_eq!(apply_diff(old, new, diff), vec![int, struct1, float]);
}
