#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
CORE_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
REPOSITORY_ROOT=$(CDPATH= cd -- "$CORE_DIR/../.." && pwd)

fail() {
    printf 'Portable backend prerequisite failed: %s\n' "$1" >&2
    exit 1
}

expect() {
    label=$1
    expected=$2
    actual=$3
    corrective=$4
    [ "$actual" = "$expected" ] \
        || fail "$label mismatch (expected $expected, found ${actual:-missing}). $corrective"
}

expect "Rust" "1.93.0" "$(rustc --version 2>/dev/null | awk '{ print $2 }')" \
    "Run rustup toolchain install 1.93.0 --profile minimal --component clippy,rustfmt."
expect "cbindgen" "0.29.2" "$(cbindgen --version 2>/dev/null | awk '{ print $2 }')" \
    "Run cargo install --locked cbindgen --version 0.29.2."
expect "cargo-ndk" "4.1.2" "$(cargo ndk --version 2>/dev/null | awk '{ print $2 }')" \
    "Run cargo install --locked cargo-ndk --version 4.1.2."
expect "JDK" "17" "$(java -XshowSettings:properties -version 2>&1 | awk -F'= ' '/java.specification.version/ { print $2; exit }')" \
    "Set JAVA_HOME to a JDK 17 installation."

for target in \
    aarch64-apple-darwin \
    aarch64-apple-ios \
    aarch64-apple-ios-sim \
    x86_64-apple-ios \
    aarch64-linux-android \
    armv7-linux-androideabi \
    i686-linux-android \
    x86_64-linux-android
do
    rustup target list --installed | grep -qx "$target" \
        || fail "Rust target $target is missing. Run rustup target add --toolchain 1.93.0 $target."
done

expect "Xcode" "26.3" "$(xcodebuild -version 2>/dev/null | awk 'NR == 1 { print $2 }')" \
    "Select Xcode 26.3 with xcode-select."
expect "Swift" "6.2.4" "$(swift --version 2>/dev/null | awk 'NR == 1 { sub(/\)/, "", $4); print $4 }')" \
    "Use the Swift toolchain bundled with Xcode 26.3."

sdk_root=${ANDROID_SDK_ROOT:-${ANDROID_HOME:-}}
if [ -z "$sdk_root" ] && [ -f "$REPOSITORY_ROOT/backends/android/local.properties" ]; then
    sdk_root=$(sed -n 's/^sdk\.dir=//p' "$REPOSITORY_ROOT/backends/android/local.properties" | head -n 1)
fi
[ -n "$sdk_root" ] || fail "Android SDK is not configured. Set ANDROID_SDK_ROOT or sdk.dir."
ndk_properties="$sdk_root/ndk/28.2.13676358/source.properties"
[ -f "$ndk_properties" ] \
    || fail "Android NDK 28.2.13676358 is missing. Install it from Android Studio."
expect "Android NDK" "28.2.13676358" \
    "$(awk -F'= ' '/^Pkg.Revision/ { print $2; exit }' "$ndk_properties")" \
    "Install NDK 28.2.13676358 from Android Studio."

cd "$REPOSITORY_ROOT"
shasum -a 256 -c backends/core/checksums.sha256
cargo metadata --manifest-path backends/core/Cargo.toml --locked --offline \
    --no-deps --format-version 1 >/dev/null
