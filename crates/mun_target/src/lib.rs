//! This crate contains specifications on how to build Mun and how to configure
//! several tools to generate machine code.
//!
//! It is heavily inspired by the
//! [librustc_target](https://github.com/rust-lang/rust/tree/master/src/librustc_target) crate.

pub mod abi;
pub mod spec;

/// Returns the target triple of the host machine. This can be used as a default
/// target.
pub fn host_triple() -> &'static str {
    // Get the host triple out of the build environment. This ensures that our
    // idea of the host triple is the same as for the set of libraries we've
    // actually built.  We can't just take LLVM's host triple because they
    // normalize all ix86 architectures to i386.
    //
    // Instead of grabbing the host triple (for the current host), we grab (at
    // compile time) the target triple that this rustc is built with and
    // calling that (at runtime) the host triple.
    env!("CFG_COMPILER_HOST_TRIPLE")
}
