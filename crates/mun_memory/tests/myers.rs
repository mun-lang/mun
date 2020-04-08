mod util;

use util::*;

#[test]
fn test1() {
    let old = vec!["a", "b", "g", "d", "e", "f"];
    let new = vec!["g", "h"];
    let diff = mun_memory::myers::diff(&old, &new);
    assert_eq!(apply_myers_diff(&old, &new, diff), new);
}

#[test]
fn add1() {
    let old = vec!["a"];
    let new = vec!["a", "b"];
    let diff = mun_memory::myers::diff(&old, &new);
    assert_eq!(apply_myers_diff(&old, &new, diff), new);
}

#[test]
fn add2() {
    let old = vec!["a"];
    let new = vec!["b", "a"];
    let diff = mun_memory::myers::diff(&old, &new);
    assert_eq!(apply_myers_diff(&old, &new, diff), new);
}

#[test]
fn remove1() {
    let old = vec!["a", "b"];
    let new = vec!["a"];
    let diff = mun_memory::myers::diff(&old, &new);
    assert_eq!(apply_myers_diff(&old, &new, diff), new);
}

#[test]
fn remove2() {
    let old = vec!["a", "b"];
    let new = vec!["b"];
    let diff = mun_memory::myers::diff(&old, &new);
    assert_eq!(apply_myers_diff(&old, &new, diff), new);
}
