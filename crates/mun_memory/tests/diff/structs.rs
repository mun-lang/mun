use super::util::*;
use mun_memory::diff::{diff, Diff, FieldDiff, FieldEditKind};

// TODO: Once we can generate `Guid`s based on the data layout, we can just directly check
// `TypeInfo`s against each other.
fn assert_eq_struct(result: &[TypeInfo], expected: &[TypeInfo]) {
    assert_eq!(result.len(), expected.len());
    for (lhs, rhs) in result.into_iter().zip(expected.into_iter()) {
        assert_eq!(lhs.group, rhs.group);
        assert_eq!(lhs.layout, rhs.layout);
        assert_eq!(lhs.tail, rhs.tail);
    }
}

#[test]
fn add() {
    let int = TypeInfo::new_fundamental::<i64>(INT_NAME, INT_GUID);
    let float = TypeInfo::new_fundamental::<f64>(FLOAT_NAME, FLOAT_GUID);

    let struct1 = TypeInfo::new_struct(
        STRUCT1_NAME,
        STRUCT1_GUID,
        StructInfo::new(&[("a", &int), ("b", &float)]),
    );
    let struct2 = TypeInfo::new_struct(
        STRUCT1_NAME,
        STRUCT2_GUID,
        StructInfo::new(&[("c", &float), ("d", &int)]),
    );

    let old = &[&struct1];
    let new = &[&struct1, &struct2];

    let diff = diff(old, new);
    assert_eq!(diff, vec![Diff::Insert { index: 1 }]);
    assert_eq_struct(
        &apply_diff(old, new, diff),
        &vec![struct1.clone(), struct2.clone()],
    );
}

#[test]
fn remove() {
    let int = TypeInfo::new_fundamental::<i64>(INT_NAME, INT_GUID);
    let float = TypeInfo::new_fundamental::<f64>(FLOAT_NAME, FLOAT_GUID);

    let struct1 = TypeInfo::new_struct(
        STRUCT1_NAME,
        STRUCT1_GUID,
        StructInfo::new(&[("a", &int), ("b", &float)]),
    );
    let struct2 = TypeInfo::new_struct(
        STRUCT1_NAME,
        STRUCT2_GUID,
        StructInfo::new(&[("c", &float), ("d", &int)]),
    );

    let old = &[&struct1, &struct2];
    let new = &[&struct1];

    let diff = diff(old, new);
    assert_eq!(diff, vec![Diff::Delete { index: 1 },]);
    assert_eq_struct(&apply_diff(old, new, diff), &vec![struct1.clone()]);
}

#[test]
fn replace() {
    let int = TypeInfo::new_fundamental::<i64>(INT_NAME, INT_GUID);
    let float = TypeInfo::new_fundamental::<f64>(FLOAT_NAME, FLOAT_GUID);

    let struct1 = TypeInfo::new_struct(
        STRUCT1_NAME,
        STRUCT1_GUID,
        StructInfo::new(&[("a", &int), ("b", &float)]),
    );
    let struct2 = TypeInfo::new_struct(
        STRUCT2_NAME,
        STRUCT2_GUID,
        StructInfo::new(&[("c", &float), ("d", &int)]),
    );

    let old = &[&struct1];
    let new = &[&struct2];

    let diff = diff(old, new);
    assert_eq!(
        diff,
        vec![Diff::Delete { index: 0 }, Diff::Insert { index: 0 }]
    );
    assert_eq_struct(&apply_diff(old, new, diff), &vec![struct2.clone()]);
}

#[test]
fn swap() {
    let int = TypeInfo::new_fundamental::<i64>(INT_NAME, INT_GUID);
    let float = TypeInfo::new_fundamental::<f64>(FLOAT_NAME, FLOAT_GUID);

    let struct1 = TypeInfo::new_struct(
        STRUCT1_NAME,
        STRUCT1_GUID,
        StructInfo::new(&[("a", &int), ("b", &float)]),
    );
    let struct2 = TypeInfo::new_struct(
        STRUCT1_NAME,
        STRUCT2_GUID,
        StructInfo::new(&[("c", &float), ("d", &int)]),
    );

    let old = &[&struct1, &struct2];
    let new = &[&struct2, &struct1];

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
    let int = TypeInfo::new_fundamental::<i64>(INT_NAME, INT_GUID);
    let float = TypeInfo::new_fundamental::<f64>(FLOAT_NAME, FLOAT_GUID);

    let struct1 = TypeInfo::new_struct(
        STRUCT1_NAME,
        STRUCT1_GUID,
        StructInfo::new(&[("b", &int), ("c", &float)]),
    );
    let struct2 = TypeInfo::new_struct(
        STRUCT1_NAME,
        STRUCT2_GUID,
        StructInfo::new(&[("a", &int), ("b", &int), ("c", &float)]),
    );

    let old = &[&struct1];
    let new = &[&struct2];

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
    let int = TypeInfo::new_fundamental::<i64>(INT_NAME, INT_GUID);
    let float = TypeInfo::new_fundamental::<f64>(FLOAT_NAME, FLOAT_GUID);

    let struct1 = TypeInfo::new_struct(
        STRUCT1_NAME,
        STRUCT1_GUID,
        StructInfo::new(&[("a", &int), ("c", &float)]),
    );
    let struct2 = TypeInfo::new_struct(
        STRUCT1_NAME,
        STRUCT2_GUID,
        StructInfo::new(&[("a", &int), ("b", &float), ("c", &float)]),
    );

    let old = &[&struct1];
    let new = &[&struct2];

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
    let int = TypeInfo::new_fundamental::<i64>(INT_NAME, INT_GUID);
    let float = TypeInfo::new_fundamental::<f64>(FLOAT_NAME, FLOAT_GUID);

    let struct1 = TypeInfo::new_struct(
        STRUCT1_NAME,
        STRUCT1_GUID,
        StructInfo::new(&[("a", &int), ("b", &float)]),
    );
    let struct2 = TypeInfo::new_struct(
        STRUCT1_NAME,
        STRUCT2_GUID,
        StructInfo::new(&[("a", &int), ("b", &float), ("c", &float)]),
    );

    let old = &[&struct1];
    let new = &[&struct2];

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
    let int = TypeInfo::new_fundamental::<i64>(INT_NAME, INT_GUID);
    let float = TypeInfo::new_fundamental::<f64>(FLOAT_NAME, FLOAT_GUID);

    let struct1 = TypeInfo::new_struct(
        STRUCT1_NAME,
        STRUCT1_GUID,
        StructInfo::new(&[("a", &float), ("b", &float), ("c", &int)]),
    );
    let struct2 = TypeInfo::new_struct(STRUCT1_NAME, STRUCT2_GUID, StructInfo::new(&[("c", &int)]));

    let old = &[&struct1];
    let new = &[&struct2];

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
    let int = TypeInfo::new_fundamental::<i64>(INT_NAME, INT_GUID);
    let float = TypeInfo::new_fundamental::<f64>(FLOAT_NAME, FLOAT_GUID);

    let struct1 = TypeInfo::new_struct(
        STRUCT1_NAME,
        STRUCT1_GUID,
        StructInfo::new(&[("a", &float), ("b", &int), ("c", &float)]),
    );
    let struct2 = TypeInfo::new_struct(STRUCT1_NAME, STRUCT2_GUID, StructInfo::new(&[("b", &int)]));

    let old = &[&struct1];
    let new = &[&struct2];

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
    let int = TypeInfo::new_fundamental::<i64>(INT_NAME, INT_GUID);
    let float = TypeInfo::new_fundamental::<f64>(FLOAT_NAME, FLOAT_GUID);

    let struct1 = TypeInfo::new_struct(
        STRUCT1_NAME,
        STRUCT1_GUID,
        StructInfo::new(&[("a", &int), ("b", &float), ("c", &float)]),
    );
    let struct2 = TypeInfo::new_struct(STRUCT1_NAME, STRUCT2_GUID, StructInfo::new(&[("a", &int)]));

    let old = &[&struct1];
    let new = &[&struct2];

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
    let int = TypeInfo::new_fundamental::<i64>(INT_NAME, INT_GUID);
    let float = TypeInfo::new_fundamental::<f64>(FLOAT_NAME, FLOAT_GUID);

    let struct1 = TypeInfo::new_struct(
        STRUCT1_NAME,
        STRUCT1_GUID,
        StructInfo::new(&[("a", &float), ("b", &int), ("c", &float)]),
    );
    let struct2 = TypeInfo::new_struct(
        STRUCT1_NAME,
        STRUCT2_GUID,
        StructInfo::new(&[("c", &float), ("a", &float), ("b", &int)]),
    );

    let old = &[&struct1];
    let new = &[&struct2];

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
    let int = TypeInfo::new_fundamental::<i64>(INT_NAME, INT_GUID);
    let float = TypeInfo::new_fundamental::<f64>(FLOAT_NAME, FLOAT_GUID);

    let struct1 = TypeInfo::new_struct(
        STRUCT1_NAME,
        STRUCT1_GUID,
        StructInfo::new(&[("a", &float), ("b", &int), ("c", &float), ("d", &int)]),
    );
    let struct2 = TypeInfo::new_struct(
        STRUCT1_NAME,
        STRUCT2_GUID,
        StructInfo::new(&[("d", &int), ("c", &float), ("b", &int), ("a", &float)]),
    );

    let old = &[&struct1];
    let new = &[&struct2];

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
fn rename_field1() {
    let int = TypeInfo::new_fundamental::<i64>(INT_NAME, INT_GUID);
    let float = TypeInfo::new_fundamental::<f64>(FLOAT_NAME, FLOAT_GUID);

    let struct1 = TypeInfo::new_struct(
        STRUCT1_NAME,
        STRUCT1_GUID,
        StructInfo::new(&[("a", &int), ("b", &float), ("c", &float)]),
    );
    let struct2 = TypeInfo::new_struct(
        STRUCT1_NAME,
        STRUCT2_GUID,
        StructInfo::new(&[("a", &int), ("d", &float), ("c", &float)]),
    );

    let old = &[&struct1];
    let new = &[&struct2];

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
    let int = TypeInfo::new_fundamental::<i64>(INT_NAME, INT_GUID);
    let float = TypeInfo::new_fundamental::<f64>(FLOAT_NAME, FLOAT_GUID);

    let struct1 = TypeInfo::new_struct(
        STRUCT1_NAME,
        STRUCT1_GUID,
        StructInfo::new(&[("a", &int), ("b", &float), ("c", &float)]),
    );
    let struct2 = TypeInfo::new_struct(
        STRUCT1_NAME,
        STRUCT2_GUID,
        StructInfo::new(&[("d", &int), ("e", &float), ("f", &float)]),
    );

    let old = &[&struct1];
    let new = &[&struct2];

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
