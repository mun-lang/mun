# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- Reduce workspace target folder size from 10.9 GB to 10.6 GB
- Reduce workspace build dependencies from 296 to 270
- Reduce workspace dev dependencies from 502 to 496

## [0.3.0] - 2021-04-12

The third Mun release includes big usability improvements; multi-file projects, a language server with diagnostics and autocompletion, and improvements to robustness and developer workflow to name a few.

Special thanks to @emi2k01, @tdejager, @ethanboxx, @sinato, @dependabot, @legendiguess, and @sburris0 for their contributions to this release.

### Added

- generate rust tests for code snippets in book [#311](https://github.com/mun-lang/mun/pull/311)
- support for completions [#306](https://github.com/mun-lang/mun/pull/306)
- add mut keyword [#305](https://github.com/mun-lang/mun/pull/305)
- describe how to install/build LLVM [#304](https://github.com/mun-lang/mun/pull/304)
- runtime linking [#300](https://github.com/mun-lang/mun/pull/300)
- implements incremental file updates [#298](https://github.com/mun-lang/mun/pull/298)
- adds document symbol provider  [#297](https://github.com/mun-lang/mun/pull/297)
- add option to emit IR [#296](https://github.com/mun-lang/mun/pull/296)
- integrated new vfs [#294](https://github.com/mun-lang/mun/pull/294)
- adds lsp document symbol provider [#293](https://github.com/mun-lang/mun/pull/293)
- shorten commit hash with environment file [#292](https://github.com/mun-lang/mun/pull/292)
- use statements language support [#290](https://github.com/mun-lang/mun/pull/290)
- make type-specific data (such as StructInfo) part of TypeInfo [#287](https://github.com/mun-lang/mun/pull/287)
- add AsValue macro support for enums [#286](https://github.com/mun-lang/mun/pull/286)
- alignment of struct fields [#285](https://github.com/mun-lang/mun/pull/285)
- adds modules and visibility [#283](https://github.com/mun-lang/mun/pull/283)
- adds fixtures to support multiple files from string [#272](https://github.com/mun-lang/mun/pull/272)
- refactored RawItems into ItemTree [#271](https://github.com/mun-lang/mun/pull/271)
- split database and added docs [#267](https://github.com/mun-lang/mun/pull/267)
- use Idx<T> instead of macro in arena [#266](https://github.com/mun-lang/mun/pull/266)
- never return type in let initializer [#264](https://github.com/mun-lang/mun/pull/264)
- add mdbook plugin for testing mun code in book [#263](https://github.com/mun-lang/mun/pull/263)
- emit and link bitcode files instead of obj does not work on MacOS [#261](https://github.com/mun-lang/mun/pull/261)
- use custom prebuild llvm distribution on ubuntu [#260](https://github.com/mun-lang/mun/pull/260)
- emit and link bitcode files instead of object files [#258](https://github.com/mun-lang/mun/pull/258)
- move test utility functions to separate crate [#253](https://github.com/mun-lang/mun/pull/253)
- move library loading logic to separate crate [#252](https://github.com/mun-lang/mun/pull/252)
- type alias [#251](https://github.com/mun-lang/mun/pull/251)
- Implement `mun new` and `mun init` [#246](https://github.com/mun-lang/mun/pull/246)
- removed outdated comments [#245](https://github.com/mun-lang/mun/pull/245)
- upgrade to salsa 0.15 [#244](https://github.com/mun-lang/mun/pull/244)
- parsing of unmatched right curly braces ('}') [#243](https://github.com/mun-lang/mun/pull/243)
- explicitly specify latest Ubuntu LTS [#241](https://github.com/mun-lang/mun/pull/241)
- build binaries for release branches [#240](https://github.com/mun-lang/mun/pull/240)
- shared diagnostics between compiler and language server [#239](https://github.com/mun-lang/mun/pull/239)
- add benchmarks for and optimise struct field marshalling [#238](https://github.com/mun-lang/mun/pull/238)
- initial LSP support [#236](https://github.com/mun-lang/mun/pull/236)

### Changed

- updated book for 0.3 changes [#314](https://github.com/mun-lang/mun/pull/314)
- update runtime FFI [#312](https://github.com/mun-lang/mun/pull/312)
- Inkwell beta.2 and LLVM11 [#303](https://github.com/mun-lang/mun/pull/303)
- upgrade to official inkwell [#254](https://github.com/mun-lang/mun/pull/254)
- generate C ABI from Rust code [#255](https://github.com/mun-lang/mun/pull/255)

### Removed

- removed async code and switched to lsp_server  [#295](https://github.com/mun-lang/mun/pull/295)

### Fixed

- parser performance issues [#307](https://github.com/mun-lang/mun/pull/307)
- manually extract llvm release to get more output [#302](https://github.com/mun-lang/mun/pull/302)
- adds logging to windows llvm install [#301](https://github.com/mun-lang/mun/pull/301)
- fix broken book link and CoC link [#284](https://github.com/mun-lang/mun/pull/284)
- 7zip issues [#280](https://github.com/mun-lang/mun/pull/280)
- dont run mun tests on CI [#278](https://github.com/mun-lang/mun/pull/278)


## [0.2.1] - 2020-07-08

This patch release for Mun v0.2.0 includes a variety of bug fixes.

Special thanks to @RadicalZephyr, @benediktwerner, and @fominok for their contributions to this release; and to @jDomantas and @sigmaSd for the initial discovery and reporting of fixed issues.

### Added

- Type-safe inkwell types [#202](https://github.com/mun-lang/mun/pull/202)

### Fixed

- Remove return-blocking semicolon from ch01 listing 01 of the book [#192](https://github.com/mun-lang/mun/pull/192)
- Code blocks in ch02-02-functions of the book [#194](https://github.com/mun-lang/mun/pull/194)
- Replace overly complicated redirection rules with a simple hack [#196](https://github.com/mun-lang/mun/pull/196)
- Missing argument for `fibonacci` function in the book [#197](https://github.com/mun-lang/mun/pull/197)
- Replace `float` with `f32` in the book [#204](https://github.com/mun-lang/mun/pull/204)
- Windows libclang issues [#205](https://github.com/mun-lang/mun/pull/205)
- Crash on missing nested private function [#221](https://github.com/mun-lang/mun/pull/221)
- Panic when using `mun build` [#229](https://github.com/mun-lang/mun/pull/229)
- `clippy::many_single_char_names` in macro [#231](https://github.com/mun-lang/mun/pull/231)
- Compiler panics when accessing a field of a temporary [#232](https://github.com/mun-lang/mun/pull/232)
- LLVM assertions [#233](https://github.com/mun-lang/mun/pull/233)
- Proper tarpaulin skip attribute [#235](https://github.com/mun-lang/mun/pull/235)

## [0.2.0] - 2020-05-15

The second Mun release includes big new features such as hot reloading support for data structures, garbage collection, and full operator and literal support for fundamental types.

Special thanks to @legendiguess and @jakbyte for their contributions to this release.

### Added

- Updated binaries for Runtime C API [#184](https://github.com/mun-lang/mun/pull/184)
- crates.io publishing metadata [#183](https://github.com/mun-lang/mun/pull/183)
- Mun book in main repository [#182](https://github.com/mun-lang/mun/pull/182)
- Cloning instructions in README [#179](https://github.com/mun-lang/mun/pull/179)
- Log upon assembly reload [#175](https://github.com/mun-lang/mun/pull/175)
- Buoyancy example [#174](https://github.com/mun-lang/mun/pull/174)
- Zero initialise fields with different `struct` types during memory mapping
- Updated code sample in README [#181](https://github.com/mun-lang/mun/pull/181)
- Map fields with different `struct` memory kinds during memory mapping [#171](https://github.com/mun-lang/mun/pull/171)
- Support for adding extern functions in Runtime CAPI [#169](https://github.com/mun-lang/mun/pull/169)
- Split `FunctionInfo` into signature, prototype, and definition [#166](https://github.com/mun-lang/mun/pull/166)
- Test for type conversion during memory mapping [#164](https://github.com/mun-lang/mun/pull/164)
- Garbage collection methods in Runtime CAPI [#163](https://github.com/mun-lang/mun/pull/163)
- Number type inferencing [#154](https://github.com/mun-lang/mun/pull/154)
- Return `Rc<RefCell<Runtime>>` from `RuntimeBuilder` [#153](https://github.com/mun-lang/mun/pull/153)
- left-shift (`<<` and `<<=`), right-shift (`>>` and `>>=`) operators
- bitwise and (`&` and `&=`), or (`|` and `|=`), xor (`^` and `^=`) operators
- `bool` and (`&&`), or (`||`) operators
- `struct` assignment (`=`) operator
- `bool` assignment (`=`) operator [#144](https://github.com/mun-lang/mun/pull/144)
- `StructRef` can be cloned [#143](https://github.com/mun-lang/mun/pull/143)
- Clarified usage of unsafe code
- Cast fundamental types during `struct` memory mapping [#140](https://github.com/mun-lang/mun/pull/140)
- Unary `!` and `-` operators [#136](https://github.com/mun-lang/mun/pull/136)
- `%` and `%=` operators [#135](https://github.com/mun-lang/mun/pull/135)
- Missing space in `invoke_fn15` function [#132](https://github.com/mun-lang/mun/pull/132)
- Merged `file_ir` and `group_ir` snapshots [#131](https://github.com/mun-lang/mun/pull/131)
- Runtime support for `extern` functions without return type [#127](https://github.com/mun-lang/mun/pull/127)
- `i128` and `u128` integer types [#124](https://github.com/mun-lang/mun/pull/124)
- Use `->` instead of `:` for function return types [#123](https://github.com/mun-lang/mun/pull/123)
- Allow underscores in numeric literals
- Hex, binary, and octal literals
- Typed literals [#122](https://github.com/mun-lang/mun/pull/122)
- `struct` memory mapping [#117](https://github.com/mun-lang/mun/pull/117)
- Retrieve `TypeInfo` and `StructInfo` during calls in a `StructRef` [#109](https://github.com/mun-lang/mun/pull/109)
- Performance benchmarks [#104](https://github.com/mun-lang/mun/pull/104)
- Test for incremental compilation [#102](https://github.com/mun-lang/mun/pull/102)
- Garbage collection using mark & sweep [#99](https://github.com/mun-lang/mun/pull/99)
- Size and alignment of types in ABI [#98](https://github.com/mun-lang/mun/pull/98)
- Heap-allocated object management using pointer indirection [#97](https://github.com/mun-lang/mun/pull/97)
- `extern` functions [#96](https://github.com/mun-lang/mun/pull/96)
- Marshalling of `struct(value)` types [#93](https://github.com/mun-lang/mun/pull/93)
- Restrict symbol generation to `pub` functions [#92](https://github.com/mun-lang/mun/pull/92)
- Integration with [annotate-snippets](https://crates.io/crates/annotate-snippets) crate [#91](https://github.com/mun-lang/mun/pull/91)
- Support for `extern` functions in the dispatch table [#90](https://github.com/mun-lang/mun/pull/90)
- Unit tests for Runtime CAPI
- Marshalling of fields with the `struct` type [#87](https://github.com/mun-lang/mun/pull/87)
- Unit test for `LineIndex::line_str` function [#86](https://github.com/mun-lang/mun/pull/86)
- `MunStructInfo` is appended to `MunTypeInfo` for `struct` types [#84](https://github.com/mun-lang/mun/pull/84)
- Marshalling of `struct` types [#83](https://github.com/mun-lang/mun/pull/83)
- Improved error messages for missing function signatures [#80](https://github.com/mun-lang/mun/pull/80)
- License, homepage, and repository information in README [#71](https://github.com/mun-lang/mun/pull/71)
- Simple binary operation type checking [#70](https://github.com/mun-lang/mun/pull/70)
- Tools for manual generation of ABI & runtime CAPI bindings [#69](https://github.com/mun-lang/mun/pull/69)
- Test UTF-8 validity of compiler-generated `CStr` [#67](https://github.com/mun-lang/mun/pull/67)
- Optimised `CStr::from_ptr(ptr).to_str()` to `from_utf8_unchecked` in ABI [#66](https://github.com/mun-lang/mun/pull/66)
- LLVM install instructions for Arch Linux in README [#65](https://github.com/mun-lang/mun/pull/65)
- ABI support for `struct` types
- `struct` literals
- `struct` declarations [#64](https://github.com/mun-lang/mun/pull/64)
- `while` expression [#63](https://github.com/mun-lang/mun/pull/63)
- `break` expression [#62](https://github.com/mun-lang/mun/pull/62)
- Changed crate authors [#61](https://github.com/mun-lang/mun/pull/61)
- `loop` expression [#60](https://github.com/mun-lang/mun/pull/60)
- Incremental compilation when hot reloading [#49](https://github.com/mun-lang/mun/pull/49)

### Changed

- Removed old snapshots [#170](https://github.com/mun-lang/mun/pull/170)
- Updated badges in README [#162](https://github.com/mun-lang/mun/pull/162)
- Updated code sample in README [#161](https://github.com/mun-lang/mun/pull/161)
- Simplified `MemoryMapper` API [#142](https://github.com/mun-lang/mun/pull/142)
- Split artifact generation and CI tests [#141](https://github.com/mun-lang/mun/pull/141)
- Lock [cbindgen](https://crates.io/crates/cbindgen) dependency [#126](https://github.com/mun-lang/mun/pull/126)
- Updated Arch Linux install instructions in README [#116](https://github.com/mun-lang/mun/pull/116)
- Code coverage using tarpaulin instead of grcov [#100](https://github.com/mun-lang/mun/pull/100)
- Compiled libraries use `munlib` extension [#75](https://github.com/mun-lang/mun/pull/75)
- Use codecov.io instead of coveralls [#59](https://github.com/mun-lang/mun/pull/59)
- Install instructions in README [#55](https://github.com/mun-lang/mun/pull/55)

### Removed

- Removed `float`, `uint`, and `int` types [#157](https://github.com/mun-lang/mun/pull/157)

In addition, there were a lot of bug fixes.

## [0.1.0] - 2019-11-11

### Added
- Support all fundamental types as return types when starting a Mun library from the CLI [#45](https://github.com/mun-lang/mun/pull/45)
- `return` expressions [#38](https://github.com/mun-lang/mun/pull/38)
- Statically link against liblld instead of spawning as process [#37](https://github.com/mun-lang/mun/pull/37)
- Generation and upload of artifacts [#36](https://github.com/mun-lang/mun/pull/36)
- Github actions continuous integration [#30](https://github.com/mun-lang/mun/pull/30)
- Update operators and diagnostics [#29](https://github.com/mun-lang/mun/pull/29)
- Example of hot reloading in the Mun Runtime
- Error reporting
- C++ bindings for the Mun Runtime [#25](https://github.com/mun-lang/mun/pull/25)
- Run clippy on CI and pre-commit
- Diagnostics for mismatching or missing `else`
- `if` expressions code generation
- `if` statement type checking
- `never` type
- Add cargo husky to enable automatic git hooks
- Testing of type inferencing
- Parsing of `if` statements [#24](https://github.com/mun-lang/mun/pull/24)
- Comparison operators [#23](https://github.com/mun-lang/mun/pull/23)
- Automatic generation of C bindings for the runtime [#22](https://github.com/mun-lang/mun/pull/22)
- Trait extension of Result type that allows retrying and waiting for a correct result [#15](https://github.com/mun-lang/mun/pull/15)
- Integrate dispatch table in Mun runtime [#11](https://github.com/mun-lang/mun/pull/11)
- Dispatch table
- Generation of function call IR
- Function call inferencing [#9](https://github.com/mun-lang/mun/pull/9)
- Command-line interface for Mun Compiler and Mun Runtime
- Compiler daemon that detects changed files and recompiles them
- Runtime builder [#6](https://github.com/mun-lang/mun/pull/6)
- Detection of duplicate definition names [#5](https://github.com/mun-lang/mun/pull/5)
- badges & licenses [#1](https://github.com/mun-lang/mun/pull/1)


