#!/bin/sh

#: Runs formatting, compilation, linting, and tests.

set -e

echo "rustc $(rustc --version) at $(which rustc), cargo $(cargo --version) at $(which cargo)"

./format.sh --check
RUSTFLAGS="-D warnings" cargo check --all-features
cargo clippy -- -D warnings
cargo test
