#!/bin/bash
set -e -u -o pipefail
echo In musl builder
echo $*
bash -c "$*"
