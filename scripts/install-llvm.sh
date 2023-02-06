#!/bin/bash

# This script install the correct LLVM release for Debian & Ubuntu distros.

set -eux

LLVM_VERSION=14
DISTRO=$(lsb_release -is)
VERSION=$(lsb_release -sr)
DIST_VERSION="${DISTRO}_${VERSION}"

# Find the right repository name for the distro and version
case "$DIST_VERSION" in
    Debian_9* )       REPO_NAME="deb http://apt.llvm.org/stretch/  llvm-toolchain-stretch-$LLVM_VERSION main" ;;
    Debian_10* )      REPO_NAME="deb http://apt.llvm.org/buster/   llvm-toolchain-buster-$LLVM_VERSION  main" ;;
    Debian_unstable ) REPO_NAME="deb http://apt.llvm.org/unstable/ llvm-toolchain-$LLVM_VERSION         main" ;;
    Debian_testing )  REPO_NAME="deb http://apt.llvm.org/unstable/ llvm-toolchain-$LLVM_VERSION         main" ;;
    Ubuntu_16.04 )    REPO_NAME="deb http://apt.llvm.org/xenial/   llvm-toolchain-xenial-$LLVM_VERSION  main" ;;
    Ubuntu_18.04 )    REPO_NAME="deb http://apt.llvm.org/bionic/   llvm-toolchain-bionic-$LLVM_VERSION  main" ;;
    Ubuntu_18.10 )    REPO_NAME="deb http://apt.llvm.org/cosmic/   llvm-toolchain-cosmic-$LLVM_VERSION  main" ;;
    Ubuntu_19.04 )    REPO_NAME="deb http://apt.llvm.org/disco/    llvm-toolchain-disco-$LLVM_VERSION   main" ;;
    Ubuntu_19.10 )   REPO_NAME="deb http://apt.llvm.org/eoan/      llvm-toolchain-eoan-$LLVM_VERSION    main" ;;
    Ubuntu_20.04 )   REPO_NAME="deb http://apt.llvm.org/focal/     llvm-toolchain-focal-$LLVM_VERSION   main" ;;
    Ubuntu_22.04 )   REPO_NAME="deb http://apt.llvm.org/jammy/     llvm-toolchain-jammy-$LLVM_VERSION   main" ;;
    Ubuntu_22.10 )   REPO_NAME="deb http://apt.llvm.org/kinetic/   llvm-toolchain-kinetic-$LLVM_VERSION main" ;;
    * )
        echo "Distribution '$DISTRO' in version '$VERSION' is not supported by this script (${DIST_VERSION})."
        exit 2
esac

if [[ ! -f /etc/apt/trusted.gpg.d/apt.llvm.org.asc ]]; then
    # download GPG key once
    wget -qO- https://apt.llvm.org/llvm-snapshot.gpg.key | tee /etc/apt/trusted.gpg.d/apt.llvm.org.asc
fi

if [[ -z "`apt-key list 2> /dev/null | grep -i llvm`" ]]; then
    # Delete the key in the old format
    apt-key del AF4F7421
fi

# Add the right repository for the distro
add-apt-repository "${REPO_NAME}"
apt-get update

# Install required packages
apt-get install -y llvm-$LLVM_VERSION llvm-$LLVM_VERSION-* liblld-$LLVM_VERSION* libclang-common-$LLVM_VERSION-dev libpolly-$LLVM_VERSION-dev
