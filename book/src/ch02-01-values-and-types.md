## Values and types

Mun is a statically typed language, which helps to detect type-related errors at
compile-time. A type error is an invalid operation on a given type, such as an
integer divided by a string, trying to access a field that doesn't exist, or
calling a function with the wrong number of arguments.

Some languages require a programmer to explicitly annotate syntactic constructs
with type information:

```c++
int foo = 3 + 4;
```

However, often variable types can be inferred by their usage. Mun uses type
inferencing to determine variable types at compile time. However, you are still
forced to explicitly annotate variables in a few locations to ensure a contract
between interdependent code.

```mun
# pub fn main() {
#   bar(1);
# }
fn bar(a: i32) -> i32 {
    let foo = 3 + a;
    foo
}
```

Here, the parameter `a` and the return type must be annotated because it
solidifies the signature of the function. The type of `foo` can be inferred
through its usage.

> NOTE: Although the above works, as of version 0.2, Mun is not yet very good at
>type inferencing. This will be improved in the future.

### Integer types

An integer is a number without a fractional component. Table 3-1 shows the
built-in integer types in Mun. Each variant can be either signed or unsigned
and has an explicit size. *Signed* and *unsigned* refer to  whether it is
necessary to have a sign that indicates the possibility for the  number to be
negative or positive.


| Length   | Signed  | Unsigned |
|----------|---------|----------|
| 8-bit    | `i8`    | `u8`     |
| 16-bit   | `i16`   | `u16`    |
| 32-bit   | `i32`   | `u32`    |
| 64-bit   | `i64`   | `u64`    |
| 128-bit  | `i128`  | `u128`   |
| arch     | `isize` | `usize`  |

<span class="caption">Table 2-1: Integer Types in Mun</span>

Signed integer types start with `i`, unsigned integer types with `u`, followed 
by the number of bits that the integer value takes up. Each signed variant can 
store numbers from -(2<sup>n - 1</sup>) to 2<sup>n - 1</sup> - 1 inclusive, 
where *n* is the number of bits that variant uses. Unsigned variants can store 
numbers from 0 to 2<sup>n - 1</sup>. By default Mun uses 32-bit signed
integers.

The size of the `isize` and `usize` types depend on the target architecture. On
64-bit architectures,`isize` and `usize` types are 64 bits large, whereas on 32-bit
architectures they are 32 bits in size.

### Floating-Point Types

Real (or *floating-point*) numbers (i.e. numbers with a fractional component)
are represented according to the IEEE-754 standard. The `f32` type is a
single-precision float of 32 bits, and the `f64` type has double precision -
requiring 64 bits.

```mun
pub fn main() {
    let f = 3.0; // f64
}
```


### The Boolean Type

The `bool` (or *boolean*) type has two values, `true` and `false`, that are
used to  evaluate conditions. It takes up one 1 byte (or 8 bits).

```mun
pub fn main() {
    let t = true;

    let f: bool = false; // with explicit type annotation
}
```

### Literals

There are three types of literals in Mun: integer, floating-point and boolean
literals. 

A boolean literal is either `true` or `false`.

An integer literal is a number without a decimal separator (`.`). It can be
written as a decimal, hexadecimal, octal or binary value. These are all
examples of valid literals:

```mun
# pub fn main() {
let a = 367;
let b = 0xbeaf;
let c = 0o76532;
let d = 0b0101011;
# }
```

A floating-point literal comes in two forms:

* A decimal number followed by a dot which is optionally followed by another
  decimal literal and an *optional* exponent.
* A decimal number followed by an exponent.

Examples of valid floating-point literals are:

```mun
# pub fn main() {
let a: f64 = 3.1415;
let b: f64 = 3.;
let c: f64 = 314.1592654e-2;
# }
```

#### Separators

Both integer and floating-point literals can contain underscores (`_`) to
visually separate numbers from one another. They do not have any semantic
significance but can be useful to the eye.

```mun
# pub fn main() {
let a: i64 = 1_000_000;
let b: f64 = 1_000.12;
# }
```

#### Type suffix

Integer and floating-point literals may be followed by a type suffix to
explicitly specify the type of the literal.

| Literal type | Suffixes |
|--------------|----------|
|Integer |`u8`, `i8`, `u16`, `i16`, `u32`, `i32`, `u64`, `i64`, `i128`, `u128`, `usize`, `isize`, `f32`, `f64` |
| Floating-point | `f32`, `f64` |

<span class="caption">Table 2-2: Literal suffixes in Mun</span>

Note that integer literals can have floating-point suffixes. This is not the
case the other way around.

```mun
# pub fn main() {
let a: u8 = 128_u8;
let b: i128 = 99999999999999999_i128;
let c: f32 = 10_f32; // integer literal with float suffix 
# }
```

When providing a literal, the compiler will always check if a literal value will
fit the type. If not, an error will be emitted:

```mun,compile_fail
# pub fn main() {
let a: u8 = 1123123124124_u8; // literal out of range for `u8`
# }
```

### Numeric operations 

Mun supports all basic mathematical operations for number types: addition,
subtraction, division, multiplication, and remainder.

```mun
pub fn main() {
    // addition 
    let a = 10 + 5;

    // subtraction
    let b = 10 - 4;

    // multiplication
    let c = 5 * 10;

    // division
    let d = 25 / 5;

    // remainder
    let e = 21 % 5;
}
```

Each expression in these statements uses a mathematical operator and evaluates
to a single value. This is valid as long as both sides of the operator have the
same type.

Unary operators are also supported:

```mun
pub fn main() {
    let a = 4;
    // negate
    let b = -a;
    
    let c = true;
    // not
    let d = !c;
}
```

### Shadowing

Redeclaring a variable by the same name with a `let` statement is valid and will
shadow any previous declaration in the same block. This is often useful if you
want to change the type of a variable.

```mun
# pub fn main() {
let a: i32 = 3;
let a: f64 = 5.0; 
# }
```

### Use before initialization

All variables in Mun must be initialized before usage. Uninitialized variables
can be declared but they must be assigned a value before they can be read.

```mun,compile_fail
# pub fn main() {
# let some_conditional = false;
let a: i32;
if some_conditional {
    a = 4;
}
let b = a; // invalid: a is potentially uninitialized
# }
```

Note that declaring a variable without a value is often a bad code smell since
the above could have better been written by *returning* a value from the
`if`/`else` block instead of assigning to `a`. This avoids the use of an
uninitialized value.

```mun
# pub fn main() {
# let some_conditional = true;
let a: i32 = if some_conditional {
    4
} else {
    5
}
let b = a;
# }
```
