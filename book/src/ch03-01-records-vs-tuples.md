## Records vs Tuples

Mun supports two types of structures: _record structs_ and _tuple structs_. A record `struct`
definition specifies both the name and type of each piece of data, allowing you to retrieve the
_field_ by name. For example, Listing 3-1 shows a record `struct` that stores a 2-dimensional
vector.

<!-- HACK: Add an extension to support hiding of Mun code -->
```rust,ignore 
{{#include ../listings/ch03-structs/listing01.mun}}
```

<span class="caption">Listing 3-1: A record `struct` definition for a 2D vector</span>

In contrast, tuple `struct` definitions omit field names; only specifying the field types. Using a tuple `struct` makes sense when you want to associate a name with a tuple or distinguish it from
other tuples' types, but naming each field would be redundant. Listing 3-2 depicts a tuple `struct`
that stores a 3-dimensional vector.

<!-- HACK: Add an extension to support hiding of Mun code -->
```rust,ignore 
{{#include ../listings/ch03-structs/listing02.mun}}
```

<span class="caption">Listing 3-2: A tuple `struct` definition for a 3D vector</span>

### Create a Struct Instance

To use a record `struct`, we create an _instance_ of that struct by stating the name of the
`struct` and then add curly braces containing `key: value` pairs for each of its fields. The keys
have to correspond to the field names in the `struct` definition, but can be provided in any order.
Let's create an instance of our `Vector2`, as illustrated in Listing 3-3.

<!-- HACK: Add an extension to support hiding of Mun code -->
```rust,ignore 
{{#include ../listings/ch03-structs/listing03.mun}}
```

<span class="caption">Listing 3-3: Creating a `Vector2` instance</span>

To create an instance of a tuple `struct`, you only need to state the name of the `struct` and
specify a comma-separated list of values between round brackets - as shown in Listing 3-4. As
values are not linked to field names, they have to appear in the order specified by the `struct`
definition.

<!-- HACK: Add an extension to support hiding of Mun code -->
```rust,ignore 
{{#include ../listings/ch03-structs/listing04.mun}}
```

<span class="caption">Listing 3-4: Creating a `Vector3` instance</span>

#### Field Init Shorthand

It often makes sense to name function variables the same as the fields of a record `struct`.
Instead of having to repeat the `x` and `y` field names, the _field init shorthand syntax_
demonstrated in Listing 3-5 allows you to avoid repetition.

<!-- HACK: Add an extension to support hiding of Mun code -->
```rust,ignore 
{{#include ../listings/ch03-structs/listing05.mun}}
```

<span class="caption">Listing 3-5: Creating a `Vector2` instance using the _field init shorthand syntax_
</span>

### Access Struct Fields

To access a record's fields, we use the dot notation: `vector.x`. The dot notation can be used both
to retrieve and to assign a value to the record's field, as shown in Listing 3-6. As you can see,
the record's name is used to indicate that the function expects two `Vector2` instances as function
arguments and returns a `Vector2` instance as result.

<!-- HACK: Add an extension to support hiding of Mun code -->
```rust,ignore 
{{#include ../listings/ch03-structs/listing06.mun}}
```

<span class="caption">Listing 3-6: Using `Vector2` instances' fields to calculate their addition
</span>

A tuple `struct` doesn't have field names, but instead accesses fields using indices - starting
from zero - corresponding to a field's position within the struct definition (see Listing 3-7).

<!-- HACK: Add an extension to support hiding of Mun code -->
```rust,ignore 
{{#include ../listings/ch03-structs/listing07.mun}}
```

<span class="caption">Listing 3-7: Using `Vector3` instances' fields to calculate their addition
</span>

### Unit Struct

Sometimes it can be useful to define a `struct` without any fields. These so-called _unit structs_
are defined using the `struct` keyword and a name, as shown in Listing 3-8.

<!-- HACK: Add an extension to support hiding of Mun code -->
```rust,ignore 
{{#include ../listings/ch03-structs/listing08.mun}}
```

<span class="caption">Listing 3-8: A unit `struct` definition.</span>
