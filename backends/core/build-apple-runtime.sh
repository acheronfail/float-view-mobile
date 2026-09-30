#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 4 ]]; then
  echo 'Usage: build-apple-runtime.sh macosx|iphoneos|iphonesimulator "<archs>" Debug|Release <output-dir>' >&2
  exit 2
fi

readonly core_root="$(cd "$(dirname "$0")" && pwd)"
readonly rust_toolchain='1.93.0'
readonly package='float-view-application-runtime-apple-ffi'
readonly archive='libfloat_view_application_runtime_apple_ffi.a'
apple_sdk="$1"
apple_architectures="$2"
apple_configuration="$3"
apple_output_dir="$4"

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
  echo "The repository-pinned Cargo toolchain is unavailable; install Rust $rust_toolchain first." >&2
  exit 1
}
command -v rustup >/dev/null || {
  echo "rustup is unavailable; install the repository-pinned Rust $rust_toolchain toolchain first." >&2
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
  rustup target list --toolchain "$rust_toolchain" --installed | grep -Fx "$rust_target" >/dev/null || {
    echo "Missing pinned Rust target $rust_target for $apple_sdk/$apple_architecture" >&2
    exit 1
  }
  rust_targets+=("$rust_target")
done

mkdir -p "$apple_output_dir"
apple_output_dir="$(cd "$apple_output_dir" && pwd)"
cargo_target_dir="$apple_output_dir/cargo"
archives=()
for rust_target in "${rust_targets[@]}"; do
  cargo_arguments=(
    +"$rust_toolchain" build --locked --offline --package "$package" --target "$rust_target"
  )
  if [[ $cargo_release -eq 1 ]]; then
    cargo_arguments+=(--release)
  fi
  (
    cd "$core_root"
    if [[ "$apple_sdk" == macosx ]]; then
      export MACOSX_DEPLOYMENT_TARGET=14.0
    else
      export IPHONEOS_DEPLOYMENT_TARGET=17.0
    fi
    CARGO_TARGET_DIR="$cargo_target_dir" cargo "${cargo_arguments[@]}"
  )
  archives+=("$cargo_target_dir/$rust_target/$cargo_profile/$archive")
done

destination="$apple_output_dir/$archive"
if [[ ${#archives[@]} -eq 1 ]]; then
  cp "${archives[0]}" "$destination"
else
  xcrun lipo -create "${archives[@]}" -output "$destination"
fi
[[ -f "$destination" ]]

expected_symbols="$(printf '%s\n' \
  abi_version \
  service_create service_destroy service_apply service_copy_result service_server_create \
  server_destroy server_command \
  | sed 's/^/float_view_application_runtime_/' | sort)"
# Inspect native symbols without parsing Rust's potentially newer LLVM bitcode.
actual_symbols="$(nm --no-llvm-bc -gU "$destination" 2>/dev/null | awk '{print $NF}' \
  | sed 's/^_//' | rg '^float_view_application_runtime_[a-z_]+$' | sort -u)"
[[ "$actual_symbols" == "$expected_symbols" ]] || {
  echo 'Unexpected application-runtime C symbols in Apple archive' >&2
  exit 1
}
