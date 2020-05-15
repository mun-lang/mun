## Case Studies

A collection of case studies that inspired the design choices made in Mun.

### Abbey Games

Abbey Games uses Lua as its main gameplay programming language because of Lua's
ability to hot reload code. This allows for rapid iteration of game code,
enabling gameplay programmers and designers to quickly test and tweak systems
and content. Lua is a dynamically typed, JIT compiled language. Although this
has some definite advantages, it also introduces a lot of problems with bigger
codebases.

Changes in Lua code can have large implications throughout the entire codebase
and since we cannot oversee the entire codebase at all times runtime errors are
bound to occur. Runtime errors are nasty beasts because they can pop up after a
long period of time and after work on the offending piece of code has already
finished. They are also often detected by someone different from the person who
worked on the code. This causes great frustration and delay, let alone when the
runtime error is detected by a user of the software.

Lua amplifies this issue due to its dynamic and flexible nature. *It would be
great if we could turn some of these runtime errors into compile time errors.*
That way programmers are notified of errors way before someone else runs into
them. The risk of causing implicit runtime errors causes programmers to distrust
their refactoring tools. This in turn reduces the likelihood of programmers
refactoring their code.

Even though Lua offers immense flexibility, we noticed that certain opinionated
patterns recur a lot and as such have become standard practice. Introducing
these practices assists us in daily development a lot, but requires more code
and complexity than desirable. Having syntactic sugar would greatly help reduce
complexity in our code base, but would also introduce *magic* or custom keywords
that are foreign to both new developers and IDE's. 

Rapid iteration is key to prototyping game concepts and features. *Proper
IDE-integration of a scripting language gives a huge boost to productivity.*
