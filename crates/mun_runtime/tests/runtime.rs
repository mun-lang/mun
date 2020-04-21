mod util;

use std::io;
use util::*;

#[test]
fn error_assembly_not_linkable() {
    let mut driver = TestDriver::new(
        r"
    extern fn dependency() -> int;
    
    pub fn main() -> int { dependency() }
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
