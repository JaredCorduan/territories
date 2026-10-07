#!/usr/bin/env bash
# Builds the static site in web/: the WASM hint engine, the published copy
# of the puzzle collection, and the moves-page examples.
set -euo pipefail
cd "$(dirname "$0")"

wasm-pack build crates/territories-wasm --release --target web --out-dir ../../web/pkg --no-typescript
rm -f web/pkg/.gitignore web/pkg/package.json   # keep pkg/ publishable as static files

cargo run --release -p territories-gen --bin farm -- --publish-only
cargo run --release -p territories-gen --bin help-examples > web/examples.json
