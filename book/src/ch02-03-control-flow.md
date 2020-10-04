## Control flow

Executing or repeating a block of code only under specific conditions are common
constructs that allow developers to control the flow of execution. Mun provides
 `if`/`else` expressions and loops.

### `if` expressions

An `if` expression allows you to branch your code depending on conditions.

```mun
pub fn main() {
    let number = 3;

    if number < 5 {
        number = 4;
    } else {
        number = 6;
    }
}
```

All `if` expressions start with the keyword `if`, followed by a condition. As
opposed to many C-like languages, Mun omits parentheses around the condition.
Only when the condition is true - in the example, whether the `number` variable
is less than 5 - the consecutive code block (or *arm*) is executed.

Optionally, an `else` expression can be added that will be executed when the
condition evaluates to false. You can also have multiple conditions by combining
`if` and `else` in an `else if` expression. For example:

```mun
pub fn main() {
    let number = 6;
    if number > 10 {
        // The number if larger than 10
    } else if number > 8 {
        // The number is larger than 8 but smaller or equal to 10
    } else if number > 2 {
        // The number is larger than 2 but smaller or equal to 8
    } else {
        // The number is smaller than- or equal to 2.
    }
}
```


#### Using `if` in a `let` statement

The `if` expression can be used on the right side of a `let` statement
just like a block:

```mun
pub fn main() {
    let condition = true;
    let number = if condition {
        5
    } else {
        6
    };
}
```

Depending on the condition, the `number` variable will be bound to the value of
the `if` block or the `else` block. This means that both the `if` and `else`
arms need to evaluate to the same type. If the types are mismatched the compiler
will report an error.


### `loop` expressions

A `loop` expression can be used to create an infinite loop. Breaking out of the
loop is done using the `break` statement.

```mun
pub fn main() {
    let i = 0;
    loop {
        if i > 5 {
            break;
        }

        i += 1;
    }
}
```

Similar to `if`/`else` expressions, `loop` blocks can have a return value that
can be returned through the use of a `break` statement.

```mun
# pub fn main() {
#   count(4, 4);
# }
fn count(i: i32, n: i32) -> i32 {
    let loop_count = 0;
    loop {
        if i >= n {
            break loop_count;
        }

        loop_count += 1;
    }
}
```

All `break` statements in a `loop` must have the same return type.

```mun,compile_fail
# pub fn main() {
let a = loop {
    break 3;
    break; // expected `{integer}`, found `nothing`
};
# }
```


### `while` expressions

`while` loops execute a block of code as long as a condition holds. A `while`
loop starts with the keyword `while` followed by a condition expression and a
block of code to execute upon each iteration. Just like with the `if`
expression, no parentheses are required around the condition expression.

```mun
pub fn main() {
    let i = 0;
    while i <= 5 {
        i += 1;
    }
}
```

A `break` statement inside the `while` loop immediately exits the loop.

Unlike a `loop` expression, a `break` in a while loop cannot return a value
because a while loop can exit both through the use of a `break` statement and
because the condition no longer holds. Although we could explicitly return a
value from the `while` loop through the use of a `break` statement it is unclear
which value should be returned if the loop exits because the condition no longer
holds.
