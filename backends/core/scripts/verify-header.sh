#!/bin/sh
set -eu

EXPECTED_CBINDGEN="0.29.2"
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
CORE_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
REPOSITORY_ROOT=$(CDPATH= cd -- "$CORE_DIR/../.." && pwd)

actual=$(cbindgen --version 2>/dev/null | awk '{ print $2 }')
if [ "$actual" != "$EXPECTED_CBINDGEN" ]; then
    echo "cbindgen mismatch: expected $EXPECTED_CBINDGEN, found ${actual:-missing}" >&2
    echo "Run: cargo install --locked cbindgen --version $EXPECTED_CBINDGEN" >&2
    exit 1
fi

cargo metadata --manifest-path "$CORE_DIR/Cargo.toml" --locked --offline \
    --no-deps --format-version 1 >/dev/null

temporary=$(mktemp -d "${TMPDIR:-/tmp}/float-view-core-header-check.XXXXXX")
trap 'rm -rf "$temporary"' EXIT HUP INT TERM

cd "$CORE_DIR"
cbindgen --config cbindgen.toml --crate host-c-abi \
    --output "$temporary/float_view_core.h"

if ! cmp -s include/float_view_core.h "$temporary/float_view_core.h"; then
    diff -u include/float_view_core.h "$temporary/float_view_core.h" || true
    echo "Generated C header drifted. Run: just generate core-header" >&2
    exit 1
fi

cd "$REPOSITORY_ROOT"
shasum -a 256 -c backends/core/checksums.sha256
