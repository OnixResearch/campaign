#!/bin/sh
set -eu

sh scripts/check-config.sh
sh scripts/check-boundaries.sh
sh scripts/check-cairn.sh
sh scripts/check-rust-gates.sh

printf '%s\n' "campaign infrastructure checks passed"
