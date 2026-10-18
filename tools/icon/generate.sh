#!/usr/bin/env bash

set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
project_root="$(cd -- "$script_dir/../.." && pwd)"
source_svg="$script_dir/icon.svg"
temporary_dir="$(mktemp -d "${TMPDIR:-/tmp}/float-view-icons.XXXXXX")"

cleanup() {
  rm -rf -- "$temporary_dir"
}
trap cleanup EXIT

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    printf 'tools/icon: required command not found: %s\n' "$1" >&2
    exit 1
  fi
}

require_command rsvg-convert
require_command magick
require_command python3
require_command sips

if [[ "${1:-}" != "--ios-only" ]]; then
  python3 "$script_dir/generate-loading-symbol.py"
fi

if [[ ! -f "$source_svg" ]]; then
  printf 'tools/icon: source SVG not found: %s\n' "$source_svg" >&2
  exit 1
fi

ios_icon_dir="$project_root/backends/apple/ios/float-view/Assets.xcassets/AppIcon.appiconset"
mac_resources_dir="$project_root/backends/apple/macos/Resources"
android_resources_dir="$project_root/backends/android/app/src/main/res"
master_png="$temporary_dir/FloatView.png"
mac_master_png="$temporary_dir/FloatView-macos.png"
transparent_mark_png="$temporary_dir/FloatView-mark.png"
adaptive_foreground_png="$temporary_dir/ic_launcher_foreground.png"
adaptive_monochrome_png="$temporary_dir/ic_launcher_monochrome.png"
round_master_png="$temporary_dir/FloatView-round.png"

mkdir -p \
  "$ios_icon_dir" \
  "$mac_resources_dir" \
  "$android_resources_dir/drawable-nodpi"

rsvg-convert -w 1024 -h 1024 "$source_svg" -o "$master_png"

# iOS uses the light artwork by default and the original artwork in Dark Mode.
# Derive the variant from the same geometry so future artwork edits stay in sync.
magick "$master_png" -alpha off -define png:exclude-chunk=all "$ios_icon_dir/AppIcon-Dark.png"
python3 - "$source_svg" "$temporary_dir/icon-light.svg" <<'PYTHON'
import pathlib
import sys

source = pathlib.Path(sys.argv[1]).read_text()
source = source.replace("#121418", "#f1f5f9").replace("#a2d4d8", "#0f766e").replace("#000000", "#0f766e")
pathlib.Path(sys.argv[2]).write_text(source)
PYTHON
rsvg-convert -w 1024 -h 1024 "$temporary_dir/icon-light.svg" -o "$temporary_dir/light.png"
magick "$temporary_dir/light.png" -alpha off -define png:exclude-chunk=all "$ios_icon_dir/AppIcon.png"

if [[ "${1:-}" == "--ios-only" ]]; then
  printf 'Generated iOS light and dark icons from %s\n' "$source_svg"
  exit 0
fi

# macOS does not mask ICNS artwork. Inset the face to 824px on a 1024px
# transparent canvas and apply the rounded macOS silhouette before resizing.
rsvg-convert -w 1024 -h 1024 "$script_dir/macos-mask.svg" \
  -o "$temporary_dir/macos-mask.png"
magick "$master_png" -resize 824x824! -background none -gravity center \
  -extent 1024x1024 "$temporary_dir/macos-mask.png" \
  -compose DstIn -composite "$mac_master_png"

# macOS consumes an ICNS. sips writes a valid 512px ICNS on every supported
# macOS version; use iconutil's richer iconset conversion when it is available
# and functioning, then fall back to the portable single-size ICNS.
mac_iconset_dir="$temporary_dir/FloatView.iconset"
mkdir -p "$mac_iconset_dir"
for icon_spec in \
  '16 icon_16x16.png' \
  '32 icon_16x16@2x.png' \
  '32 icon_32x32.png' \
  '64 icon_32x32@2x.png' \
  '128 icon_128x128.png' \
  '256 icon_128x128@2x.png' \
  '256 icon_256x256.png' \
  '512 icon_256x256@2x.png' \
  '512 icon_512x512.png' \
  '1024 icon_512x512@2x.png'; do
  read -r icon_size icon_name <<< "$icon_spec"
  magick "$mac_master_png" -resize "${icon_size}x${icon_size}!" \
    -define png:exclude-chunk=all "$mac_iconset_dir/$icon_name"
done

mac_icon_generated=false
if command -v iconutil >/dev/null 2>&1; then
  if iconutil -c icns -o "$mac_resources_dir/FloatView.icns" "$mac_iconset_dir" \
    >/dev/null 2>&1; then
    mac_icon_generated=true
  fi
fi
if [[ "$mac_icon_generated" != true ]]; then
  mac_source_png="$temporary_dir/FloatView-512.png"
  magick "$mac_master_png" -resize 512x512! "$mac_source_png"
  sips -s format icns "$mac_source_png" --out "$mac_resources_dir/FloatView.icns" \
    >/dev/null
fi

# Android keeps density-specific WebP fallbacks for pre-adaptive-icon devices.
# The round variants are explicitly circular so launchers do not have to mask them.
icon_background="$(magick "$master_png" -format '%[pixel:p{0,0}]' info:)"
magick "$master_png" -alpha on -fuzz 1% -transparent "$icon_background" \
  "$transparent_mark_png"
magick -size 1024x1024 "xc:$icon_background" \
  -fill "$icon_background" -draw 'circle 512,512 512,0' \
  "$transparent_mark_png" -compose over -composite "$round_master_png"

for density_spec in \
  'mdpi 48' \
  'hdpi 72' \
  'xhdpi 96' \
  'xxhdpi 144' \
  'xxxhdpi 192'; do
  read -r density density_size <<< "$density_spec"
  density_dir="$android_resources_dir/mipmap-$density"
  mkdir -p "$density_dir"
  magick "$master_png" -resize "${density_size}x${density_size}!" \
    -define webp:lossless=true "$density_dir/ic_launcher.webp"
  magick "$round_master_png" -resize "${density_size}x${density_size}!" \
    -define webp:lossless=true "$density_dir/ic_launcher_round.webp"
done

# Adaptive icons use a padded foreground layer so the rider mark remains inside
# Android's safe zone, plus a monochrome silhouette for themed launchers.
magick "$transparent_mark_png" -resize 66x66! "$temporary_dir/padded-mark.png"
magick -size 108x108 canvas:none "$temporary_dir/padded-mark.png" \
  -gravity center -composite "$adaptive_foreground_png"
magick "$adaptive_foreground_png" -fill white -colorize 100% \
  "$adaptive_monochrome_png"
magick "$adaptive_foreground_png" -define webp:lossless=true \
  "$android_resources_dir/drawable-nodpi/ic_launcher_foreground.webp"
magick "$adaptive_monochrome_png" -define webp:lossless=true \
  "$android_resources_dir/drawable-nodpi/ic_launcher_monochrome.webp"
magick -size 108x108 "xc:$icon_background" -define webp:lossless=true \
  "$android_resources_dir/drawable-nodpi/ic_launcher_background.webp"

printf 'Generated Float View icons from %s\n' "$source_svg"
