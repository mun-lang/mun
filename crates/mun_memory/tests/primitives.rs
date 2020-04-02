#[test]
fn add() {
    let test1 = vec!["a", "b", "g", "d", "e", "f"];
    let test2 = vec!["g", "h"];
    let d = mun_memory::myers::diff(&test1, &test2);
    println!("d: {:?}", d);
    assert!(true);
}
