#!/bin/bash
# Run tests for urm37 crate on the host platform
cd "$(dirname "$0")" || exit 1
cargo test --target x86_64-unknown-linux-gnu "$@"
