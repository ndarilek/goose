#!/usr/bin/env bash
# Build + run the mobile-lifecycle Swift harness (suspend/resume cycles) against
# a goose iroh ACP server.
#
#   1. cargo build -p goose-cli --bin goose -p goose-tunnel-ffi
#   2. ./target/debug/goose serve --iroh    # copy the printed connection token
#   3. crates/goose-tunnel-ffi/swift-driver/run-mobile.sh "<connection-token>"
set -euo pipefail

TOKEN="${1:?usage: run-mobile.sh <connection-token>}"
REPO_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
LIBDIR="$REPO_ROOT/target/debug"
DRIVER_DIR="$(cd "$(dirname "$0")" && pwd)"
BUILD_DIR="$(mktemp -d)"

cargo build -p goose-tunnel-ffi --manifest-path "$REPO_ROOT/Cargo.toml" >/dev/null

cargo run -p goose-tunnel-ffi --bin uniffi-bindgen --manifest-path "$REPO_ROOT/Cargo.toml" -- \
  generate --library "$LIBDIR/libgoose_tunnel_ffi.dylib" --language swift --out-dir "$BUILD_DIR" >/dev/null 2>&1

echo 'module goose_tunnel_ffiFFI { header "goose_tunnel_ffiFFI.h" export * }' > "$BUILD_DIR/module.modulemap"

swiftc -parse-as-library -o "$BUILD_DIR/mobile" \
  "$DRIVER_DIR/mobile-lifecycle.swift" "$BUILD_DIR/goose_tunnel_ffi.swift" \
  -I "$BUILD_DIR" \
  -L "$LIBDIR" -lgoose_tunnel_ffi \
  -Xcc -fmodule-map-file="$BUILD_DIR/module.modulemap"

DYLD_LIBRARY_PATH="$LIBDIR" "$BUILD_DIR/mobile" "$TOKEN"
