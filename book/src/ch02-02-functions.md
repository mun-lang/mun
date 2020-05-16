## Functions

Together with `struct`, functions are the core building blocks of hot reloading
in Mun. Throughout the documentation you've already seen a lot of examples of
the `fn` keyword, which is used to define a function.

Mun uses *snake case* as the conventional style for function and variable names.
In snake case all letters are lowercase and words are separated by underscores. 

```mun
fn main() {
    another_function();
}

fn another_function() {

}
```

Function definitions start with an optional access modifier (`pub`), followed
by the `fn` keyword, a name, an argument list enclosed by parentheses, an
optional return type specifier, and finally a body. 

Marking a function with the `pub` keyword allows you to publicly expose
that function, for usage in other modules or when hot reloading. Otherwise
the function will only be accessible from the current source file.

### Function Access Modifier

Marking a function with the `pub` keyword allows you to use it from outside of
the module it is defined in.

```mun
// This function is not accessible outside of this code
fn foo() {
    // ...
}
// This function is accessible from anywhere.
pub fn bar() {
    // Because `bar` and `foo` are in the same file, this call is valid.
    foo()
}
```

When you want to interface from your host language (C++, Rust, etc.) with Mun,
you can only access `pub` functions. These functions are hot reloaded by the
runtime when they **or** functions they call have been modified.

### Function Arguments

Functions can have an argument list. Arguments are special variables that are
part of the function signature. Unlike regular variables you have to explicitly
specify the type of the arguments. This is a deliberate decision, as type
annotations in function definitions usually mean that the compiler can derive
types almost everywhere in your code. It also ensures that you as a developer
define a *contract* of what your function can accept as its input.

The following is a rewritten version of `another_function` that shows what an
argument looks like:

```mun
fn main() {
    another_function(3);
}

fn another_function(x: i32) {
}
```

The declaration of `another_function` specifies an argument `x` of the `i32`
type. When you want a function to use multiple arguments, separate them with
commas:

```mun
fn main() {
    another_function(3, 4);
}

fn another_function(x: i32, y: i32) {
}
```

### Function Bodies

Function bodies are made up of a sequence of statements and expressions.
*Statements* are instructions that perform some action and do not return any
value. *Expressions* evaluate to a result value. 

Creating a variable and assigning a value to it with the `let` keyword is a
statement. In the following example, `let y = 6;` is a statement.

```mun
fn main() {
    let y = 6;
}
```

Statements do not return values and can therefore not be assigned to another
variable. 

Expressions do evaluate to something. Consider a simple math operation `5 + 6`,
which is an expression that evaluates to `11`. Expressions can be part of a
statement, as can be seen in the example above. The expression `6` is assigned
to the variable `y`. Calling a function is also an expression.

The body of a function is just a block. In Mun, not just bodies, but all blocks
evaluate to the last expression in them. Blocks can therefore also be used on
the right hand side of a `let` statement.

```mun
fn foo() -> i32 {
    let bar = {
        let b = 3;
        b + 3
    };
    // `bar` has a value 6
    bar + 3
}
```

### Returning Values from Functions

Functions can return values to the code that calls them. We don't name return
values in the function declaration, but we do declare their type after an arrow
(`->`). In Mun, a function implicitly returns the value of the last expression
in the function body. You can however return early from a function by using the
`return` keyword and specifying a value. 

```mun
fn five() -> i32 {
    5
}

fn main() {
    let x = five();
}
```

There are no function calls or statements in the body of the `five` function,
just the expression `5`. This is perfectly valid Mun. Note that the return type
is specified too, as `-> i32`. 


Whereas the last expression in a block implicitly becomes that blocks return
value, explicit `return` statements always return from the entire function:

```mun
fn foo() -> i32 {
    let bar = {
        let b = 3;
        return b + 3;
    };

    // This code will never be executed
    return bar + 3;
}
```
