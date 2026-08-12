#!/usr/bin/env bash
set -euo pipefail

readonly core_root="$(cd "$(dirname "$0")" && pwd)"
readonly repository_root="$(cd "$core_root/../.." && pwd)"
# shellcheck source=bridge-build-catalog.sh
source "$core_root/bridge-build-catalog.sh"

usage() {
  echo 'Usage: build-android-bridge.sh BRIDGE host debug|release OUTPUT_DIRECTORY' >&2
  echo '       build-android-bridge.sh BRIDGE android ABI debug|release OUTPUT_DIRECTORY' >&2
  echo '       build-android-bridge.sh BRIDGE android-all debug|release OUTPUT_DIRECTORY' >&2
  exit 2
}

[[ $# -ge 1 ]] || usage
float_view_select_bridge "$1"
shift

[[ -n "$bridge_android_package" && -n "$bridge_android_library_stem" &&
   -n "$bridge_android_jni_prefix" && ${#bridge_android_symbols[@]} -gt 0 ]] || {
  echo "Rust host bridge $bridge_identifier is unavailable for Android." >&2
  exit 2
}

profile_arguments() {
  case "$1" in
    debug) profile='debug'; cargo_release='' ;;
    release) profile='release'; cargo_release='--release' ;;
    *) usage ;;
  esac
}

require_rust_toolchain() {
  command -v cargo >/dev/null || {
    echo "The repository-pinned Cargo toolchain is unavailable; install Rust $float_view_rust_toolchain first." >&2
    exit 1
  }
  command -v rustc >/dev/null || {
    echo "rustc is unavailable; install the repository-pinned Rust $float_view_rust_toolchain toolchain first." >&2
    exit 1
  }
  command -v rustup >/dev/null || {
    echo "rustup is unavailable; install the repository-pinned Rust $float_view_rust_toolchain toolchain first." >&2
    exit 1
  }
}

audit_symbols() {
  local artifact="$1"
  local nm_command="$2"
  local actual expected
  actual="$($nm_command --defined-only --extern-only "$artifact" 2>/dev/null \
    | awk '{print $NF}' \
    | sed 's/^_//' \
    | rg "^${bridge_android_jni_prefix}_|^JNI_OnLoad$" \
    | sort -u)"
  expected="$(printf '%s\n' "${bridge_android_symbols[@]}")"
  [[ "$actual" == "$expected" ]] || {
    echo "Unexpected Android $bridge_diagnostic JNI symbols in $artifact:" >&2
    printf '%s\n' "$actual" >&2
    exit 1
  }
}

resolve_ndk() {
  local sdk_directory=''
  if [[ -n "${ANDROID_NDK_HOME:-}" ]]; then
    ndk_directory="$ANDROID_NDK_HOME"
  elif [[ -n "${ANDROID_NDK_ROOT:-}" ]]; then
    ndk_directory="$ANDROID_NDK_ROOT"
  else
    if [[ -n "${ANDROID_HOME:-}" ]]; then
      sdk_directory="$ANDROID_HOME"
    elif [[ -n "${ANDROID_SDK_ROOT:-}" ]]; then
      sdk_directory="$ANDROID_SDK_ROOT"
    elif [[ -f "$repository_root/backends/android/local.properties" ]]; then
      sdk_directory="$(sed -n 's/^sdk\.dir=//p' "$repository_root/backends/android/local.properties" | head -1)"
      sdk_directory="${sdk_directory//\\:/:}"
      sdk_directory="${sdk_directory//\\\\/\\}"
    fi
    [[ -n "$sdk_directory" ]] || {
      echo 'Android SDK is unavailable; set ANDROID_HOME or ANDROID_SDK_ROOT.' >&2
      exit 1
    }
    ndk_directory="$sdk_directory/ndk/$float_view_android_ndk_version"
  fi
  [[ -f "$ndk_directory/source.properties" ]] || {
    echo "Pinned Android NDK $float_view_android_ndk_version is unavailable at $ndk_directory." >&2
    exit 1
  }
  rg -q "^Pkg.Revision = $float_view_android_ndk_version$" "$ndk_directory/source.properties" || {
    echo "Android NDK at $ndk_directory is not pinned revision $float_view_android_ndk_version." >&2
    exit 1
  }
  local prebuilt_root="$ndk_directory/toolchains/llvm/prebuilt"
  shopt -s nullglob
  local prebuilts=("$prebuilt_root"/*)
  shopt -u nullglob
  [[ ${#prebuilts[@]} -eq 1 && -d "${prebuilts[0]}" ]] || {
    echo "Expected exactly one NDK LLVM prebuilt beneath $prebuilt_root." >&2
    exit 1
  }
  llvm_bin="${prebuilts[0]}/bin"
}

build_host() {
  local selected_profile="$1"
  local output_directory="$2"
  profile_arguments "$selected_profile"
  require_rust_toolchain
  local host_target extension artifact destination nm_command
  host_target="$(rustc +"$float_view_rust_toolchain" -vV | sed -n 's/^host: //p')"
  [[ -n "$host_target" ]] || {
    echo 'Unable to determine the Rust host target.' >&2
    exit 1
  }
  case "$host_target" in
    *-apple-darwin) extension='dylib'; nm_command='nm' ;;
    *-linux-*) extension='so'; nm_command='nm' ;;
    *) echo "Unsupported host-JVM Rust target: $host_target" >&2; exit 1 ;;
  esac
  cargo +"$float_view_rust_toolchain" build --manifest-path "$core_root/Cargo.toml" \
    --package "$bridge_android_package" --locked --offline ${cargo_release:+"$cargo_release"}
  artifact="$core_root/target/$profile/lib$bridge_android_library_stem.$extension"
  [[ -f "$artifact" ]] || {
    echo "Host $bridge_diagnostic JNI library was not produced: $artifact" >&2
    exit 1
  }
  mkdir -p "$output_directory"
  destination="$output_directory/lib$bridge_android_library_stem.$extension"
  install -m 0644 "$artifact" "$destination"
  audit_symbols "$destination" "$nm_command"
  file "$destination" | rg -q 'Mach-O.*dynamically linked shared library|ELF.*shared object' || {
    echo "Host $bridge_diagnostic JNI artifact is not a shared library: $destination" >&2
    exit 1
  }
  printf '%s\n' "$destination"
}

build_android() {
  local abi="$1"
  local selected_profile="$2"
  local output_directory="$3"
  profile_arguments "$selected_profile"
  local target compiler architecture cargo_linker artifact destination
  case "$abi" in
    arm64-v8a)
      target='aarch64-linux-android'
      compiler="aarch64-linux-android${float_view_android_api}-clang"
      architecture='ARM aarch64'
      ;;
    armeabi-v7a)
      target='armv7-linux-androideabi'
      compiler="armv7a-linux-androideabi${float_view_android_api}-clang"
      architecture='ARM, EABI5'
      ;;
    x86)
      target='i686-linux-android'
      compiler="i686-linux-android${float_view_android_api}-clang"
      architecture='Intel 80386'
      ;;
    x86_64)
      target='x86_64-linux-android'
      compiler="x86_64-linux-android${float_view_android_api}-clang"
      architecture='x86-64'
      ;;
    *) echo "Unsupported Android ABI: $abi" >&2; exit 2 ;;
  esac
  require_rust_toolchain
  resolve_ndk
  rustup target list --toolchain "$float_view_rust_toolchain" --installed | rg -Fxq "$target" || {
    echo "Rust target $target is unavailable for toolchain $float_view_rust_toolchain." >&2
    exit 1
  }
  local linker="$llvm_bin/$compiler"
  [[ -x "$linker" ]] || {
    echo "Pinned NDK linker is unavailable: $linker" >&2
    exit 1
  }
  local target_environment
  target_environment="$(printf '%s' "$target" | tr '[:lower:]-' '[:upper:]_')"
  cargo_linker="CARGO_TARGET_${target_environment}_LINKER"
  env "$cargo_linker=$linker" cargo +"$float_view_rust_toolchain" build \
    --manifest-path "$core_root/Cargo.toml" --package "$bridge_android_package" \
    --target "$target" --locked --offline ${cargo_release:+"$cargo_release"}
  artifact="$core_root/target/$target/$profile/lib$bridge_android_library_stem.so"
  [[ -f "$artifact" ]] || {
    echo "Android $bridge_diagnostic JNI library was not produced: $artifact" >&2
    exit 1
  }
  mkdir -p "$output_directory/$abi"
  destination="$output_directory/$abi/lib$bridge_android_library_stem.so"
  install -m 0644 "$artifact" "$destination"
  audit_symbols "$destination" "$llvm_bin/llvm-nm"
  file "$destination" | rg -Fq "$architecture" || {
    echo "Android $bridge_diagnostic JNI artifact has the wrong architecture for $abi: $destination" >&2
    exit 1
  }
  printf '%s\n' "$destination"
}

case "${1:-}" in
  host)
    [[ $# -eq 3 ]] || usage
    build_host "$2" "$3"
    ;;
  android)
    [[ $# -eq 4 ]] || usage
    build_android "$2" "$3" "$4"
    ;;
  android-all)
    [[ $# -eq 3 ]] || usage
    for abi in arm64-v8a armeabi-v7a x86 x86_64; do
      build_android "$abi" "$2" "$3"
    done
    ;;
  *) usage ;;
esac
