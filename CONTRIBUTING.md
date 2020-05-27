# Contributing to Mun

Ah! Good to see you are reading this. We feel that developing a programming
language without the input of our target audience is counterproductive.
Therefore we gladly welcome contributions from all developers! 

If you haven't already, [join our discord server](https://discord.gg/SfvvcCU).
We can help you get underway with the things you're most excited about.

Some important resources to get started are:

- [Our roadmap](https://trello.com/b/ZcMiREnC/mun-roadmap) gives a very broad
  overview of where we're heading.
- We're usually online on [our discord server](https://discord.gg/SfvvcCU)
  during business hours (UTC+1).
- Bugs? [Report them on GitHub](https://github.com/mun-lang/mun/issues)
- What is our vision on [testing](#testing)?
  - Try to use [Test Driven Development](#test-driven-development)
  - About [snapshot tests](#snapshot-tests)
- Our [Git workflow](#git-workflow)
  1. [Fork](#fork-the-repository) in the repository
  2. Locally [clone](#locally-clone-the-fork) the fork
  3. Create a [branch](#create-a-branch)
  4. [Synchronise](#synchronise-your-branch) your branch
  5. [Commit](#commit-changes) changes
  6. [Push](#push-changes) changes
  7. Create a [Pull Request](#create-a-pull-request)
  8. [Merging](#merging-a-pull-request) a Pull Request

## Testing

We feel that having an extensive [regression
testing](https://en.wikipedia.org/wiki/Regression_testing) suite allows
developers to add new features or change existing features with more confidence;
knowing that changes behave the way you expect them to and without unwittingly
impacting other features.

### Test Driven Development

We try to implement new features using [Test Driven Development
(TTD)](https://en.wikipedia.org/wiki/Test-driven_development). In practice this
means that we write tests - based on requirements - before implementing new
features to ensure that they work. This seamlessly integrates with regression
testing, as there is no extra workload.

### Snapshot Tests

A snapshot test asserts that the output of an operation doesn't change. As such,
we use it to verify that a feature generates (and keeps generating) the correct
output. The [insta crate](https://crates.io/crates/insta) is used throughout the
codebase to enable this functionality. See
[crates/mun_hir/src/ty/tests.rs](crates/mun_hir/src/ty/tests.rs) for an example.

## Git Workflow

We follow a Git workflow similar to
[Kubernetes](https://github.com/kubernetes/community/blob/master/contributors/guide/git_workflow.png).
If you are not familiar with it, please review the following instructions.

### Fork the Repository

1. Visit https://github.com/mun-lang/mun
2. Click the `Fork` button (top right) to establish a cloud-based fork.

You should now have a fork of the Mun repo at `https://github.com/$user/mun`,
where `$user` is your GitHub handle.

### Locally Clone the Fork

To create a local clone of your fork at the `$working_dir` folder, execute the
following Git commands:

```bash
cd $working_dir
git clone https://github.com/$user/mun.git 
# or: git clone git@github.com:$user/mun.git

cd $working_dir/mun
git remote add upstream https://github.com/mun-lang/mun.git
# or: git remote add upstream git@github.com:mun-lang/mun.git

# Confirm that your remotes make sense:
git remote -v
```

### Create a Branch

Update your local `master` branch:

```bash
cd $working_dir/mun
git fetch upstream
git checkout master
git rebase upstream/master
```

Branch from `master`:

```bash
git checkout -b feature/my-precious
```

Now you are ready to edit code on the `feature/my-precious` branch.

### Synchronise your Branch

```bash
# While on your feature/my-precious branch
git fetch upstream
git rebase upstream/master
```

Please don't use `git pull` instead of the above `fetch` / `rebase`. By default,
`pull` uses a built-in list of strategies that result in merge commits. These
make the commit history messy and violate the principle that commits ought to be
individually understandable and useful. You can also consider changing your
`.git/config` file via `git config branch.autoSetupRebase always` to change the
behaviour of `git pull` to always use [`--rebase`
merging](https://www.atlassian.com/git/tutorials/rewriting-history/git-rebase).

### Commit changes

Commit your changes.

```bash
git commit
```

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

### Push Changes

When ready to review (or to create an off-site backup of your work), push your
branch to your fork:

```bash
git push $your_remote_name feature/my-precious
```

If history was rewritten as a result of a rebase merge, you'll need to [force
push](https://blog.developer.atlassian.com/force-with-lease/) changes.

### Create a Pull Request

Please submit a [GitHub Pull Request to
mun-lang/mun](https://github.com/mun-lang/mun/pull/new/master) with a clear list
of changes (read more about [pull
requests](http://help.github.com/pull-requests/)). When you submit a pull request,
make sure to include tests that validate the implemented feature or bugfix
([read about testing in Mun](testing)). Before committing, please confirm that your code
style is correct (using `cargo fmt`) and all lint warning have been resolved
(using `cargo clippy`). We integrated [cargo-husky](https://github.com/rhysd/cargo-husky)
as a pre-commit hook, to make this process as simple as possible.

Please consider that very small PRs are easy to review, whereas very large PRs
are very difficult to review. The more focused your PR, the shorter the timeline
for approval.

#### Get a Code Review

Once your pull request has been opened, it will be assigned to one or more
reviewers. Those reviewers will do a thorough code review, looking for
correctness, bugs, opportunities for improvement, documentation and comments,
and style.

Please commit changes made in response to review comments to the same branch on
your fork. Feel free to use GitHub's suggestions, but you'll likely have to
clean up your history afterwards.

#### Cleaning up Commit History

After a review, prepare your PR for merging by cleaning up the commit history.
All commits left on your branch after a review should represent meaningful
milestones or units of work. Use commits to add clarity to the development and
review process.

Before merging a PR, squash the following kinds of commits:

- Fixes/review feedback
- Typos
- Merges and rebases
- Work in progress

Aim to have every commit in a PR compile and pass tests independently if you
can, but it's not a requirement. In particular, merge commits must be removed!

To edit or squash your commits, perform an [interactive
rebase](https://git-scm.com/book/en/v2/Git-Tools-Rewriting-History). Start an
interactive rebase using a specific commit hash, or count backwards from your
last commit using `HEAD~<n>`, where `<n>` represents the number of commits to
include in the rebase.

```bash
git rebase -i HEAD~3
```

The output looks similar to:

```bash
pick 2ebe926 feat(memory): add mark-region garbage collector
pick 31f33e9 misc: apply review suggestions
pick b0315fe test(memory): add unit test for mark-region gc

# Rebase 7c34fc9..b0315ff onto 7c34fc9 (3 commands)
#
# Commands:
# p, pick <commit> = use commit
# r, reword <commit> = use commit, but edit the commit message
# e, edit <commit> = use commit, but stop for amending
# s, squash <commit> = use commit, but meld into previous commit
# f, fixup <commit> = like "squash", but discard this commit's log message

...
```

Use a command-line text editor to change the word `pick` to the appropriate
command, e.g. `fixup` for commits that you want to squash:

```bash
pick 2ebe926 feat(memory): add mark-region garbage collector
fixup 31f33e9 misc: apply review suggestions
pick b0315fe test(memory): add unit test for mark-region gc

...
```

Upon a successful rebase, push your changes to the remote branch:

```bash
git push --force-with-lease
```

### Merging a Pull Request

Once your pull request has been reviewed and approved, your PR is ready for
merging. Merging will automatically be taken care of by the Reviewer.
