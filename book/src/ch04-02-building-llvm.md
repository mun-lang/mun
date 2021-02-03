# Building LLVM

Most, if not all, dependencies can be build by cargo except for LLVM.
The Mun compiler makes heavy use of LLVM for all code-generation capabilities.
Installing it, however, can be tricky.
This document is a short guide on how to install LLVM on your machine so you can build Mun yourself.

Currently, Mun targets LLVM 8 so everything in this document refers to that version.
However, these instructions should also hold for newer versions. 

## Prebuild binaries

On some OSes prebuild binaries are available.

> NOTE: not all prebuild releases contain all libraries required by Mun. 
> For instance, prebuild releases of LLVM for Windows are missing required executables and libraries.

### Windows

For Windows, [we maintain a repository](https://github.com/mun-lang/llvm-package-windows) which contains [releases](https://github.com/mun-lang/llvm-package-windows/releases) that can can be used to build Mun. 
These releases are also used on our CI runners. 

To use a release, download and extract it to your machine. 
To make sure the build pipeline can find the binaries, add an environment variable called `LLVM_SYS_80_PREFIX` that points to the folder where you extracted the release.
It is also possible to add the `bin` folder of the release to your path but using the environment variables allows you to have multiple LLVM releases on your machine.

> For LLVM 8 you should add the `LLVM_SYS_80_PREFIX` environment variable, for LLVM 11 add `LLVM_SYS_110_PREFIX`.

### Debian & Ubuntu

LLVM provides APT repositories for several versions of LLVM which contain all the required binaries required to build Mun. 
Visit the [LLVM APT website](https://apt.llvm.org/) to find the correct APT repository to use. 
To add the repository:

```bash
# Retrieve the archive signature
wget -O - https://apt.llvm.org/llvm-snapshot.gpg.key | apt-key add -

# Add the repository
# ${REPO_NAME} should be something like:
# deb http://apt.llvm.org/focal/ llvm-toolchain-focal-8 main
#
# The `add-apt-repository` command is installed by the `software-properties-common` package:
# sudo apt install software-properties-common 
add-apt-repository "${REPO_NAME}"
```

Once you have the proper APT repository configured you can install the required LLVM binaries with:

```bash
apt install llvm-8 llvm-8-* liblld-8*
```

### MacOS

[Brew](https://brew.sh/) contains a cask for LLVM that can be used to build Mun:

```bash
brew install llvm@8
```

After installing LLVM, you can either add the `bin` folder of the release to your path; or you can add a release-specific environment variable called `LLVM_SYS_80_PREFIX` that points to the release:

```bash
export LLVM_SYS_80_PREFIX=$(brew --prefix llvm@8)
```

Adding the `LLVM_SYS_80_PREFIX` variable is usually easier because the LLVM binaries will not conflict with any preinstalled version of LLVM and it allows you to easily install another version of LLVM side-by-side.

> For LLVM 8 you should add the `LLVM_SYS_80_PREFIX` environment variable, for LLVM 11 add `LLVM_SYS_110_PREFIX`.

## Building from source

If there are no prebuild packages available for your OS, your best bet is to install LLVM from source. 
The build time of LLVM is quite long so this is a relatively time-consuming process.

You need at least: 
- A C++ compiler (like GCC)
- [CMake](https://cmake.org/)
- [Python](https://www.python.org/)

Download a dump of the LLVM repository from the [LLVM github repository](https://github.com/llvm/llvm-project) and extract it somewhere, e.g.:

```bash
wget -qO- \
  https://github.com/llvm/llvm-project/archive/llvmorg-8.0.1.tar.gz | \
  tar xzf -
```

Then build the required components and install them to `~/local`.

```bash
cd llvm-project-llvmorg-8.0.1/llvm
mkdir build
cd build
cmake .. -DCMAKE_BUILD_TYPE=Release -DLLVM_ENABLE_PROJECTS="lld;clang" -DCMAKE_INSTALL_PREFIX=$HOME/local -DCMAKE_INSTALL_PREFIX=$HOME/local -DLLVM_ENABLE_LIBXML2=OFF
make install -j
```

After LLVM is build, make sure to add the `$HOME/local/bin` to you path or add an environment variable `LLVM_SYS_80_PREFIX` (or `LLVM_SYS_110_PREFIX` depending on the LLVM version you installed) that points to `$HOME/local`. 
