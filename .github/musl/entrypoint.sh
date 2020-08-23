#!/bin/bash
set -e -u -o pipefail
echo In musl builder
cp --recursive /root/.cargo $HOME
cp --recursive /root/.rustup $HOME
echo $*
bash -c "$*"
