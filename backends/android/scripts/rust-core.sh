#!/bin/sh
set -eu

EXPECTED_RUST="1.93.0"
EXPECTED_CARGO_NDK="4.1.2"
EXPECTED_JAVA="17"
EXPECTED_NDK="28.2.13676358"
EXPECTED_ABI_VERSION="4"
LIBRARY_NAME="libhost_android_jni.so"

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPOSITORY_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/../../.." && pwd)
CORE_DIR="$REPOSITORY_ROOT/backends/core"

fail() {
    printf 'Android Rust packaging validation failed: %s\n' "$1" >&2
    exit 1
}

require_value() {
    actual=$1
    expected=$2
    label=$3
    corrective=$4
    if [ "$actual" != "$expected" ]; then
        fail "$label mismatch (expected $expected, found $actual). $corrective"
    fi
}

sdk_root() {
    root=${ANDROID_SDK_ROOT:-${ANDROID_HOME:-}}
    [ -n "$root" ] || fail "ANDROID_SDK_ROOT or ANDROID_HOME must identify Android SDK 36"
    printf '%s\n' "$root"
}

ndk_root() {
    printf '%s/ndk/%s\n' "$(sdk_root)" "$EXPECTED_NDK"
}

validate_toolchain() {
    command -v rustc >/dev/null 2>&1 || fail "rustc is missing; install Rust $EXPECTED_RUST"
    command -v cargo >/dev/null 2>&1 || fail "cargo is missing; install Rust $EXPECTED_RUST"
    command -v java >/dev/null 2>&1 || fail "java is missing; run Gradle with JDK $EXPECTED_JAVA"

    rust_version=$(rustc -V | awk '{print $2}')
    require_value "$rust_version" "$EXPECTED_RUST" "Rust" \
        "Run rustup toolchain install $EXPECTED_RUST --profile minimal."

    cargo_ndk_version=$(cargo ndk --version 2>/dev/null | awk '{print $2}')
    require_value "$cargo_ndk_version" "$EXPECTED_CARGO_NDK" "cargo-ndk" \
        "Run cargo install --locked cargo-ndk --version $EXPECTED_CARGO_NDK."

    java_version=$(java -XshowSettings:properties -version 2>&1 \
        | awk -F'= ' '/java.specification.version/ { print $2; exit }')
    require_value "$java_version" "$EXPECTED_JAVA" "Gradle JDK" \
        "Set JAVA_HOME to a JDK $EXPECTED_JAVA installation."

    ndk=$(ndk_root)
    [ -d "$ndk" ] || fail "NDK $EXPECTED_NDK is missing. Install ndk;$EXPECTED_NDK from Android Studio."
    revision=$(awk -F'= ' '/^Pkg.Revision/ { print $2; exit }' "$ndk/source.properties")
    require_value "$revision" "$EXPECTED_NDK" "Android NDK" \
        "Install ndk;$EXPECTED_NDK from Android Studio."

    for target in \
        aarch64-linux-android \
        armv7-linux-androideabi \
        i686-linux-android \
        x86_64-linux-android
    do
        rustup target list --installed | grep -qx "$target" \
            || fail "Rust target $target is missing. Run rustup target add --toolchain $EXPECTED_RUST $target."
    done

    header_abi=$(awk '/^#define FLOAT_VIEW_ABI_VERSION / { print $3; exit }' \
        "$CORE_DIR/include/float_view_core.h")
    require_value "$header_abi" "$EXPECTED_ABI_VERSION" "checked-in C header ABI" \
        "Regenerate and review the core header and host wrappers together."

    (cd "$REPOSITORY_ROOT" && shasum -a 256 -c backends/core/checksums.sha256 >/dev/null) \
        || fail "repository-owned tool/header checksum mismatch"
    cargo metadata --manifest-path "$CORE_DIR/Cargo.toml" --locked --offline \
        --no-deps --format-version 1 >/dev/null \
        || fail "Cargo.lock or the offline locked dependency graph does not match the workspace"
}

rust_target_for_abi() {
    case "$1" in
        arm64-v8a) printf '%s\n' aarch64-linux-android ;;
        armeabi-v7a) printf '%s\n' armv7-linux-androideabi ;;
        x86) printf '%s\n' i686-linux-android ;;
        x86_64) printf '%s\n' x86_64-linux-android ;;
        *) fail "unsupported Android ABI $1" ;;
    esac
}

build_abi() {
    [ "$#" -eq 2 ] || fail "build requires ABI and output directory"
    abi=$1
    output=$2
    rust_target_for_abi "$abi" >/dev/null
    mkdir -p "$output"
    export ANDROID_NDK_HOME
    ANDROID_NDK_HOME=$(ndk_root)
    export NDK_HOME="$ANDROID_NDK_HOME"
    (cd "$CORE_DIR" && cargo ndk --platform 29 --target "$abi" --output-dir "$output" \
        build --release --package host-android-jni --locked --offline)
    [ -f "$output/$abi/$LIBRARY_NAME" ] \
        || fail "Cargo did not produce $output/$abi/$LIBRARY_NAME"
}

verify_symbol_once() {
    symbols=$1
    name=$2
    count=$(awk -v name="$name" '$NF == name { count += 1 } END { print count + 0 }' "$symbols")
    [ "$count" -eq 1 ] || fail "$name must be exported exactly once (found $count)"
}

verify_abi() {
    abi=$1
    output=$2
    library="$output/$abi/$LIBRARY_NAME"
    [ -f "$library" ] || fail "missing packaged library for $abi: $library"

    readelf="$(ndk_root)/toolchains/llvm/prebuilt/darwin-x86_64/bin/llvm-readelf"
    [ -x "$readelf" ] || fail "NDK llvm-readelf is missing at $readelf"
    symbols="$output/$abi/dynamic-symbols.txt"
    "$readelf" --dyn-syms --wide "$library" > "$symbols"

    verify_symbol_once "$symbols" JNI_OnLoad
    verify_symbol_once "$symbols" float_view_abi_version
    verify_symbol_once "$symbols" float_view_build_id
    if awk '$NF ~ /^Java_/ { found = 1 } END { exit !found }' "$symbols"; then
        fail "$abi exports name-derived Java_ JNI symbols; native methods must be explicitly registered"
    fi
    awk '$NF == "JNI_OnLoad" && $5 == "GLOBAL" && $6 == "DEFAULT" { found = 1 } END { exit !found }' \
        "$symbols" || fail "$abi JNI_OnLoad does not have global default visibility"

    expected_exports='JNI_OnLoad
float_view_abi_version
float_view_build_id
float_view_runtime_complete_operation
float_view_runtime_create
float_view_runtime_destroy
float_view_runtime_start
float_view_runtime_start_configured
float_view_runtime_start_on_port
float_view_runtime_stop
float_view_runtime_submit_ble_bytes
float_view_runtime_submit_bluetooth_connection
float_view_runtime_submit_bluetooth_device
float_view_runtime_submit_bluetooth_state
float_view_runtime_submit_event
float_view_runtime_submit_lifecycle
float_view_runtime_submit_location
float_view_runtime_submit_location_authorization
float_view_runtime_wait_command'
    actual_exports=$(awk '$5 == "GLOBAL" && $6 == "DEFAULT" && $7 != "UND" { print $NF }' \
        "$symbols" | LC_ALL=C sort)
    [ "$actual_exports" = "$expected_exports" ] \
        || fail "$abi exported-symbol allowlist mismatch"

    soname=$($readelf -d "$library" | awk -F'[][]' '/SONAME/ { print $2; exit }')
    if [ -n "$soname" ] && [ "$soname" != "$LIBRARY_NAME" ]; then
        fail "$abi ELF SONAME mismatch (expected $LIBRARY_NAME or no SONAME, found $soname)"
    fi

    machine=$($readelf -h "$library" | awk -F': *' '/Machine:/ { print $2; exit }')
    case "$abi:$machine" in
        "arm64-v8a:AArch64"|"armeabi-v7a:ARM"|"x86:Intel 80386"|"x86_64:Advanced Micro Devices X86-64") ;;
        *) fail "$abi ELF machine mismatch (found $machine)" ;;
    esac

    if "$readelf" -d "$library" | grep -q 'Shared library: \[libsqlite3.so\]'; then
        fail "$abi links system libsqlite3.so instead of Cargo-bundled SQLite"
    fi
}

verify_packaging() {
    [ "$#" -eq 1 ] || fail "verify requires the packaged JNI root"
    output=$1
    expected_count=0
    for abi in arm64-v8a armeabi-v7a x86 x86_64; do
        verify_abi "$abi" "$output"
        expected_count=$((expected_count + 1))
    done

    actual_count=$(find "$output" -type f -name '*.so' | wc -l | tr -d ' ')
    [ "$actual_count" -eq "$expected_count" ] \
        || fail "native-library duplication detected (expected $expected_count .so files, found $actual_count)"

    manifest="$output/manifest.sha256"
    : > "$manifest"
    for abi in arm64-v8a armeabi-v7a x86 x86_64; do
        shasum -a 256 "$output/$abi/$LIBRARY_NAME" >> "$manifest"
    done
}

command=${1:-}
case "$command" in
    validate)
        [ "$#" -eq 1 ] || fail "validate takes no arguments"
        validate_toolchain
        ;;
    build)
        shift
        build_abi "$@"
        ;;
    verify)
        shift
        validate_toolchain
        verify_packaging "$@"
        ;;
    *) fail "usage: $0 validate | build ABI OUTPUT | verify OUTPUT" ;;
esac
