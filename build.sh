#!/usr/bin/env bash
# Builds the static site in web/: the WASM hint engine, the published copies
# of the puzzle collections, and the moves-page examples.
set -euo pipefail
cd "$(dirname "$0")"

wasm-pack build crates/territories-wasm --release --target web --out-dir ../../web/pkg --no-typescript
rm -f web/pkg/.gitignore web/pkg/package.json   # keep pkg/ publishable as static files

cargo run --release -p territories-gen --bin farm -- --publish-only
# The two-animal game appears on the site once it has a collection.
if [ -f data/puzzles-2.json ]; then
  cargo run --release -p territories-gen --bin farm -- --stars 2 --publish-only
fi
cargo run --release -p territories-gen --bin help-examples > web/examples.json
cargo run --release -p territories-gen --bin help-examples -- --stars 2 > web/examples-2.json
