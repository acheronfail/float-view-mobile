#!/usr/bin/env bash
set -euo pipefail

apple_root="$(cd "$(dirname "$0")" && pwd)"
repository_root="$(cd "$apple_root/../.." && pwd)"
: "${FLOAT_VIEW_VERSION:?Set the stable, candidate or nightly version}"
metadata="$(node --strip-types "$repository_root/.github/scripts/version.ts" native)"
read -r app_version build_number <<< "$metadata"
version="$FLOAT_VIEW_VERSION"

bash "$repository_root/backends/core/build-apple-runtime.sh" macosx 'arm64 x86_64' Release \
  "$repository_root/backends/core/target/apple/macos"
scratch="${FLOAT_VIEW_MACOS_BUILD_ROOT:-${RUNNER_TEMP:-${TMPDIR:-/tmp}}/float-view-macos-build}"
cd "$apple_root"
swift_arguments=(-c release --scratch-path "$scratch" --arch arm64 --arch x86_64)
swift build "${swift_arguments[@]}" --product FloatViewMac
binary_root="$(swift build "${swift_arguments[@]}" --show-bin-path)"
app="$scratch/Float View Desktop.app"
bash "$apple_root/package-macos.sh" "$binary_root/FloatViewMac" "$app" \
  "$binary_root/FloatViewApple_FloatViewMac.bundle/Contents/Resources/Resources"
/usr/libexec/PlistBuddy -c "Set :CFBundleVersion $build_number" "$app/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString $app_version" "$app/Contents/Info.plist"
# Both Swift and the linked Rust runtime must be available on both Mac families.
xcrun lipo "$app/Contents/MacOS/FloatViewMac" -verify_arch arm64 x86_64
output="$repository_root/FloatView-$version-macos-universal.zip"
if [[ -n "${GITHUB_OUTPUT:-}" ]]; then
    printf 'app-path=%s\noutput-archive=%s\n' "$app" "$output" >> "$GITHUB_OUTPUT"
fi
printf 'Built universal macOS app: %s\n' "$app"
