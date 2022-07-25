use std::sync::Arc;

use crate::{diff::util::*, fake_struct};
use mun_memory::{
    diff::{diff, Diff, FieldDiff, FieldEditKind},
    type_table::TypeTable,
    HasStaticTypeInfo, TypeInfo,
};

fn assert_eq_struct(result: &[Arc<TypeInfo>], expected: &[Arc<TypeInfo>]) {
    assert_eq!(result.len(), expected.len());
    for (lhs, rhs) in result.iter().zip(expected.iter()) {
        assert_eq!(lhs, rhs);
    }
}

#[test]
fn add() {
    let type_table = TypeTable::default();

    let struct1 = fake_struct!(type_table, "struct1",
        "a" => i64, "b" => f64
    );
    let struct2 = fake_struct!(type_table, "struct2",
        "c" => f64, "d" => i64
    );

    let old = &[struct1.clone()];
    let new = &[struct1.clone(), struct2.clone()];

    let diff = diff(old, new);
    assert_eq!(diff, vec![Diff::Insert { index: 1 }]);
    assert_eq_struct(&apply_diff(old, new, diff), &[struct1, struct2]);
}

#[test]
fn remove() {
    let type_table = TypeTable::default();

    let struct1 = fake_struct!(type_table, "struct1",
        "a" => i64, "b" => f64
    );
    let struct2 = fake_struct!(type_table, "struct2",
        "c" => f64, "d" => i64
    );

    let old = &[struct1.clone(), struct2];
    let new = &[struct1.clone()];

    let diff = diff(old, new);
    assert_eq!(diff, vec![Diff::Delete { index: 1 },]);
    assert_eq_struct(&apply_diff(old, new, diff), &[struct1]);
}

#[test]
fn replace() {
    let type_table = TypeTable::default();

    let struct1 = fake_struct!(type_table, "struct1",
        "a" => i64, "b" => f64
    );
    let struct2 = fake_struct!(type_table, "struct2",
        "c" => f64, "d" => i64
    );

    let old = &[struct1];
    let new = &[struct2.clone()];

    let diff = diff(old, new);
    assert_eq!(
        diff,
        vec![Diff::Delete { index: 0 }, Diff::Insert { index: 0 }]
    );
    assert_eq_struct(&apply_diff(old, new, diff), &[struct2]);
}

#[test]
fn swap() {
    let type_table = TypeTable::default();

    let struct1 = fake_struct!(type_table, "struct1",
        "a" => i64, "b" => f64
    );
    let struct2 = fake_struct!(type_table, "struct2",
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
    assert_eq_struct(&apply_diff(old, new, diff), &[struct2, struct1]);
}

#[test]
fn add_field1() {
    let type_table = TypeTable::default();

    let struct1 = fake_struct!(type_table, "struct1",
        "b" => i64, "c" => f64
    );
    let struct2 = fake_struct!(type_table, "struct1",
        "a" => i64, "b" => i64, "c" => f64
    );

    let old = &[struct1];
    let new = &[struct2.clone()];

    let diff = diff(old, new);
    assert_eq!(
        diff,
        vec![Diff::Edit {
            diff: vec![FieldDiff::Insert {
                index: 0,
                new_type: i64::type_info().clone(),
            }],
            old_index: 0,
            new_index: 0,
        }]
    );
    assert_eq_struct(&apply_diff(old, new, diff), &[struct2]);
}

#[test]
fn add_field2() {
    let type_table = TypeTable::default();

    let struct1 = fake_struct!(type_table, "struct1",
        "a" => i64, "c" => f64
    );
    let struct2 = fake_struct!(type_table, "struct1",
        "a" => i64, "b" => f64, "c" => f64
    );

    let old = &[struct1];
    let new = &[struct2.clone()];

    let diff = diff(old, new);
    assert_eq!(
        diff,
        vec![Diff::Edit {
            diff: vec![FieldDiff::Insert {
                index: 1,
                new_type: f64::type_info().clone()
            }],
            old_index: 0,
            new_index: 0,
        }]
    );
    assert_eq_struct(&apply_diff(old, new, diff), &[struct2]);
}

#[test]
fn add_field3() {
    let type_table = TypeTable::default();

    let struct1 = fake_struct!(type_table, "struct1",
        "a" => i64, "b" => f64
    );
    let struct2 = fake_struct!(type_table, "struct1",
        "a" => i64, "b" => f64, "c" => f64
    );

    let old = &[struct1];
    let new = &[struct2.clone()];

    let diff = diff(old, new);
    assert_eq!(
        diff,
        vec![Diff::Edit {
            diff: vec![FieldDiff::Insert {
                index: 2,
                new_type: f64::type_info().clone()
            }],
            old_index: 0,
            new_index: 0,
        }]
    );
    assert_eq_struct(&apply_diff(old, new, diff), &[struct2]);
}

#[test]
fn remove_field1() {
    let type_table = TypeTable::default();

    let struct1 = fake_struct!(type_table, "struct1",
        "a" => f64, "b" => f64, "c" => i64
    );
    let struct2 = fake_struct!(type_table, "struct1", "c" => i64);

    let old = &[struct1];
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
    assert_eq_struct(&apply_diff(old, new, diff), &[struct2]);
}

#[test]
fn remove_field2() {
    let type_table = TypeTable::default();

    let struct1 = fake_struct!(type_table, "struct1",
        "a" => f64, "b" => i64, "c" => f64
    );
    let struct2 = fake_struct!(type_table, "struct1", "b" => i64);

    let old = &[struct1];
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
    assert_eq_struct(&apply_diff(old, new, diff), &[struct2]);
}

#[test]
fn remove_field3() {
    let type_table = TypeTable::default();

    let struct1 = fake_struct!(type_table, "struct1",
        "a" => i64, "b" => f64, "c" => f64
    );
    let struct2 = fake_struct!(type_table, "struct1", "a" => i64);

    let old = &[struct1];
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
    assert_eq_struct(&apply_diff(old, new, diff), &[struct2]);
}

#[test]
fn swap_fields() {
    let type_table = TypeTable::default();

    let struct1 = fake_struct!(type_table, "struct1",
        "a" => f64, "b" => i64, "c" => f64
    );
    let struct2 = fake_struct!(type_table, "struct1",
        "c" => f64, "a" => f64, "b" => i64
    );

    let old = &[struct1];
    let new = &[struct2.clone()];

    let diff = diff(old, new);
    assert_eq!(
        diff,
        vec![Diff::Edit {
            diff: vec![FieldDiff::Move {
                ty: f64::type_info().clone(),
                old_index: 2,
                new_index: 0,
            },],
            old_index: 0,
            new_index: 0,
        }]
    );
    assert_eq_struct(&apply_diff(old, new, diff), &[struct2]);
}

#[test]
fn swap_fields2() {
    let type_table = TypeTable::default();

    let struct1 = fake_struct!(type_table, "struct1",
        "a" => f64, "b" => i64, "c" => f64, "d" => i64
    );
    let struct2 = fake_struct!(type_table, "struct1",
        "d" => i64, "c" => f64, "b" => i64, "a" => f64
    );

    let old = &[struct1];
    let new = &[struct2.clone()];

    let diff = diff(old, new);
    assert_eq!(
        diff,
        vec![Diff::Edit {
            diff: vec![
                FieldDiff::Move {
                    ty: f64::type_info().clone(),
                    old_index: 0,
                    new_index: 3,
                },
                FieldDiff::Move {
                    ty: i64::type_info().clone(),
                    old_index: 1,
                    new_index: 2,
                },
                FieldDiff::Move {
                    ty: f64::type_info().clone(),
                    old_index: 2,
                    new_index: 1,
                }
            ],
            old_index: 0,
            new_index: 0
        }]
    );
    assert_eq_struct(&apply_diff(old, new, diff), &[struct2]);
}

#[test]
fn cast_field() {
    let type_table = TypeTable::default();

    let struct1 = fake_struct!(type_table, "struct1",
        "a" => i64, "b" => f64, "c" => f64
    );
    let struct2 = fake_struct!(type_table, "struct1",
        "a" => f64, "b" => i64, "c" => i64
    );

    let old = &[struct1];
    let new = &[struct2.clone()];

    let diff = diff(old, new);
    assert_eq!(
        diff,
        vec![Diff::Edit {
            diff: vec![
                FieldDiff::Edit {
                    old_type: i64::type_info().clone(),
                    new_type: f64::type_info().clone(),
                    old_index: None,
                    new_index: 0,
                    kind: FieldEditKind::ChangedTyped,
                },
                FieldDiff::Edit {
                    old_type: f64::type_info().clone(),
                    new_type: i64::type_info().clone(),
                    old_index: None,
                    new_index: 1,
                    kind: FieldEditKind::ChangedTyped,
                },
                FieldDiff::Edit {
                    old_type: f64::type_info().clone(),
                    new_type: i64::type_info().clone(),
                    old_index: None,
                    new_index: 2,
                    kind: FieldEditKind::ChangedTyped,
                }
            ],
            old_index: 0,
            new_index: 0,
        }]
    );
    assert_eq_struct(&apply_diff(old, new, diff), &[struct2]);
}

#[test]
fn rename_field1() {
    let type_table = TypeTable::default();

    let struct1 = fake_struct!(type_table, "struct1",
        "a" => i64, "b" => f64, "c" => f64
    );
    let struct2 = fake_struct!(type_table, "struct1",
        "a" => i64, "d" => f64, "c" => f64
    );

    let old = &[struct1];
    let new = &[struct2.clone()];

    let diff = diff(old, new);
    assert_eq!(
        diff,
        vec![Diff::Edit {
            diff: vec![FieldDiff::Edit {
                old_type: f64::type_info().clone(),
                new_type: f64::type_info().clone(),
                old_index: None,
                new_index: 1,
                kind: FieldEditKind::RenamedField,
            }],
            old_index: 0,
            new_index: 0,
        }]
    );
    assert_eq_struct(&apply_diff(old, new, diff), &[struct2]);
}

#[test]
fn rename_field2() {
    let type_table = TypeTable::default();

    let struct1 = fake_struct!(type_table, "struct1",
        "a" => i64, "b" => f64, "c" => f64
    );
    let struct2 = fake_struct!(type_table, "struct1",
        "d" => i64, "e" => f64, "f" => f64
    );

    let old = &[struct1];
    let new = &[struct2.clone()];

    let diff = diff(old, new);
    assert_eq!(
        diff,
        vec![Diff::Edit {
            diff: vec![
                FieldDiff::Edit {
                    old_type: i64::type_info().clone(),
                    new_type: i64::type_info().clone(),
                    old_index: None,
                    new_index: 0,
                    kind: FieldEditKind::RenamedField,
                },
                FieldDiff::Edit {
                    old_type: f64::type_info().clone(),
                    new_type: f64::type_info().clone(),
                    old_index: None,
                    new_index: 1,
                    kind: FieldEditKind::RenamedField,
                },
                FieldDiff::Edit {
                    old_type: f64::type_info().clone(),
                    new_type: f64::type_info().clone(),
                    old_index: None,
                    new_index: 2,
                    kind: FieldEditKind::RenamedField,
                }
            ],
            old_index: 0,
            new_index: 0,
        }]
    );
    assert_eq_struct(&apply_diff(old, new, diff), &[struct2]);
}
