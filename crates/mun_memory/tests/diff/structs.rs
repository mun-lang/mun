use std::sync::Arc;

use crate::{diff::util::*, fake_struct};
use mun_memory::{
    diff::{diff, Diff, FieldDiff, FieldEditKind},
    type_table::TypeTable,
    TypeInfo,
};

// TODO: Once we can generate `Guid`s based on the data layout, we can just directly check
// `TypeInfo`s against each other.
fn assert_eq_struct(result: &[Arc<TypeInfo>], expected: &[Arc<TypeInfo>]) {
    assert_eq!(result.len(), expected.len());
    for (lhs, rhs) in result.into_iter().zip(expected.into_iter()) {
        assert_eq!(lhs.layout, rhs.layout);
        assert_eq!(lhs.data.is_struct(), rhs.data.is_struct());

        let lhs = lhs.as_struct().unwrap();
        let rhs = rhs.as_struct().unwrap();

        assert_eq!(lhs.field_names, rhs.field_names);
        assert_eq!(lhs.field_types, rhs.field_types);
        assert_eq!(lhs.field_offsets, rhs.field_offsets);
        assert_eq!(lhs.memory_kind, rhs.memory_kind);
    }
}

#[test]
fn add() {
    let type_table = TypeTable::default();

    let int = type_table.find_type_info_by_name("core::i64").unwrap();
    let float = type_table.find_type_info_by_name("core::f64").unwrap();

    let struct1 = fake_struct!(type_table, STRUCT1_NAME,
        "a" => i64, "b" => f64
    );
    let struct2 = fake_struct!(type_table, STRUCT2_NAME,
        "c" => f64, "d" => i64
    );

    let old = &[struct1.clone()];
    let new = &[struct1.clone(), struct2.clone()];

    let diff = diff(old, new);
    assert_eq!(diff, vec![Diff::Insert { index: 1 }]);
    assert_eq_struct(
        &apply_diff(old, new, diff),
        &vec![struct1.clone(), struct2.clone()],
    );
}

#[test]
fn remove() {
    let type_table = TypeTable::default();

    let int = type_table.find_type_info_by_name("core::i64").unwrap();
    let float = type_table.find_type_info_by_name("core::f64").unwrap();

    let struct1 = fake_struct!(type_table, STRUCT1_NAME,
        "a" => i64, "b" => f64
    );
    let struct2 = fake_struct!(type_table, STRUCT2_NAME,
        "c" => f64, "d" => i64
    );

    let old = &[struct1.clone(), struct2.clone()];
    let new = &[struct1.clone()];

    let diff = diff(old, new);
    assert_eq!(diff, vec![Diff::Delete { index: 1 },]);
    assert_eq_struct(&apply_diff(old, new, diff), &vec![struct1.clone()]);
}

#[test]
fn replace() {
    let type_table = TypeTable::default();

    let int = type_table.find_type_info_by_name("core::i64").unwrap();
    let float = type_table.find_type_info_by_name("core::f64").unwrap();

    let struct1 = fake_struct!(type_table, STRUCT1_NAME,
        "a" => i64, "b" => f64
    );
    let struct2 = fake_struct!(type_table, STRUCT2_NAME,
        "c" => f64, "d" => i64
    );

    let old = &[struct1.clone()];
    let new = &[struct2.clone()];

    let diff = diff(old, new);
    assert_eq!(
        diff,
        vec![Diff::Delete { index: 0 }, Diff::Insert { index: 0 }]
    );
    assert_eq_struct(&apply_diff(old, new, diff), &vec![struct2]);
}

#[test]
fn swap() {
    let type_table = TypeTable::default();

    let int = type_table.find_type_info_by_name("core::i64").unwrap();
    let float = type_table.find_type_info_by_name("core::f64").unwrap();

    let struct1 = fake_struct!(type_table, STRUCT1_NAME,
        "a" => i64, "b" => f64
    );
    let struct2 = fake_struct!(type_table, STRUCT2_NAME,
        "c" => f64, "d" => i64
    );

    let old = &[struct1.clone(), struct2.clone()];
    let new = &[struct2.clone(), struct1.clone()];

    let diff = diff(old, new);
    assert_eq!(
        diff,
        vec![Diff::Move {
            old_index: 0,
            new_index: 1
        }]
    );
    assert_eq_struct(
        &apply_diff(old, new, diff),
        &vec![struct2.clone(), struct1.clone()],
    );
}

#[test]
fn add_field1() {
    let type_table = TypeTable::default();

    let struct1 = fake_struct!(type_table, STRUCT1_NAME,
        "b" => i64, "c" => f64
    );
    let struct2 = fake_struct!(type_table, STRUCT2_NAME,
        "a" => i64, "b" => i64, "c" => f64
    );

    let old = &[struct1.clone()];
    let new = &[struct2.clone()];

    let diff = diff(old, new);
    assert_eq!(
        diff,
        vec![Diff::Edit {
            diff: vec![FieldDiff::Insert { index: 0 }],
            old_index: 0,
            new_index: 0,
        }]
    );
    assert_eq_struct(&apply_diff(old, new, diff), &vec![struct2.clone()]);
}

#[test]
fn add_field2() {
    let type_table = TypeTable::default();

    let struct1 = fake_struct!(type_table, STRUCT1_NAME,
        "a" => i64, "c" => f64
    );
    let struct2 = fake_struct!(type_table, STRUCT2_NAME,
        "a" => i64, "b" => f64, "c" => f64
    );

    let old = &[struct1.clone()];
    let new = &[struct2.clone()];

    let diff = diff(old, new);
    assert_eq!(
        diff,
        vec![Diff::Edit {
            diff: vec![FieldDiff::Insert { index: 1 }],
            old_index: 0,
            new_index: 0,
        }]
    );
    assert_eq_struct(&apply_diff(old, new, diff), &vec![struct2.clone()]);
}

#[test]
fn add_field3() {
    let type_table = TypeTable::default();

    let struct1 = fake_struct!(type_table, STRUCT1_NAME,
        "a" => i64, "b" => f64
    );
    let struct2 = fake_struct!(type_table, STRUCT2_NAME,
        "a" => i64, "b" => f64, "c" => f64
    );

    let old = &[struct1.clone()];
    let new = &[struct2.clone()];

    let diff = diff(old, new);
    assert_eq!(
        diff,
        vec![Diff::Edit {
            diff: vec![FieldDiff::Insert { index: 2 }],
            old_index: 0,
            new_index: 0,
        }]
    );
    assert_eq_struct(&apply_diff(old, new, diff), &vec![struct2.clone()]);
}

#[test]
fn remove_field1() {
    let type_table = TypeTable::default();

    let struct1 = fake_struct!(type_table, STRUCT1_NAME,
        "a" => f64, "b" => f64, "c" => i64
    );
    let struct2 = fake_struct!(type_table, STRUCT2_NAME, "c" => i64);

    let old = &[struct1.clone()];
    let new = &[struct2.clone()];

    let diff = diff(old, new);
    assert_eq!(
        diff,
        vec![Diff::Edit {
            diff: vec![
                FieldDiff::Delete { index: 0 },
                FieldDiff::Delete { index: 1 }
            ],
            old_index: 0,
            new_index: 0,
        }]
    );
    assert_eq_struct(&apply_diff(old, new, diff), &vec![struct2.clone()]);
}

#[test]
fn remove_field2() {
    let type_table = TypeTable::default();

    let struct1 = fake_struct!(type_table, STRUCT1_NAME,
        "a" => f64, "b" => i64, "c" => f64
    );
    let struct2 = fake_struct!(type_table, STRUCT1_NAME, "b" => i64);

    let old = &[struct1.clone()];
    let new = &[struct2.clone()];

    let diff = diff(old, new);
    assert_eq!(
        diff,
        vec![Diff::Edit {
            diff: vec![
                FieldDiff::Delete { index: 0 },
                FieldDiff::Delete { index: 2 }
            ],
            old_index: 0,
            new_index: 0,
        }]
    );
    assert_eq_struct(&apply_diff(old, new, diff), &vec![struct2.clone()]);
}

#[test]
fn remove_field3() {
    let type_table = TypeTable::default();

    let struct1 = fake_struct!(type_table, STRUCT1_NAME,
        "a" => i64, "b" => f64, "c" => f64
    );
    let struct2 = fake_struct!(type_table, STRUCT2_NAME, "a" => i64);

    let old = &[struct1.clone()];
    let new = &[struct2.clone()];

    let diff = diff(old, new);
    assert_eq!(
        diff,
        vec![Diff::Edit {
            diff: vec![
                FieldDiff::Delete { index: 1 },
                FieldDiff::Delete { index: 2 }
            ],
            old_index: 0,
            new_index: 0,
        }]
    );
    assert_eq_struct(&apply_diff(old, new, diff), &vec![struct2.clone()]);
}

#[test]
fn swap_fields() {
    let type_table = TypeTable::default();

    let struct1 = fake_struct!(type_table, STRUCT1_NAME,
        "a" => f64, "b" => i64, "c" => f64
    );
    let struct2 = fake_struct!(type_table, STRUCT2_NAME,
        "c" => f64, "a" => f64, "b" => i64
    );

    let old = &[struct1.clone()];
    let new = &[struct2.clone()];

    let diff = diff(old, new);
    assert_eq!(
        diff,
        vec![Diff::Edit {
            diff: vec![FieldDiff::Move {
                old_index: 2,
                new_index: 0,
                edit: None,
            },],
            old_index: 0,
            new_index: 0,
        }]
    );
    assert_eq_struct(&apply_diff(old, new, diff), &vec![struct2.clone()]);
}

#[test]
fn swap_fields2() {
    let type_table = TypeTable::default();

    let struct1 = fake_struct!(type_table, STRUCT1_NAME,
        "a" => f64, "b" => i64, "c" => f64, "d" => i64
    );
    let struct2 = fake_struct!(type_table, STRUCT2_NAME,
        "d" => i64, "c" => f64, "b" => i64, "a" => f64
    );

    let old = &[struct1.clone()];
    let new = &[struct2.clone()];

    let diff = diff(old, new);
    assert_eq!(
        diff,
        vec![Diff::Edit {
            diff: vec![
                FieldDiff::Move {
                    old_index: 0,
                    new_index: 3,
                    edit: None,
                },
                FieldDiff::Move {
                    old_index: 1,
                    new_index: 2,
                    edit: None,
                },
                FieldDiff::Move {
                    old_index: 2,
                    new_index: 1,
                    edit: None,
                }
            ],
            old_index: 0,
            new_index: 0
        }]
    );
    assert_eq_struct(&apply_diff(old, new, diff), &vec![struct2.clone()]);
}

#[test]
fn cast_field() {
    let type_table = TypeTable::default();

    let struct1 = fake_struct!(type_table, STRUCT1_NAME,
        "a" => i64, "b" => f64, "c" => f64
    );
    let struct2 = fake_struct!(type_table, STRUCT2_NAME,
        "a" => f64, "b" => i64, "c" => i64
    );

    let old = &[struct1.clone()];
    let new = &[struct2.clone()];

    let diff = diff(old, new);
    assert_eq!(
        diff,
        vec![Diff::Edit {
            diff: vec![
                FieldDiff::Edit {
                    index: 0,
                    kind: FieldEditKind::ConvertType,
                },
                FieldDiff::Edit {
                    index: 1,
                    kind: FieldEditKind::ConvertType,
                },
                FieldDiff::Edit {
                    index: 2,
                    kind: FieldEditKind::ConvertType,
                }
            ],
            old_index: 0,
            new_index: 0,
        }]
    );
    assert_eq_struct(&apply_diff(old, new, diff), &vec![struct2.clone()]);
}

#[test]
fn rename_field1() {
    let type_table = TypeTable::default();

    let struct1 = fake_struct!(type_table, STRUCT1_NAME,
        "a" => i64, "b" => f64, "c" => f64
    );
    let struct2 = fake_struct!(type_table, STRUCT2_NAME,
        "a" => i64, "d" => f64, "c" => f64
    );

    let old = &[struct1.clone()];
    let new = &[struct2.clone()];

    let diff = diff(old, new);
    assert_eq!(
        diff,
        vec![Diff::Edit {
            diff: vec![FieldDiff::Edit {
                index: 1,
                kind: FieldEditKind::Rename,
            }],
            old_index: 0,
            new_index: 0,
        }]
    );
    assert_eq_struct(&apply_diff(old, new, diff), &vec![struct2.clone()]);
}

#[test]
fn rename_field2() {
    let type_table = TypeTable::default();

    let struct1 = fake_struct!(type_table, STRUCT1_NAME,
        "a" => i64, "b" => f64, "c" => f64
    );
    let struct2 = fake_struct!(type_table, STRUCT2_NAME,
        "d" => i64, "e" => f64, "f" => f64
    );

    let old = &[struct1.clone()];
    let new = &[struct2.clone()];

    let diff = diff(old, new);
    assert_eq!(
        diff,
        vec![Diff::Edit {
            diff: vec![
                FieldDiff::Edit {
                    index: 0,
                    kind: FieldEditKind::Rename,
                },
                FieldDiff::Edit {
                    index: 1,
                    kind: FieldEditKind::Rename,
                },
                FieldDiff::Edit {
                    index: 2,
                    kind: FieldEditKind::Rename,
                }
            ],
            old_index: 0,
            new_index: 0,
        }]
    );
    assert_eq_struct(&apply_diff(old, new, diff), &vec![struct2.clone()]);
}
