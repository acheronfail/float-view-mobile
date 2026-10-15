#!/usr/bin/env bash
set -euo pipefail

apple_root="$(cd "$(dirname "$0")" && pwd)"
repository_root="$(cd "$apple_root/../.." && pwd)"
: "${FLOAT_VIEW_VERSION:?Set the v0.1.x release tag}"
: "${CODE_SIGN_IDENTITY:?Use with-macos-signing-identity.sh}"
: "${CODE_SIGN_KEYCHAIN:?Use with-macos-signing-identity.sh}"
: "${CODE_SIGN_REQUIREMENTS:?Use with-macos-signing-identity.sh}"
: "${CODE_SIGN_REQUIREMENT:?Use with-macos-signing-identity.sh}"
[[ "$FLOAT_VIEW_VERSION" =~ ^v0\.1\.(0|[1-9][0-9]*)$ ]] || { echo 'Invalid release version' >&2; exit 2; }
version="$FLOAT_VIEW_VERSION"
[[ "$CODE_SIGN_IDENTITY" != - ]] || { echo 'A persistent signing identity is required' >&2; exit 2; }

bash "$repository_root/backends/core/build-apple-runtime.sh" macosx 'arm64 x86_64' Release \
  "$repository_root/backends/core/target/apple/macos"
scratch="${RUNNER_TEMP:?}/float-view-macos-build"
cd "$apple_root"
swift_arguments=(-c release --scratch-path "$scratch" --arch arm64 --arch x86_64)
swift build "${swift_arguments[@]}" --product FloatViewMac
binary_root="$(swift build "${swift_arguments[@]}" --show-bin-path)"
app="$scratch/FloatView.app"
bash "$apple_root/package-macos.sh" "$binary_root/FloatViewMac" "$app" \
  "$binary_root/FloatViewApple_FloatViewMac.bundle/Contents/Resources/Resources"
/usr/libexec/PlistBuddy -c "Set :CFBundleVersion ${version##*.}" "$app/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString ${version#v}" "$app/Contents/Info.plist"
# Both Swift and the linked Rust runtime must be available on both Mac families.
xcrun lipo "$app/Contents/MacOS/FloatViewMac" -verify_arch arm64 x86_64
codesign --force --sign "$CODE_SIGN_IDENTITY" --keychain "$CODE_SIGN_KEYCHAIN" \
  --requirements "$CODE_SIGN_REQUIREMENTS" --timestamp=none "$app"
codesign --verify --deep --strict --test-requirement "=$CODE_SIGN_REQUIREMENT" "$app"
codesign --display --requirements - "$app"
output="$repository_root/FloatView-$version-macos-universal.zip"
ditto -c -k --sequesterRsrc --keepParent "$app" "$output"
unzip -tq "$output"
printf 'Universal macOS app: %s\n' "$output"
