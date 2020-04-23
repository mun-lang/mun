use super::util::*;
use mun_memory::diff::{diff, Diff};

#[test]
fn add() {
    let int = TypeInfo::new_fundamental::<i64>();
    let float = TypeInfo::new_fundamental::<f64>();

    let old = &[&int];
    let new = &[&int, &float];

    let diff = diff(old, new);
    assert_eq!(diff, vec![Diff::Insert { index: 1 }]);
    assert_eq!(apply_diff(old, new, diff), vec![int.clone(), float.clone()]);
}

#[test]
fn remove() {
    let int = TypeInfo::new_fundamental::<i64>();
    let float = TypeInfo::new_fundamental::<f64>();

    let old = &[&int, &float];
    let new = &[&float];

    let diff = diff(old, new);
    assert_eq!(diff, vec![Diff::Delete { index: 0 },]);
    assert_eq!(apply_diff(old, new, diff), vec![float.clone()]);
}

#[test]
fn replace() {
    let int = TypeInfo::new_fundamental::<i64>();
    let float = TypeInfo::new_fundamental::<f64>();

    let old = &[&int];
    let new = &[&float];

    let diff = diff(old, new);
    assert_eq!(
        diff,
        vec![Diff::Delete { index: 0 }, Diff::Insert { index: 0 }]
    );
    assert_eq!(apply_diff(old, new, diff), vec![float.clone()]);
}

#[test]
fn swap() {
    let int = TypeInfo::new_fundamental::<i64>();
    let float = TypeInfo::new_fundamental::<f64>();

    let old = &[&int, &float];
    let new = &[&float, &int];

    let diff = diff(old, new);
    assert_eq!(
        diff,
        vec![Diff::Move {
            old_index: 0,
            new_index: 1
        },]
    );
    assert_eq!(apply_diff(old, new, diff), vec![float.clone(), int.clone()]);
}
