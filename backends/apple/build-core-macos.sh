#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
CORE_DIR="$SCRIPT_DIR/../core"
EXPECTED_RUST="rustc 1.93.0"
ACTUAL_RUST=$(rustc --version)

if [ "${ACTUAL_RUST#"$EXPECTED_RUST"}" = "$ACTUAL_RUST" ]; then
  echo "Rust toolchain mismatch: expected $EXPECTED_RUST, found $ACTUAL_RUST" >&2
  echo "Run: rustup toolchain install 1.93.0 --profile minimal" >&2
  exit 1
fi

cd "$SCRIPT_DIR/../.."
shasum -a 256 -c backends/core/checksums.sha256
cd "$CORE_DIR"
cargo build --package host-c-abi --locked --offline

LIBRARY="$CORE_DIR/target/debug/libhost_c_abi.a"
if [ ! -f "$LIBRARY" ]; then
  echo "Rust core packaging failed: missing $LIBRARY" >&2
  exit 1
fi

nm -gU "$LIBRARY" | rg -q '_float_view_abi_version$' || {
  echo "Rust core packaging failed: ABI identity symbol is missing" >&2
  exit 1
}
