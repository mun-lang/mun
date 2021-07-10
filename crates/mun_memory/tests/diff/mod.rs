mod myers;
mod primitives;
mod structs;
mod util;

use mun_memory::diff::{diff, Diff};
use util::*;

#[test]
fn add() {
    let int = TypeInfo::new_fundamental::<i64>();
    let float = TypeInfo::new_fundamental::<f64>();
    let struct1 = TypeInfo::new_struct(
        STRUCT1_NAME,
        STRUCT1_GUID,
        StructInfo::new(&[("a", &int), ("b", &float)]),
    );

    let old = &[int.clone(), struct1.clone()];
    let new = &[int.clone(), struct1.clone(), float.clone()];

    let diff = diff(old, new);
    assert_eq!(diff, vec![Diff::Insert { index: 2 }]);
    assert_eq!(
        apply_diff(old, new, diff),
        vec![int.clone(), struct1.clone(), float.clone()]
    );
}
