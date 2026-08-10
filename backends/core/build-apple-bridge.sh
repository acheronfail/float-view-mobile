#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 5 ]]; then
  echo 'Usage: build-apple-bridge.sh BRIDGE macosx|iphoneos|iphonesimulator "<archs>" Debug|Release <output-dir>' >&2
  exit 2
fi

readonly core_root="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=bridge-build-catalog.sh
source "$core_root/bridge-build-catalog.sh"

float_view_select_bridge "$1"
apple_sdk="$2"
apple_architectures="$3"
apple_configuration="$4"
apple_output_dir="$5"

case "$apple_configuration" in
  Debug|debug) cargo_profile='debug'; cargo_release=0 ;;
  Release|release) cargo_profile='release'; cargo_release=1 ;;
  *)
    echo "Unsupported Apple Rust configuration: $apple_configuration (expected Debug or Release)" >&2
    exit 2
    ;;
esac

[[ -n "$apple_architectures" ]] || {
  echo "No architectures were supplied for Apple SDK $apple_sdk" >&2
  exit 2
}
command -v cargo >/dev/null || {
  echo "The repository-pinned Cargo toolchain is unavailable; install Rust $float_view_rust_toolchain first." >&2
  exit 1
}
command -v rustup >/dev/null || {
  echo "rustup is unavailable; install the repository-pinned Rust $float_view_rust_toolchain toolchain first." >&2
  exit 1
}

rust_targets=()
for apple_architecture in $apple_architectures; do
  case "$apple_sdk:$apple_architecture" in
    macosx:arm64) rust_target='aarch64-apple-darwin' ;;
    macosx:x86_64) rust_target='x86_64-apple-darwin' ;;
    iphoneos:arm64) rust_target='aarch64-apple-ios' ;;
    iphonesimulator:arm64) rust_target='aarch64-apple-ios-sim' ;;
    iphonesimulator:x86_64) rust_target='x86_64-apple-ios' ;;
    *)
      echo "Unsupported Apple Rust SDK/architecture: $apple_sdk/$apple_architecture" >&2
      exit 2
      ;;
  esac

  if ! rustup target list --toolchain "$float_view_rust_toolchain" --installed \
    | grep -Fx "$rust_target" >/dev/null; then
    echo "Missing pinned Rust target $rust_target for $apple_sdk/$apple_architecture; install it with: rustup target add $rust_target --toolchain $float_view_rust_toolchain" >&2
    exit 1
  fi
  rust_targets+=("$rust_target")
done

mkdir -p "$apple_output_dir"
cargo_target_dir="$apple_output_dir/cargo"
archives=()
for rust_target in "${rust_targets[@]}"; do
  cargo_arguments=(
    +"$float_view_rust_toolchain"
    build
    --locked
    --offline
    --package "$bridge_apple_package"
    --target "$rust_target"
  )
  if [[ $cargo_release -eq 1 ]]; then
    cargo_arguments+=(--release)
  fi
  (
    cd "$core_root"
    CARGO_TARGET_DIR="$cargo_target_dir" cargo "${cargo_arguments[@]}"
  )
  archives+=("$cargo_target_dir/$rust_target/$cargo_profile/$bridge_apple_archive")
done

destination="$apple_output_dir/$bridge_apple_archive"
if [[ ${#archives[@]} -eq 1 ]]; then
  cp "${archives[0]}" "$destination"
else
  xcrun lipo -create "${archives[@]}" -output "$destination"
fi
[[ -f "$destination" ]] || {
  echo "$bridge_diagnostic Rust archive was not produced at $destination" >&2
  exit 1
}
