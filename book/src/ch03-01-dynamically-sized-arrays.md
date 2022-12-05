> **Warning**
>
> Array functionality is still very basic as you cannot resize arrays (incl. pushing elements) at runtime.
> You can only get, set, and replace array elements.
> Future releases of Mun will extend this functionality.

## Dynamically Sized Arrays

In Mun, the default array type is dynamically sized and heap-allocated.
To create an array, write its values as a comma-separated list inside square brackets, as shown in Listing 3-1.

<!-- HACK: Add an extension to support hiding of Mun code -->

```rust,ignore
{{#include ../listings/ch03-arrays/listing01.mun}}
```

<span class="caption">Listing 3-1: Creating an `array` instance</span>

An array's type is written using square brackets around the type of each element, as demonstrated in Listing 3-2.
This is only necessary when the compiler cannot automatically deduce the type of the array based on the context; although you you can always manually notate your code.

<!-- HACK: Add an extension to support hiding of Mun code -->

```rust,ignore
{{#include ../listings/ch03-arrays/listing02.mun}}
```

<span class="caption">Listing 3-2: Creating an `array` instance of type `[u64]`</span>

### Accessing Array Elements

An array is a single chunk of heap-allocated memory of dynamic size. You can access elements of an array using indexing, as illustrated in Listing 3-3.

<!-- HACK: Add an extension to support hiding of Mun code -->

```rust,ignore
{{#include ../listings/ch03-arrays/listing03.mun}}
```

<span class="caption">Listing 3-3: Accessing elements of an `array` instance`</span>

In this example, the variable named `first` will get the value `1`, because that is the value at index `[0]` in the array.
The variable named `second` will get the value `2` from index `[1]` in the array.

### Invalid Array Element Access

> **Warning**
>
> Mun is still in early development and should thus be considered **unsafe**!
> In particular for arrays, as they're allowed to perform _out-of-bounds_ memory access.

Currently, Mun does **not** check for invalid array element access.
As such, it will attempt to access an array element, even if it is out of bounds.
You will have to manually prevent out-of-bounds access; e.g. as shown in Listing 3-4.

<!-- HACK: Add an extension to support hiding of Mun code -->

```rust,ignore
{{#include ../listings/ch03-arrays/listing04.mun}}
```

<span class="caption">Listing 3-4: Preventing invalid element access of an `array` instance</span>

When Mun implements a way to _panic_, this will change.
