#!/bin/sh
set -eu

EXPECTED_CBINDGEN="0.29.2"
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
CORE_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
REPOSITORY_ROOT=$(CDPATH= cd -- "$CORE_DIR/../.." && pwd)
HEADER="$CORE_DIR/include/float_view_core.h"
CHECKSUMS="$CORE_DIR/checksums.sha256"

actual=$(cbindgen --version 2>/dev/null | awk '{ print $2 }')
if [ "$actual" != "$EXPECTED_CBINDGEN" ]; then
    echo "cbindgen mismatch: expected $EXPECTED_CBINDGEN, found ${actual:-missing}" >&2
    echo "Run: cargo install --locked cbindgen --version $EXPECTED_CBINDGEN" >&2
    exit 1
fi

cargo metadata --manifest-path "$CORE_DIR/Cargo.toml" --locked --offline \
    --no-deps --format-version 1 >/dev/null

temporary=$(mktemp -d "${TMPDIR:-/tmp}/float-view-core-header.XXXXXX")
trap 'rm -rf "$temporary"' EXIT HUP INT TERM

cd "$CORE_DIR"
cbindgen --config cbindgen.toml --crate float-view-native-abi \
    --output "$temporary/float_view_core.h"
mv "$temporary/float_view_core.h" "$HEADER"

hash=$(shasum -a 256 "$HEADER" | awk '{ print $1 }')
awk -v hash="$hash" '
    $2 == "backends/core/include/float_view_core.h" {
        print hash "  backends/core/include/float_view_core.h"
        found = 1
        next
    }
    { print }
    END {
        if (!found) {
            print hash "  backends/core/include/float_view_core.h"
        }
    }
' "$CHECKSUMS" > "$temporary/checksums.sha256"
mv "$temporary/checksums.sha256" "$CHECKSUMS"

cd "$REPOSITORY_ROOT"
shasum -a 256 -c backends/core/checksums.sha256
