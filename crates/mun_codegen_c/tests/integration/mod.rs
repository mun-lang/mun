mod config;
mod db;
mod driver;

use driver::Driver;

#[test]
fn end_to_end() -> anyhow::Result<()> {
    let (mut driver, file_id) = Driver::with_text(
        r#"
pub fn foo() -> i32 {
    42
}

pub fn main() {
    let x = foo();
}
    "#,
    )?;

    let units = driver.generate_all()?;
    println!("C units: {units:?}");

    Ok(())
}
