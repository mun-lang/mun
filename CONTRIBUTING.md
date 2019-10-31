# Contributing to Mun

Ah! Good to see you are reading this. We feel that developing a programming
language without the input of our target audience is counterproductive. Therefore
we gladly welcome contributions from all developers! 

If you haven't already, [join our discord server](https://discord.gg/SfvvcCU). We can help you get underway with the things you're most excited about.

Some important resources to get started are:

- [Our roadmap](https://trello.com/b/ZcMiREnC/mun-roadmap) gives a very broad
  overview of where we're heading.
- We're usually online on [our discord server](https://discord.gg/SfvvcCU)
  during business hours (UTC+1).
- Bugs? [Report them on GitHub](https://github.com/mun-lang/mun/issues)
- What is our vision on [testing](#testing)?
  - Try to use [Test Driven Development](#test-driven-development)
  - About [snapshot tests](#snapshot-tests)
- How to [submit changes](#submitting-changes)?
  - Writing [commit messages](#commit-message-format)

## Testing

We feel that having an extensive [regression
testing](https://en.wikipedia.org/wiki/Regression_testing) suite allows
developers to add new features or change existing features with more confidence;
knowing that changes behave the way you expect them to and without unwittingly
impacting other features.

### Test Driven Development

We try to implement new features using [Test Driven Development
(TTD)](https://en.wikipedia.org/wiki/Test-driven_development). In practice this
means that we write tests - based on requirements - before implementing new features to ensure
that they work. This seamlessly integrates with regression testing, as
there is no extra workload.

### Snapshot tests

A snapshot test asserts that the output of an operation doesn't change. As such, we use it to verify that a feature generates (and keeps
generating) the correct output. The [insta crate](https://crates.io/crates/insta) is
used throughout the codebase to enable this functionality. See
[crates/mun_hir/src/ty/tests.rs](crates/mun_hir/src/ty/tests.rs) for an example.

## Submitting changes

Please submit a [GitHub Pull Request to
mun-lang/mun](https://github.com/mun-lang/mun/pull/new/master) with a clear list
of changes (read more about [pull
requests](http://help.github.com/pull-requests/)). When you submit a pull request,
make sure to include tests that validate the implemented feature or bugfix
([read about testing in Mun](testing)). Before committing, please confirm that your code
style is correct (using `cargo fmt`) and all lint warning have been resolved
(using `cargo clippy`). We integrated [cargo-husky](https://github.com/rhysd/cargo-husky)
as a pre-commit hook, to make this process as simple as possible.

### Commit message format

Always write a clear log message for your commits. We use the [Conventional
Commits](https://www.conventionalcommits.org/) format, which states that a commit
message should be structured as follows:

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

BREAKING CHANGE: a commit that has a footer BREAKING CHANGE:, or appends a !
after the type/scope, introduces a breaking API change (correlating with MAJOR
in semantic versioning). A BREAKING CHANGE can be part of commits of any type.

Recommended *type*s are: `feat`, `fix`, `ci`, `docs`, `style`, `refactor`,
`perf`, `test`, `revert` or `improvement`. 

One-line messages are fine for small changes, but bigger changes should include
a body.

#### Examples

```
feat: allow provided config object to extend other configs
```
```
refactor!: drop support for Node 6
```
```
feat(lang): add polish language
```
```
fix: correct minor typos in code

see the issue for details

on typos fixed.

Reviewed-by: Z
Refs #133
```

For more examples, check [recent commit
message](https://github.com/mun-lang/mun/commits/master).
