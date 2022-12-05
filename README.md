# ![Mun][logo-light-mode] ![Mun][logo-dark-mode]

[logo-light-mode]: assets/readme/logotype.svg#gh-light-mode-only
[logo-dark-mode]: assets/readme/logotype-white.svg#gh-dark-mode-only

[![Build Status][build-badge]][build]
[![Crates.io][crates-badge]][crates]
[![docs main][docs-main-badge]][docs-main]
[![docs v0.3][docs-v0.3-badge]][docs-v0.3]
[![MIT/Apache][licence-badge]][license]
[![Join us on Discord][discord-badge]][discord]
[![codecov][coverage-badge]][coverage]
![Lines of Code][lines-of-code-badge]

[build-badge]: https://img.shields.io/github/workflow/status/mun-lang/mun/CI
[build]: https://github.com/mun-lang/mun/actions
[crates-badge]: https://img.shields.io/crates/v/mun.svg
[crates]: https://crates.io/crates/mun/
[coverage-badge]: https://img.shields.io/codecov/c/github/mun-lang/mun.svg
[coverage]: https://codecov.io/gh/mun-lang/mun
[docs-main-badge]: https://img.shields.io/badge/docs-main-blue.svg
[docs-main]: https://docs.mun-lang.org/
[docs-v0.3-badge]: https://img.shields.io/badge/docs-v0.3-blue.svg
[docs-v0.3]: https://docs.mun-lang.org/v0.3/
[licence-badge]: https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue
[license]: COPYRIGHT
[discord-badge]: https://img.shields.io/discord/602227728480993281.svg?logo=discord
[discord]: https://discord.gg/SfvvcCU
[lines-of-code-badge]: https://tokei.rs/b1/github/mun-lang/mun?category=code

_Mun_ is a programming language empowering creation through iteration.

## Features

- **Ahead of time compilation** - Mun is compiled ahead of time (AOT), as
  opposed to being interpreted or compiled just in time (JIT). By detecting
  errors in the code during AOT compilation, an entire class of runtime errors
  is eliminated. This allows developers to stay within the comfort of their IDE
  instead of having to switch between the IDE and target application to debug
  runtime errors.

- **Statically typed** - Mun resolves types at compilation time instead of at
  runtime, resulting in immediate feedback when writing code and opening the
  door for powerful refactoring tools.

- **First class hot-reloading** - Every aspect of Mun is designed with hot
  reloading in mind. Hot reloading is the process of changing code and resources
  of a live application, removing the need to start, stop and recompile an
  application whenever a function or value is changed.

- **Performance** - AOT compilation combined with static typing ensure that Mun
  is compiled to machine code that can be natively executed on any target
  platform. LLVM is used for compilation and optimization, guaranteeing the best
  possible performance. Hot reloading does introduce a slight runtime overhead,
  but it can be disabled for production builds to ensure the best possible
  runtime performance.

- **Cross compilation** - The Mun compiler is able to compile to all supported
  target platforms from any supported compiler platform.

- **Powerful IDE integration** - The Mun language and compiler framework are
  designed to support source code queries, allowing for powerful IDE
  integrations such as code completion and refactoring tools.

## Example

<!-- inline HTML is intentionally used to add the id. This allows retrieval of the HTML -->
<pre language="mun">
<code id="code-sample">fn fibonacci(n: i32) -> i32 {
    if n <= 1 {
        n
    } else {
        fibonacci(n - 1) + fibonacci(n - 2)
    }
}

// Comments: functions marked as `pub` can be called outside the module
pub fn main() {
    // Native support for bool, f32, f64, i8, u8, u128, i128, usize, isize, etc
    let is_true = true;
    let var = 0.5;

    // Type annotations are not required when a variable's type can be deduced
    let n = 3;

    let result = fibonacci(n);

    // Adding a suffix to a literal restricts its type
    let lit = 15u128;

    let foo = record();
    let bar = tuple();
    let baz = on_heap();
}

// Both record structs and tuple structs are supported
struct Record {
    n: i32,
}

// Struct definitions include whether they are allocated by a garbage collector
// (`gc`) and passed by reference, or passed by `value`. By default, a struct
// is garbage collected.
struct(value) Tuple(f32, f32);

struct(gc) GC(i32);

// The order of function definitions doesn't matter
fn record() -> Record {
    // Mun allows implicit returns
    Record { n: 7 }
}

fn tuple() -> Tuple {
    // Mun allows explicit returns
    return Tuple(3.14, -6.28);
}

fn on_heap() -> GC {
    GC(0)
}</code>
</pre>

## Documentation

[The Mun Programming Language Book](https://docs.mun-lang.org/) is hosted on
[netlify](https://www.netlify.com/).

## Pre-Built Binaries

**[NOTE] We do not provide support for milestone releases**

**[NOTE] None of the binaries are currently signed**

Download pre-built binaries of [milestone
releases](https://github.com/mun-lang/mun/releases) for macOS, Linux, and
Windows (64-bit only).

## Building from Source

Make sure you have the following dependencies installed on you machine:

- [Rust](https://www.rust-lang.org/tools/install)
- [LLVM 11](https://docs.mun-lang.org/dev/02-building-llvm.html)

Clone the source code, including all submodules:

```bash
git clone https://github.com/mun-lang/mun.git
git submodule update --init --recursive
```

Use `cargo` to build a release version

```bash
cargo build --release
```

## Language server

Mun contains support for the lsp protocol, start the executable using:

```bash
mun language-server
```

Alternatively, you can install editor-specific extensions.

### VS code

To run in [Visual Studio Code](https://code.visualstudio.com/). Use the following extension:
[VS code extension](https://github.com/mun-lang/vscode-extension).

### Vim/Neovim

Use a language server plugin (or built-in lsp support of neovim), for example using [coc.nvim](https://github.com/neoclide/coc.nvim).

Paste the following config into your `:CocConfig`, replace the `command`, with the correct path to the mun executable.

```json
  "languageserver": {
      "mun": {
          "command": "<path_to_mun>",
          "rootPatterns": ["mun.toml"],
          "trace.server": "verbose",
          "args": ["language-server"],
          "filetypes": ["mun"]
      }
  }
```

Note that, `"trace.server": "verbose"` is optional and helps with language server debugging.

## Building Documentation

Building the book requires
[mdBook](https://github.com/rust-lang-nursery/mdBook), ideally version 0.3.x. To
install it, run:

```
$ cargo install mdbook --vers [version-num]
```

The Mun book uses a [custom version of
Highlight.js](https://github.com/mun-lang/highlight.js) to enable highlighting
of Mun code. The build version of Highlight.js is required by mdbook in the
`theme/` folder but it is not distributed with the source. Instead, it can be
build by invoking the build script:

```bash
cd book
./ci/build-highlight-js
```

Every time you change something in the custom version of highlight.js you have
to call the above script to ensure you locally use the latest version.

After generating the custom minified Highlight.js, to build the book, type:

```
$ mdbook build
```

The output will be in the book subdirectory. To view the book, open it in your
web browser.

For local development use `mdbook serve` instead of `mdbook build`. This will
start a local webserver on port `3000` that serves the book and rebuilds the
content when changes are detected.

All of the above is also combined in a single shell script that can be invoked
by simply running:

```bash
./ci/build
```

To test the `rust` source code in the book, run:

```bash
mdbook test -L path/to/target/debug/deps
```

For this to work, there can only be one `libmun_runtime-{HASH}.rlib` file in the
provided library path.

## License

The Mun Runtime is licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
  http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or
  http://opensource.org/licenses/MIT)

at your option.
