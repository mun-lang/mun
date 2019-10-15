# Mun Runtime Experiment

[![Build Status][s1]][ci]
[![codecov](https://codecov.io/gh/mun-lang/mun/branch/master/graph/badge.svg)](https://codecov.io/gh/mun-lang/mun)
[![docs page][docs-badge]][docs] [![MIT/Apache][licence-badge]][li]
[![Join us on Discord][s4]][di]
![Lines of Code][s6]

[s1]: https://dev.azure.com/mun-lang/mun/_apis/build/status/mun-lang.mun?branchName=master
[docs-badge]: https://img.shields.io/badge/docs-website-blue.svg
[docs]: https://docs.mun-lang.org/
[licence-badge]: https://img.shields.io/badge/license-MIT%2FApache-blue.svg
[s4]: https://img.shields.io/discord/602227728480993281.svg?logo=discord
[s6]: https://tokei.rs/b1/github/mun-lang/mun?category=code
[ci]: https://dev.azure.com/mun-lang/mun/_build/latest?definitionId=1&branchName=master
[li]: COPYRIGHT
[di]: https://discord.gg/SfvvcCU

An experiment to:

* Detect file changes
* Hot reload Rust shared libraries upon file changes
* Design an API for describing a shared library's symbols
- Reflect type and function information at runtime
* Compare run-time types between different compilation units

## Used resources

- C# Reflection API: https://docs.microsoft.com/en-us/dotnet/csharp/programming-guide/concepts/reflection

## Choices

### Hot reloading

For code to be hot reloadable the main project's dependencies need to be compiled as shared
libraries. All necessary information for matching file changes to their corresponding shared
library can be found using the [cargo](https://github.com/rust-lang/cargo) package manager.

### File changes

[cargo-watch](https://github.com/passcod/cargo-watch) can be used to trigger a process upon file
changes, but as the runtime needs to be running continuously it'd require inter-process
communication. To simplify the experiment, it was opted to integrate a file watcher into the
runtime.

### Reflection

Rust's `Any` trait only generates unique `TypeId`s for types included in a single compilation unit.
Hot reloading also requires comparisons between multiple compilation units, i.e. the main
executable and shared libraries. To that end, the `TypeInfo` struct also contains a `Uuid` that is
shared among all compilation units. For performance, one could still opt to use the `TypeId` (`u64`)
within a single compilation unit. A release build (not consisting of shared libaries) can remove
the `Uuid`s altogether to reduce the memory footprint and improve performance.

## License

The Mun Runtime is licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
 
 at your option.
