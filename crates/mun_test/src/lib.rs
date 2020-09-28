//! Mun Test
//!
//! Mun Test contains shared functionality for testing Mun crates.
#![warn(missing_docs)]

mod driver;
mod fixture;

pub use driver::*;
pub use fixture::{trim_raw_string_literal, Fixture};
