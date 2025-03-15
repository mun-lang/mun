mod config;
mod db;
mod driver;

use driver::Driver;

#[test]
fn end_to_end() -> anyhow::Result<()> {
    let mut driver = Driver::with_fixture(
        r#"
    //- /src/mod.mun
    pub fn main() -> i32 { foo::foo() }

    fn bar() -> i32 { 5 }

    //- /src/foo.mun
    pub fn foo() -> i32 { super::bar() }
    "#,
    );

    let units = driver.transpile_all_packages()?;
    println!("C units: {units:?}");

    Ok(())
}
