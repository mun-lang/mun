## Marshalling

When embedding Mun in other languages, you will probably want to retrieve, modify and send structures across the *boundary* - of the two languages. 
When this so-called *marshalling* occurs, there is often an associated performance penalty because the Mun Runtime needs to perform *runtime checks* to validate the provided data types.

Mun provides a homogeneous interface for marshalling any struct through a `StructRef`- a reference to a heap-allocated struct. 
The Mun Runtime automatically handles the conversion from a function return type into a `StructRef` and function arguments into Mun structs.

> For structs with the `gc` memory kind, marshalling reuses the memory allocated by the garbage collector, but for structs with the `value` memory kind this requires their value to be copied into heap memory.

Listing 3-11 shows how to *marshal* `Vector2` instances from Mun to Rust and vice versa, using the `vector2_new` and `vector2_add` functions - previously [defined](ch03-01-records-vs-tuples.md).

```rust,no_run,noplaypen
{{#include ../listings/ch03-structs/listing11.rs}}
```

<span class="caption">Listing 3-11: Marshalling `Vector2` instances</span>

### Accessing Fields

The API of `StructRef` consists of three generic methods for accessing fields: `get`, `set`, and `replace`; respectively for retrieving, modifying, and replacing a struct field. 
The desired field is specified using a string `field_name` parameter, which is identical to the one used with the dot notation in Mun code.

```rust,no_run,noplaypen
{{#include ../listings/ch03-structs/listing12.rs}}
```

<span class="caption">Listing 3-12: Accessing fields of a `StructRef`</span>
