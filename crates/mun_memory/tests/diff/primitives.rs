use super::util::*;
use mun_memory::{
    diff::{diff, Diff},
    type_table::TypeTable,
};

#[test]
fn add() {
    let type_table = TypeTable::default();

    let int = type_table.find_type_info_by_name("core::i64").unwrap();
    let float = type_table.find_type_info_by_name("core::f64").unwrap();

    let old = &[int.clone()];
    let new = &[int.clone(), float.clone()];

    let diff = diff(old, new);
    assert_eq!(diff, vec![Diff::Insert { index: 1 }]);
    assert_eq!(apply_diff(old, new, diff), vec![int, float]);
}

#[test]
fn remove() {
    let type_table = TypeTable::default();

    let int = type_table.find_type_info_by_name("core::i64").unwrap();
    let float = type_table.find_type_info_by_name("core::f64").unwrap();

    let old = &[int.clone(), float.clone()];
    let new = &[float.clone()];

    let diff = diff(old, new);
    assert_eq!(diff, vec![Diff::Delete { index: 0 },]);
    assert_eq!(apply_diff(old, new, diff), vec![float]);
}

#[test]
fn replace() {
    let type_table = TypeTable::default();

    let int = type_table.find_type_info_by_name("core::i64").unwrap();
    let float = type_table.find_type_info_by_name("core::f64").unwrap();

    let old = &[int.clone()];
    let new = &[float.clone()];

    let diff = diff(old, new);
    assert_eq!(
        diff,
        vec![Diff::Delete { index: 0 }, Diff::Insert { index: 0 }]
    );
    assert_eq!(apply_diff(old, new, diff), vec![float]);
}

#[test]
fn swap() {
    let type_table = TypeTable::default();

    let int = type_table.find_type_info_by_name("core::i64").unwrap();
    let float = type_table.find_type_info_by_name("core::f64").unwrap();

    let old = &[int.clone(), float.clone()];
    let new = &[float.clone(), int.clone()];

    let diff = diff(old, new);
    assert_eq!(
        diff,
        vec![Diff::Move {
            old_index: 0,
            new_index: 1
        },]
    );
    assert_eq!(apply_diff(old, new, diff), vec![float, int]);
}
