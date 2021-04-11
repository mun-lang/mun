## `use` keyword

So far we've worked with a single source file when writing Mun code.
Let's look at an example of a multi-file project.
We'll rewrite the fibonacci example from Listing 1-1, by splitting it into two files `src/fibonacci.mun` and `src/mod.mun` - the former being a submodule of the latter - as shown in Listing 2-4 and Listing 2-5, respectively.

> Alternatively, you can also create a fibonacci submodule using the path `src/fibonacci/mod.mun`. Both are equally valid and it's up to you to decide what you prefer.

Filename: src/fibonacci.mun

```mun,no_run
{{#include ../listings/ch02-basic-concepts/listing04.mun}}
```

<span class="caption">Listing 2-4: The fibonacci function extracted into its own submodule</span>

To show Mun where to find an item in the *module tree*, we use a *path* in the same way we use a path when navigating a filesystem.
If we want to call a function, we need to know its path.

A path can take two forms:

- An *absolute path* starts from the package's root.
- A *relative path* starts from the current module and uses `self`, `super`, or an identifier in the current module.

Both absolute and relative paths are followed by one or more identifiers separated by double colons (`::`). It's up to you to decide which style is preferable.

Filename: src/mod.mun

```mun,ignore
{{#include ../listings/ch02-basic-concepts/listing05.mun}}
```

<span class="caption">Listing 2-5: The fibonacci function being called using its full path.</span>

Writing full paths can result in inconveniently long and repetitive code.
Fortunately, there is a way to simplify this process.
We can bring a path into a module's scope once and then call the items in that path as if they are local items with the `use` keyword.

In Listing 2-6, we bring the `fibonacci::fibonacci` function into the module's scope, allowing us to directly call the function..

Filename: src/mod.mun

```mun,ignore
{{#include ../listings/ch02-basic-concepts/listing06.mun}}
```

<span class="caption">Listing 2-6: Bringing the fibonacci function into the module's scope with the `use` keyword</span>

### The Glob Operator

To bring all public items defined in a path into scope, we can specify that path followed by the glob operator (`*`):

```mun,ignore
use fibonacci::*;
```

This brings all public items defined in the `fibonacci` module into the current scope.

> **Be careful when using the glob operator!**
>
> Glob can make it harder to tell what names are in scope and where a name used in your program was defined.
