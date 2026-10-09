#!/usr/bin/env bash
# Assemble a development app so Core Location sees our identity and usage text.
set -euo pipefail

executable="${1:?Usage: package-macos.sh executable destination.app}"
bundle="${2:?Usage: package-macos.sh executable destination.app}"
apple_root="$(cd "$(dirname "$0")" && pwd)"
contents="$bundle/Contents"
mkdir -p "$contents/MacOS" "$contents/Resources"
cp "$apple_root/FloatViewMac-Info.plist" "$contents/Info.plist"
# Replace the inode rather than overwriting a potentially running executable.
rm -f "$contents/MacOS/FloatViewMac"
cp "$executable" "$contents/MacOS/FloatViewMac"
# Put SwiftPM resources in the standard app location for signing and discovery.
cp -R "$(dirname "$executable")/FloatViewApple_FloatViewMac.bundle/Resources/." "$contents/Resources/"
/usr/bin/plutil -lint "$contents/Info.plist"
/usr/bin/codesign --force --sign - "$bundle"
