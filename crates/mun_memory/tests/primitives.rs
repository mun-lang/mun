mod util;

use util::*;

#[test]
fn add() {
    let int = TypeInfo::new_fundamental::<i64>(INT_NAME, INT_GUID);
    let float = TypeInfo::new_fundamental::<f64>(FLOAT_NAME, FLOAT_GUID);

    let old = &[&int];
    let new = &[&int, &float];

    let diff = mun_memory::diff(old, new);
    assert_eq!(diff, vec![mun_memory::Diff::Insert { index: 1 }]);
    assert_eq!(apply_diff(old, new, diff), vec![int.clone(), float.clone()]);
}

#[test]
fn remove() {
    let int = TypeInfo::new_fundamental::<i64>(INT_NAME, INT_GUID);
    let float = TypeInfo::new_fundamental::<f64>(FLOAT_NAME, FLOAT_GUID);

    let old = &[&int, &float];
    let new = &[&float];

    let diff = mun_memory::diff(old, new);
    assert_eq!(diff, vec![mun_memory::Diff::Delete { index: 0 },]);
    assert_eq!(apply_diff(old, new, diff), vec![float.clone()]);
}

#[test]
fn replace() {
    let int = TypeInfo::new_fundamental::<i64>(INT_NAME, INT_GUID);
    let float = TypeInfo::new_fundamental::<f64>(FLOAT_NAME, FLOAT_GUID);

    let old = &[&int];
    let new = &[&float];

    let diff = mun_memory::diff(old, new);
    assert_eq!(
        diff,
        vec![
            mun_memory::Diff::Delete { index: 0 },
            mun_memory::Diff::Insert { index: 0 }
        ]
    );
    assert_eq!(apply_diff(old, new, diff), vec![float.clone()]);
}

#[test]
fn swap() {
    let int = TypeInfo::new_fundamental::<i64>(INT_NAME, INT_GUID);
    let float = TypeInfo::new_fundamental::<f64>(FLOAT_NAME, FLOAT_GUID);

    let old = &[&int, &float];
    let new = &[&float, &int];

    let diff = mun_memory::diff(old, new);
    assert_eq!(
        diff,
        vec![mun_memory::Diff::Move {
            old_index: 0,
            new_index: 1
        },]
    );
    assert_eq!(apply_diff(old, new, diff), vec![float.clone(), int.clone()]);
}
