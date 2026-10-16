#!/usr/bin/env bash
set -euo pipefail

apple_root="$(cd "$(dirname "$0")" && pwd)"
repository_root="$(cd "$apple_root/../.." && pwd)"
: "${FLOAT_VIEW_VERSION:?Set the stable or nightly version}"
: "${CODE_SIGN_IDENTITY:?Use with-macos-signing-identity.sh}"
: "${APPLE_TEAM_ID:?Set the publishing Apple Team ID}"
[[ "$APPLE_TEAM_ID" =~ ^[A-Z0-9]{10}$ ]] || { echo 'Invalid Apple Team ID.' >&2; exit 2; }
metadata="$(node --strip-types "$repository_root/.github/scripts/version.ts" native)"
read -r app_version build_number <<< "$metadata"
version="$FLOAT_VIEW_VERSION"
[[ "$CODE_SIGN_IDENTITY" != - ]] || { echo 'A persistent signing identity is required' >&2; exit 2; }

bash "$repository_root/backends/core/build-apple-runtime.sh" macosx 'arm64 x86_64' Release \
  "$repository_root/backends/core/target/apple/macos"
scratch="${FLOAT_VIEW_MACOS_BUILD_ROOT:-${RUNNER_TEMP:-${TMPDIR:-/tmp}}/float-view-macos-build}"
cd "$apple_root"
swift_arguments=(-c release --scratch-path "$scratch" --arch arm64 --arch x86_64)
swift build "${swift_arguments[@]}" --product FloatViewMac
binary_root="$(swift build "${swift_arguments[@]}" --show-bin-path)"
app="$scratch/FloatView.app"
bash "$apple_root/package-macos.sh" "$binary_root/FloatViewMac" "$app" \
  "$binary_root/FloatViewApple_FloatViewMac.bundle/Contents/Resources/Resources"
/usr/libexec/PlistBuddy -c "Set :CFBundleVersion $build_number" "$app/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString $app_version" "$app/Contents/Info.plist"
# Both Swift and the linked Rust runtime must be available on both Mac families.
xcrun lipo "$app/Contents/MacOS/FloatViewMac" -verify_arch arm64 x86_64
sign_arguments=(--force --sign "$CODE_SIGN_IDENTITY" --options runtime --timestamp)
if [[ -n "${CODE_SIGN_KEYCHAIN:-}" ]]; then sign_arguments+=(--keychain "$CODE_SIGN_KEYCHAIN"); fi
codesign "${sign_arguments[@]}" "$app"
requirement="anchor apple generic and identifier \"fail.acheron.float-view.macos\" and certificate leaf[field.1.2.840.113635.100.6.1.13] exists and certificate leaf[subject.OU] = \"$APPLE_TEAM_ID\""
codesign --verify --deep --strict --test-requirement "=$requirement" "$app"
codesign --display --requirements - "$app"
output="$repository_root/FloatView-$version-macos-universal.zip"
bash "$apple_root/notarize-macos-release.sh" "$app" "$output"
