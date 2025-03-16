mod config;
mod db;
mod driver;
mod snapshot;

use snapshot::assert_snapshot_of_transpiled_fixture;

#[test]
fn end_to_end() {
    assert_snapshot_of_transpiled_fixture!("\
//- /src/mod.mun
pub fn main() -> i32 { foo::foo() }

fn bar() -> i32 { 5 }

//- /src/foo.mun
pub fn foo() -> i32 { super::bar() }", @""
    );
}
