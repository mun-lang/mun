## `extern` functions

Extern functions are declared in Mun but their function bodies are defined
externally. They behave exactly the same as regular functions but their
definitions have to be provided to the runtime when loading a Mun library.
Failure to do so will result in a runtime link error, and loading the library
will fail. Take this code for example:

```mun,no_run
{{#include ../listings/ch02-basic-concepts/listing01.mun}}
```

<span class="caption">Listing 2-1: Random bool in Mun</span>

The `random` function is marked as an `extern` function, which means that it
must be provided to the runtime when loading this library.

First building the above code as `main.munlib` and then trying to load the
library in Rust using:

```rust,no_run,noplaypen
# extern crate mun_runtime;
{{#include ../listings/ch02-basic-concepts/listing02.rs}}
```

<span class="caption">Listing 2-2: Load listing 2-1 without adding extern function</span>

will result in an error:

```bash
Failed to link: function `random` is missing.
```

This indicates that we have to provide the runtime with the `random` method,
which we can do through the use of the `insert_fn` method. Let's add a method
that uses the current time as the base of our `random` method:

```rust,no_run,noplaypen
# extern crate mun_runtime;
{{#include ../listings/ch02-basic-concepts/listing03.rs}}
```

<span class="caption">Listing 2-3: Load listing 2-1 with custom `random` function</span>

Note that we have to explicitly cast the function `random` to `extern "C" fn()
-> i64`. This is because each function in Rust has its own unique type.

When we run this now, the error is gone and you should have a function that
returns a random boolean in Mun.
