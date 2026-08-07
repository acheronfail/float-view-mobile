#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
SCRATCH_PATH=${1:-/tmp/float-view-swift-build}

sh "$SCRIPT_DIR/build-core-macos.sh"

CORE_LIBRARY="$SCRIPT_DIR/../core/target/debug/libfloat_view_native_abi.a"
BIN_PATH=$(cd "$SCRIPT_DIR" && swift build --scratch-path "$SCRATCH_PATH" --show-bin-path)
EXECUTABLE="$BIN_PATH/FloatViewMac"

# SwiftPM does not track an external static archive as an input. Clean only
# when Cargo produced a newer archive so the next Swift build must relink it.
if [ ! -f "$EXECUTABLE" ] || [ "$CORE_LIBRARY" -nt "$EXECUTABLE" ]; then
    (cd "$SCRIPT_DIR" && swift package clean --scratch-path "$SCRATCH_PATH")
fi
