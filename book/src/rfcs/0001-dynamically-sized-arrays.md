- Feature name: dynamically_sized_arrays
- Start date: 2021-04-23
- RFC PR: [https://github.com/mun-lang/mun/pull/324](https://github.com/mun-lang/mun/pull/324)

# Summary

This is an RFC to introduce the concept of dynamically sized arrays in Mun.
These are arrays where the length of the array is not yet known at compile time.
This is different from statically sized arrays, where the size of the array is known at compile time.

# Motivation

Reasons for having dynamically allocated arrays.

* Having only statically sized arrays limits the use of Mun to only use types which have a known size. 
* Dynamically sized arrays pose more flexibility than statically sized arrays. 
    See C# and Swift as an example where statically sized arrays are rarely used.
* Tuples can already be used for statically sized arrays (although not very ergonomically).

    ```rust,ignore
    struct(value) ArrayOfFiveFloats(f32,f32,f32,f32,f32)
    ```
* Dynamically sized arrays are easily understandable from a user perspective

# Detailed design

This RFC proposes to add the language construct of a dynamically sized array as well as several additions to the language which are required as supporting features.

The type of a dynamically sized arrays is introduced as a new language construct indicated as `[T]`. This is similar to [Rusts array syntax](https://doc.rust-lang.org/std/primitive.array.html) as well as [Swifts shortened array form](https://developer.apple.com/documentation/swift/array).

```rust,ignore
let an_array: [f32] = construct_array()
```

```rust,ignore
fn construct_array() -> [f32] {
    // ... code
}
```

Arrays are reference types and can contain both reference and values types.

```rust,ignore
let x: [Foo] = // ...
let y: [f32] = // ...
```

## Constructing arrays

Arrays can be constructed using *array literals*: a comma-separated list of values. 
Without any other information, Mun creates an array that includes the specified values, automatically inferring the array's element type. For example:

```rust,ignore
// An array of integers.
let odd_numbers = [1,3,5,7,9,11,13,15]
```

Since Mun doesn't allow uninitialized values, arrays cannot be preallocated with default values and then initialized in a second operation.
To accommodate for this common behavior arrays can be dynamically resized.

```rust,ignore
let i = 0;
let array: [i32] = []
while i < count {
    array.push(i);
    i += 1;
}
```

This behavior is equivalent in Swift and is similar to a `Vec<T>` in a Rust.

Every array reserves a specific amount of memory to hold its contents. 
When you add elements to an array and that array begins to exceed its reserved capacity, the array allocates a larger region of memory and copies its elements into the new storage.

> TODO: In the future it would be nice if you can create an array with an initially allocated size.
> This will reduce the number of reallocations required when constructing a large array. 
> To copy Swift:
> 
> ```rust,ignore
> let array = [i32]::with_capacity(some_initial_size);
> ```

> TODO: In the future it would be nice if you can create an array by replicating a certain expression.
> ```rust,ignore
> // constructs an array of `count` elements all initialized to 0.0
> let array = [0.0; count] 
>
> // constructs an array of `count` elements all initialized to `Foo {}`
> let array = [value; count]
>
> // constructs an array of `count` elements all initialized to `value`
> let array = [value; count]
> ```

## Accessing Array Values

Array's can be indexed with the index operator. Array indexes are `0` based. 

```rust,ignore
let an_array: [f32] = construct_array()
let first_element = an_array[0]
an_array[1] = 5.0
```

# Features to be implemented

This is a high-level list of required features to be implemented to support arrays in Mun.

## ABI support for array types
    
Similar to structs, arrays are complex types that reference another type. 
We can probably implement this by adding something along the lines of:

```rust,ignore
enum TypeInfoData {
    // ...
    Array(element: TypeInfo const*)
}
```

## Syntax support for arrays

* Parsing of array types: `[T]`.
* Parsing of array literals: `[1,2,3]`
* Indexing expressions: `a[x]`

## HIR support for arrays

* Add a `Ty` for arrays.
* Support for parsing array literals.
* Support for array element inferencing. e.g
    ```rust,ignore
    let a = [1,2,3,4]
    foo(a[12])
    // what is the type of a? [i32]? [i8]? [usize]?
    ```

## Code generation support for arrays

* Construction operations. Probably requires a new intrinsic.
* Indexing operations
* Path-expression indexing operations

## Garbage collection support

The garbage collector has to be able construct and traverse array elements.

## Runtime support

Similar to `StructRef` and `RootedStruct` we will need `ArrayRef` and `RootedArray`.

Optionally, it would be nice if you could create an empty array from Rust or C#. This will require implementing `TypeInfo` and `TypeRef` to be able to construct an array of a certain *type*.

# Open questions

* How can we implement the `push`, `len`, etc operation? Does this require `impl`?
* The `len` and `push` functions have to be implemented in Rust. How can we expose this information. Hard-code this in HIR?
* Can we already handle the generics in arrays? In `[T]`, `T` is a generic.
* How do we handle out of bounds indexing? We have no support for panics yet. Should we implement that first?



