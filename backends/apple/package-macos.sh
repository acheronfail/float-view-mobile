#!/usr/bin/env bash
# Assemble a development app so Core Location sees our identity and usage text.
set -euo pipefail
# Shipped resources must be readable after a root-owned installer deploys them.
umask 022

executable="${1:?Usage: package-macos.sh executable destination.app [resources-directory]}"
bundle="${2:?Usage: package-macos.sh executable destination.app [resources-directory]}"
# Native SwiftPM builds use a flat bundle; universal XCBuild uses Contents/Resources.
resources="${3:-$(dirname "$executable")/FloatViewApple_FloatViewMac.bundle/Resources}"
apple_root="$(cd "$(dirname "$0")" && pwd)"
contents="$bundle/Contents"
mkdir -p "$contents/MacOS" "$contents/Resources"
cp "$apple_root/FloatViewMac-Info.plist" "$contents/Info.plist"
# Replace the inode rather than overwriting a potentially running executable.
rm -f "$contents/MacOS/FloatViewMac"
cp "$executable" "$contents/MacOS/FloatViewMac"
# Put SwiftPM resources in the standard app location for signing and discovery.
cp -R "$resources/." "$contents/Resources/"
# Compile appearance variants into Assets.car, with a light ICNS fallback for
# systems before macOS 26. Keep this before either ad-hoc or distribution signing.
xcrun actool "$apple_root/../../tools/icon/FloatView.icon" \
  --compile "$contents/Resources" --platform macosx \
  --minimum-deployment-target 14.0 --app-icon FloatView \
  --output-partial-info-plist "$contents/Resources/FloatView-icon-info.plist"
rm -f "$contents/Resources/FloatView-icon-info.plist"
# Source resources can have inherited restrictive build permissions.
chmod -R a+rX "$bundle"
/usr/bin/plutil -lint "$contents/Info.plist"
/usr/bin/codesign --force --sign - "$bundle"
