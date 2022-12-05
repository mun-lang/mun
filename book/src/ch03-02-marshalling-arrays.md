> **Warning**
>
> Array functionality is still very basic as you cannot resize arrays (incl. pushing elements) at runtime.
> You can only get, set, and replace array elements.
> Future releases of Mun will extend this functionality.

## Marshalling Arrays

When embedding Mun in other languages, you will probably want to marshal arrays to and from another language.
Mun provides a homogeneous interface for marshalling any array through an `ArrayRef`- a reference to a heap-allocated array.
The Mun Runtime automatically handles the conversion from a function return type into an `ArrayRef` and function arguments into Mun arrays.

> Marshalling reuses the memory allocated by the Mun garbage collector for arrays.

Listing 3-5 shows how to _marshal_ `array` instances from Mun to Rust and vice versa, using the `generate` and `add_one` functions - previously [defined](ch03-01-dynamically-sized-arrays.md).

```rust,no_run,noplaypen
{{#include ../listings/ch03-arrays/listing05.rs}}
```

<span class="caption">Listing 3-5: Marshalling `array` instances</span>

### Array methods

The API of `ArrayRef` contains two other methods for interacting with its data: `capacity` and `len`; respectively for retrieving the array's capacity and length:

```rust,no_run,noplaypen
{{#include ../listings/ch03-arrays/listing05.rs:13:16}}
}
```

### Iterating elements

To obtain an iterator over the `ArrayRef` instance's elements, you can call the `iter` function, which returns an `impl Iterator`:

```rust,no_run,noplaypen
{{#include ../listings/ch03-arrays/listing05.rs:18:22}}
```
