mod util;

use std::io;
use util::*;

#[test]
fn error_assembly_not_linkable() {
    let mut driver = TestDriver::new(
        r"
    extern fn dependency() -> i32;
    
    pub fn main() -> i32 { dependency() }
    ",
    );

    assert_eq!(
        format!("{}", driver.spawn().unwrap_err()),
        format!(
            "{}",
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("Failed to link: function `dependency` is missing."),
            )
        )
    );
}
