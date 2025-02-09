#!/bin/bash

# This script overrides the libpolly static library to fix a static linking issue with LLVM 14

set -eux

apt download libpolly-14-dev && dpkg --force-all -i libpolly-14-dev* && rm libpolly-14-dev*
