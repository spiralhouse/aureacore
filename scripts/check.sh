#!/bin/bash
set -e

echo "Running cargo fmt check..."
cargo fmt --all -- --check

echo "Running clippy..."
cargo clippy --all-targets --all-features -- -D warnings

echo "Running tests..."
cargo test --all-features

echo "All checks passed! ✨"