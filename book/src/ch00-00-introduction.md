# Introduction

> Note: Mun & this book are currently under active development, any and all
> content of this book is not final and may still change.

*Mun* is an embeddable scripting language designed for developer productivity. 

* **Ahead of time compilation**  
  Mun is compiled ahead of time (AOT), as opposed to being interpreted or
  compiled just in time (JIT). By detecting errors in the code during AOT
  compilation, an entire class of runtime errors is eliminated. This allows
  developers to stay within the comfort of their IDE instead of having to switch
  between the IDE and target application to debug runtime errors.

* **Statically typed**  
  Mun resolves types at compilation time instead of at runtime, resulting in
  immediate feedback when writing code and opening the door for powerful
  refactoring tools.

* **First class hot-reloading**  
  Every aspect of Mun is designed with hot reloading in mind. Hot reloading is
  the process of changing code and resources of a live application, removing the
  need to start, stop and recompile an application whenever a function or value
  is changed.

* **Performance**  
  AOT compilation combined with static typing ensure that Mun is compiled to
  machine code that can be natively executed on any target platform. LLVM is
  used for compilation and optimization, guaranteeing the best possible
  performance. Hot reloading does introduce a slight runtime overhead, but it
  can be disabled for production builds to ensure the best possible runtime
  performance.

* **Cross compilation**  
  The Mun compiler is able to compile to all supported target platforms from any
  supported compiler platform. 

* **Powerful IDE integration**  
  The Mun language and compiler framework are designed to support source code
  queries, allowing for powerful IDE integrations such as code completion and
  refactoring tools.
