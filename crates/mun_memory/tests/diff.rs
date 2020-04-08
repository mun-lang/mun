mod util;

use util::*;

#[test]
fn add() {
    let int = TypeInfo::new_fundamental::<i64>(INT_NAME, INT_GUID);
    let float = TypeInfo::new_fundamental::<f64>(FLOAT_NAME, FLOAT_GUID);
    let struct1 = TypeInfo::new_struct(
        STRUCT1_NAME,
        STRUCT1_GUID,
        StructInfo::new(&[("a", &int), ("b", &float)]),
    );

    let old = &[&int, &struct1];
    let new = &[&int, &struct1, &float];

    let diff = mun_memory::diff(old, new);
    assert_eq!(diff, vec![mun_memory::Diff::Insert { index: 2 }]);
    assert_eq!(
        apply_diff(old, new, diff),
        vec![int.clone(), struct1.clone(), float.clone()]
    );
}
