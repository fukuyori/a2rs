#!/usr/bin/env bash
set -euo pipefail

PROFILE=${1:-release}
BIN=target/${PROFILE}/a2rs

printf "[1/3] building (%s)\n" "$PROFILE"
cargo build --profile "$PROFILE" --features full

printf "[2/3] show version\n"
"$BIN" --version

printf "[3/3] regression checklist\n"
echo "- Boot DOS 3.3"
echo "- Boot ProDOS"
echo "- Boot Lode Runner"
echo "- Verify floating bus dependent titles"
echo "- Verify MAX -> x1 timing remains near 1.023 MHz"
