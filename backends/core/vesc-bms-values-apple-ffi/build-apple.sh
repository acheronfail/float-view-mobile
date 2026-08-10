#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 4 ]]; then
  echo 'Usage: build-apple.sh macosx|iphoneos|iphonesimulator "<archs>" Debug|Release <output-dir>' >&2
  exit 2
fi

apple_sdk="$1"
apple_architectures="$2"
apple_configuration="$3"
apple_output_dir="$4"
bridge_root="$(cd "$(dirname "$0")" && pwd)"
core_root="$(cd "$bridge_root/.." && pwd)"
archive_name='libfloat_view_vesc_bms_values_apple_ffi.a'

case "$apple_configuration" in
  Debug|debug) cargo_profile='debug'; cargo_release=0 ;;
  Release|release) cargo_profile='release'; cargo_release=1 ;;
  *) echo "Unsupported Apple Rust configuration: $apple_configuration (expected Debug or Release)" >&2; exit 2 ;;
esac

[[ -n "$apple_architectures" ]] || { echo "No architectures were supplied for Apple SDK $apple_sdk" >&2; exit 2; }
command -v cargo >/dev/null || { echo 'The repository-pinned Cargo toolchain is unavailable; install Rust 1.93.0 first.' >&2; exit 1; }
command -v rustup >/dev/null || { echo 'rustup is unavailable; install the repository-pinned Rust 1.93.0 toolchain first.' >&2; exit 1; }

mkdir -p "$apple_output_dir"
cargo_target_dir="$apple_output_dir/cargo"
archives=()
for apple_architecture in $apple_architectures; do
  case "$apple_sdk:$apple_architecture" in
    macosx:arm64) rust_target='aarch64-apple-darwin' ;;
    macosx:x86_64) rust_target='x86_64-apple-darwin' ;;
    iphoneos:arm64) rust_target='aarch64-apple-ios' ;;
    iphonesimulator:arm64) rust_target='aarch64-apple-ios-sim' ;;
    iphonesimulator:x86_64) rust_target='x86_64-apple-ios' ;;
    *) echo "Unsupported Apple Rust SDK/architecture: $apple_sdk/$apple_architecture" >&2; exit 2 ;;
  esac
  if ! rustup target list --installed | grep -Fx "$rust_target" >/dev/null; then
    echo "Missing pinned Rust target $rust_target for $apple_sdk/$apple_architecture; install it with: rustup target add $rust_target --toolchain 1.93.0" >&2
    exit 1
  fi
  (
    cd "$core_root"
    if [[ $cargo_release -eq 1 ]]; then
      CARGO_TARGET_DIR="$cargo_target_dir" cargo build --locked --offline --package float-view-vesc-bms-values-apple-ffi --target "$rust_target" --release
    else
      CARGO_TARGET_DIR="$cargo_target_dir" cargo build --locked --offline --package float-view-vesc-bms-values-apple-ffi --target "$rust_target"
    fi
  )
  archives+=("$cargo_target_dir/$rust_target/$cargo_profile/$archive_name")
done

destination="$apple_output_dir/$archive_name"
if [[ ${#archives[@]} -eq 1 ]]; then
  cp "${archives[0]}" "$destination"
else
  xcrun lipo -create "${archives[@]}" -output "$destination"
fi
[[ -f "$destination" ]] || { echo "Rust archive was not produced at $destination" >&2; exit 1; }
