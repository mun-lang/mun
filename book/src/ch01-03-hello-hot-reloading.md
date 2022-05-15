## Hello, Hot Reloading!

Mun distinguishes itself from other languages by its inherent hot reloading capabilities. 
The following example illustrates how you can create a hot reloadable application by slightly modifying the [Hello, fibonacci?](ch01-02-hello-fibonacci.md) example. 
In Listing 1-2, the `fibonacci_n` function has been removed and the `pub` keyword has been added to both `args` and `fibonacci`.

Filename: `src/mod.mun`

<!-- HACK: Add an extension to support hiding of Mun code -->
<!-- https://github.com/rust-lang/mdBook/pull/1339 -->
```mun
{{#include ../listings/ch01-getting-started/listing02.mun}}
```

<span class="caption">Listing 1-2: A function that calculates a fibonacci number</span>

Apart from running Mun libraries from the command-line interface, a common use case is embedding them in other programming languages.

### Mun embedded in C++

Mun [exposes](https://github.com/mun-lang/mun/tree/main/cpp) a C API and complementary C++ bindings for the Mun Runtime. 
Listing 1-3 shows a C++ application that constructs a Mun Runtime for the `hello_fibonacci` library and continuously invokes the `fibonacci` function and outputs its result.

Filename: main.cc

```cpp
{{#include ../listings/ch01-getting-started/listing03.cpp}}
```

<span class="caption">Listing 1-3: Hello, Fibonacci? embedded in a C++ application</span>

### Mun embedded in Rust

As the Mun Runtime is written in Rust, it can be easily embedded in Rust applications by adding the `mun_runtime` crate as a dependency. 
Listing 1-4 illustrates a simple Rust application that builds a Mun Runtime and continuously invokes the `fibonacci` function and prints its output.

Filename: mod.rs

```rust,no_run,noplaypen
# extern crate mun_runtime;
{{#include ../listings/ch01-getting-started/listing04.rs}}
```

<span class="caption">Listing 1-4: Hello, Fibonacci? embedded in a Rust application</span>

### Hot Reloading

The prior examples both update the runtime every loop cycle. 
In the background, this detects recompiled code and reloads the resulting Mun libraries.

To ensure that the Mun compiler recompiles our code every time the `mod.mun` source file from Listing 1-2 changes, the `--watch` argument must be added:

```bash
mun build --watch
```

When saved, changes in the source file will automatically take effect in the running example application.
E.g. change the return value of the `arg` function and the application will log the corresponding Fibonacci number.

Some changes, such as a type mismatch between the compiled application and the hot reloadable library, can lead to runtime errors.
When these occur, theruntime will log the error and halt until an update to the source code arrives.

That's it!
Now you are ready to start developing hot reloadable Mun libraries.
