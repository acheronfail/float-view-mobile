#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
CORE_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
REPOSITORY_ROOT=$(CDPATH= cd -- "$CORE_DIR/../.." && pwd)

fail() {
    printf 'Portable backend audit failed: %s\n' "$1" >&2
    exit 1
}

cd "$REPOSITORY_ROOT"
shasum -a 256 -c backends/core/checksums.sha256 >/dev/null
cargo metadata --manifest-path backends/core/Cargo.toml --locked --offline \
    --no-deps --format-version 1 >/dev/null

if rg -n '^source = "git\+' backends/core/Cargo.lock >/dev/null; then
    fail "Cargo.lock contains a mutable git dependency"
fi
if rg -n '(^|[,{[:space:]])git[[:space:]]*=' \
    backends/core/Cargo.toml backends/core/crates -g 'Cargo.toml' >/dev/null; then
    fail "a Cargo manifest contains a git dependency"
fi
if rg -n -i 'uniffi|\bjna\b|swift-java' \
    backends/core/crates backends/core/Cargo.toml -g '*.rs' -g 'Cargo.toml' >/dev/null; then
    fail "a prohibited interoperability layer is present in the portable core"
fi

if rg -n 'FlyingFox|CSQLite|SQLiteDatabase|io\.ktor|NATIVE_BACKEND|floatViewBackend' \
    backends/apple/Package.swift backends/apple/Sources backends/apple/ios/float-view.xcodeproj \
    backends/android/app backends/android/gradle/libs.versions.toml \
    -g '!**/build/**' -g '!**/.build/**' >/dev/null; then
    fail "a deleted backend dependency or compatibility selector remains"
fi

for removed_owner in \
    backends/apple/Sources/FloatViewBackend/protocol \
    backends/apple/Sources/FloatViewBackend/server \
    backends/apple/Sources/FloatViewBackend/storage \
    backends/apple/Sources/CSQLite \
    backends/android/app/src/main/java/fail/acheron/floatview/protocol \
    backends/android/app/src/main/java/fail/acheron/floatview/storage
do
    if [ -d "$removed_owner" ] && find "$removed_owner" -type f | grep -q .; then
        fail "deleted native behavior owner remains under $removed_owner"
    fi
done

for removed_file in \
    backends/apple/Sources/FloatViewBackend/application/ApplicationService.swift \
    backends/apple/Sources/FloatViewBackend/server/LoopbackServer.swift \
    backends/apple/Sources/FloatViewBackend/storage/SQLiteLogStore.swift \
    backends/android/app/src/main/java/fail/acheron/floatview/BackendRuntime.kt \
    backends/android/app/src/main/java/fail/acheron/floatview/LoopbackServer.kt \
    backends/android/app/src/main/java/fail/acheron/floatview/backend/ApplicationService.kt \
    backends/android/app/src/main/java/fail/acheron/floatview/storage/SQLiteLogStore.kt
do
    [ ! -e "$removed_file" ] || fail "deleted native behavior owner remains: $removed_file"
done

rg -q 'rusqlite = .*features = \["bundled"\]' backends/core/crates/infra-sqlite-storage/Cargo.toml \
    || fail "rusqlite must select the bundled SQLite source"
cargo tree --manifest-path backends/core/Cargo.toml --locked --offline \
    -e features -i libsqlite3-sys | rg -q 'libsqlite3-sys feature "bundled"' \
    || fail "the resolved SQLite feature graph is not bundled"

for dependency in \
    axum base64 crc32c csv flate2 futures-util getrandom jni roxmltree rusqlite \
    serde serde_json sha2 tokio tower zip
do
    grep -Eq '^\| `'$dependency'`[[:space:]]*\|' backends/core/DEPENDENCIES.md \
        || fail "DEPENDENCIES.md does not document direct dependency $dependency"
done

header_abi=$(awk '/^#define FLOAT_VIEW_ABI_VERSION / { print $3; exit }' \
    backends/core/include/float_view_core.h)
rust_abi=$(awk '/^pub const FLOAT_VIEW_ABI_VERSION:/ { value=$NF; gsub(/[^0-9]/, "", value); print value; exit }' \
    backends/core/crates/host-c-abi/src/lib.rs)
[ -n "$header_abi" ] && [ "$header_abi" = "$rust_abi" ] \
    || fail "generated header and Rust ABI versions differ"

printf 'Portable backend audit passed: %s Rust, %s Swift, %s Kotlin source files; %s manifests.\n' \
    "$(find backends/core/crates -type f -name '*.rs' | wc -l | tr -d ' ')" \
    "$(find backends/apple/Sources backends/apple/Tests backends/apple/ios/float-view backends/apple/ios/float-view-lifecycle-tests -type f -name '*.swift' | wc -l | tr -d ' ')" \
    "$(find backends/android/app/src -type f -name '*.kt' | wc -l | tr -d ' ')" \
    "$(find backends/core -type f -name Cargo.toml | wc -l | tr -d ' ')"
