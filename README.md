# Mun

[![Build Status](https://github.com/mun-lang/mun/workflows/CI/badge.svg?branch=master)](https://github.com/mun-lang/mun/actions)
[![Coverage Status](https://coveralls.io/repos/github/mun-lang/mun/badge.svg?branch=master)](https://coveralls.io/github/mun-lang/mun?branch=master)
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

*Mun* is a programming language empowering creation through iteration.

## Features

- **Ahead of time compilation** - Mun is compiled ahead of time (AOT), as opposed to being
  interpreted or compiled just in time (JIT). By detecting errors in the code during AOT
  compilation, an entire class of runtime errors is eliminated. This allows developers to stay
  within the comfort of their IDE instead of having to switch between the IDE and target application
  to debug runtime errors.

- **Statically typed** - Mun resolves types at compilation time instead of at runtime, resulting in
  immediate feedback when writing code and opening the door for powerful refactoring tools.

- **First class hot-reloading** - Every aspect of Mun is designed with hot reloading in mind. Hot
  reloading is the process of changing code and resources of a live application, removing the need
  to start, stop and recompile an application whenever a function or value is changed.

- **Performance** - AOT compilation combined with static typing ensure that Mun is compiled to
  machine code that can be natively executed on any target platform. LLVM is used for compilation
  and optimization, guaranteeing the best possible performance. Hot reloading does introduce a
  slight runtime overhead, but it can be disabled for production builds to ensure the best possible
  runtime performance.

- **Cross compilation** - The Mun compiler is able to compile to all supported target platforms from
  any supported compiler platform.

- **Powerful IDE integration** *not implemented yet* - The Mun language and compiler framework are
  designed to support source code queries, allowing for powerful IDE integrations such as code
  completion and refactoring tools.

## Examples

```mun
fn main() {
    let sum = add(a, b);

    // Comments: Mun natively supports bool, float, and int
    let is_true = true;
    let var: float = 0.5;
    
}

// The order of function definitions doesn't matter
fn add(a: int, b: int): int {
    a + b
}
```

## Documentation

[The Mun Programming Language Book](https://docs.mun-lang.org/)

## Building from Source

### Installing dependencies

Make sure you have the following dependencies installed on you machine:

#### Rust

Install the latest stable version of Rust, [e.g. using
rustup](https://www.rust-lang.org/tools/install). 

#### LLVM

Mun targets LLVM 7.1.0. Installing LLVM is platform dependant and as such can be a pain. The
following steps are how we install LLVM on [our CI
runners](.github/actions/install-llvm/index.js):

* ***nix**: Package managers of recent *nix distros can install binary versions of LLVM, e.g.:
  ```bash
  # Ubuntu 18.04
  sudo apt install llvm-7 llvm-7-* liblld-7*
  ```
* **macOS**: [Brew](https://brew.sh/) contains a binary distribution of LLVM 7.1.0. However, as it's
  not the latest version, it won't be added to the path. We are using
  [llvm-sys](https://crates.io/crates/llvm-sys) to manage version, but another option is to export 
  the `LLVM_SYS_70_PREFIX` variable, which will not clutter your `PATH`. To install:
  ```bash
  brew install llvm@7
  # Export LLVM_SYS_PREFIX to not clubber PATH
  export LLVM_SYS_PREFIX=$(brew --prefix llvm@7)
  ```
* **windows**: Binary distrubutions are available for Windows on the LLVM website, but they
  do not contain a number of libraries that are required by Mun. To avoid having to go to the 
  trouble of compiling LLVM yourself, we created a
  [repository](https://github.com/mun-lang/llvm-package-windows) that automatically compiles the
 required binaries. It also contains a
  [release](https://github.com/mun-lang/llvm-package-windows/releases/download/v7.1.0/llvm-7.1.0-windows-x64-msvc15.7z)
  that you can download and extract to your machine. Once downloaded and extracted, add the `<extract_dir>/bin`
  folder to the `PATH` environment variable.

### Compiling

```
cargo build --release
```

## License

The Mun Runtime is licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or 
   http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
 
 at your option.
