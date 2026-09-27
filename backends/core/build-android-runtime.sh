#!/usr/bin/env bash
set -euo pipefail

readonly core_root="$(cd "$(dirname "$0")" && pwd)"
readonly repository_root="$(cd "$core_root/../.." && pwd)"
readonly package='float-view-application-runtime-android-jni'
readonly library_stem='float_view_application_runtime_android_jni'
readonly rust_toolchain='1.93.0'
readonly android_ndk_version='28.2.13676358'
readonly android_api='29'
readonly jni_prefix='Java_fail_acheron_floatview_application_RustApplicationRuntime'
readonly symbols=(
  "${jni_prefix}_serverCreate"
  "${jni_prefix}_serverDestroy"
  "${jni_prefix}_serverCommand"
  "${jni_prefix}_serverReadTransfer"
  "${jni_prefix}_serverWriteTransfer"
  "${jni_prefix}_abiVersion"
  "${jni_prefix}_apply"
  "${jni_prefix}_copyResult"
  "${jni_prefix}_create"
  "${jni_prefix}_destroy"
  "${jni_prefix}_interrupt"
)

usage() {
  echo 'Usage: build-android-runtime.sh host debug|release OUTPUT_DIRECTORY' >&2
  echo '       build-android-runtime.sh android ABI debug|release OUTPUT_DIRECTORY' >&2
  echo '       build-android-runtime.sh android-all debug|release OUTPUT_DIRECTORY' >&2
  exit 2
}

profile_arguments() {
  case "$1" in
    debug) profile='debug'; cargo_release=0 ;;
    release) profile='release'; cargo_release=1 ;;
    *) usage ;;
  esac
}

require_rust_toolchain() {
  command -v cargo >/dev/null && command -v rustc >/dev/null && command -v rustup >/dev/null || {
    echo "The repository-pinned Rust $rust_toolchain toolchain is unavailable." >&2
    exit 1
  }
}

audit_symbols() {
  local artifact="$1"
  local nm_command="$2"
  local actual expected
  actual="$($nm_command --defined-only --extern-only "$artifact" 2>/dev/null \
    | awk '{print $NF}' | sed 's/^_//' | rg "^${jni_prefix}_" | sort -u)"
  expected="$(printf '%s\n' "${symbols[@]}" | sort -u)"
  [[ "$actual" == "$expected" ]] || {
    echo "Unexpected application-runtime JNI symbols in $artifact:" >&2
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
    ndk_directory="$sdk_directory/ndk/$android_ndk_version"
  fi
  [[ -f "$ndk_directory/source.properties" ]] || {
    echo "Pinned Android NDK $android_ndk_version is unavailable at $ndk_directory." >&2
    exit 1
  }
  rg -q "^Pkg.Revision = $android_ndk_version$" "$ndk_directory/source.properties" || {
    echo "Android NDK at $ndk_directory is not pinned revision $android_ndk_version." >&2
    exit 1
  }
  local prebuilt_root="$ndk_directory/toolchains/llvm/prebuilt"
  shopt -s nullglob
  local prebuilts=("$prebuilt_root"/*)
  shopt -u nullglob
  [[ ${#prebuilts[@]} -eq 1 ]] || {
    echo "Expected exactly one NDK LLVM prebuilt beneath $prebuilt_root." >&2
    exit 1
  }
  llvm_bin="${prebuilts[0]}/bin"
}

build_host() {
  profile_arguments "$1"
  local output_directory="$2"
  require_rust_toolchain
  local host_target extension artifact destination
  host_target="$(rustc +"$rust_toolchain" -vV | sed -n 's/^host: //p')"
  case "$host_target" in
    *-apple-darwin) extension='dylib' ;;
    *-linux-*) extension='so' ;;
    *) echo "Unsupported host-JVM Rust target: $host_target" >&2; exit 1 ;;
  esac
  local cargo_arguments=(+"$rust_toolchain" build --manifest-path "$core_root/Cargo.toml" \
    --package "$package" --locked --offline)
  if [[ $cargo_release -eq 1 ]]; then cargo_arguments+=(--release); fi
  cargo "${cargo_arguments[@]}"
  artifact="$core_root/target/$profile/lib$library_stem.$extension"
  [[ -f "$artifact" ]] || { echo "Host runtime library was not produced: $artifact" >&2; exit 1; }
  mkdir -p "$output_directory"
  destination="$output_directory/lib$library_stem.$extension"
  install -m 0644 "$artifact" "$destination"
  audit_symbols "$destination" nm
  printf '%s\n' "$destination"
}

build_android() {
  local abi="$1"
  profile_arguments "$2"
  local output_directory="$3"
  local target compiler architecture target_environment cargo_linker artifact destination
  case "$abi" in
    arm64-v8a) target='aarch64-linux-android'; compiler="aarch64-linux-android${android_api}-clang"; architecture='ARM aarch64' ;;
    armeabi-v7a) target='armv7-linux-androideabi'; compiler="armv7a-linux-androideabi${android_api}-clang"; architecture='ARM, EABI5' ;;
    x86) target='i686-linux-android'; compiler="i686-linux-android${android_api}-clang"; architecture='Intel 80386' ;;
    x86_64) target='x86_64-linux-android'; compiler="x86_64-linux-android${android_api}-clang"; architecture='x86-64' ;;
    *) echo "Unsupported Android ABI: $abi" >&2; exit 2 ;;
  esac
  require_rust_toolchain
  resolve_ndk
  rustup target list --toolchain "$rust_toolchain" --installed | rg -Fxq "$target" || {
    echo "Rust target $target is unavailable for toolchain $rust_toolchain." >&2
    exit 1
  }
  local linker="$llvm_bin/$compiler"
  [[ -x "$linker" ]] || { echo "Pinned NDK linker is unavailable: $linker" >&2; exit 1; }
  target_environment="$(printf '%s' "$target" | tr '[:lower:]-' '[:upper:]_')"
  cargo_linker="CARGO_TARGET_${target_environment}_LINKER"
  local cargo_arguments=(+"$rust_toolchain" build \
    --manifest-path "$core_root/Cargo.toml" --package "$package" --target "$target" \
    --locked --offline)
  if [[ $cargo_release -eq 1 ]]; then cargo_arguments+=(--release); fi
  env "$cargo_linker=$linker" "CC_$target=$linker" "AR_$target=$llvm_bin/llvm-ar" \
    cargo "${cargo_arguments[@]}"
  artifact="$core_root/target/$target/$profile/lib$library_stem.so"
  [[ -f "$artifact" ]] || { echo "Android runtime library was not produced: $artifact" >&2; exit 1; }
  mkdir -p "$output_directory/$abi"
  destination="$output_directory/$abi/lib$library_stem.so"
  install -m 0644 "$artifact" "$destination"
  audit_symbols "$destination" "$llvm_bin/llvm-nm"
  file "$destination" | rg -Fq "$architecture" || {
    echo "Android runtime artifact has the wrong architecture for $abi: $destination" >&2
    exit 1
  }
  printf '%s\n' "$destination"
}

case "${1:-}" in
  host) [[ $# -eq 3 ]] || usage; build_host "$2" "$3" ;;
  android) [[ $# -eq 4 ]] || usage; build_android "$2" "$3" "$4" ;;
  android-all)
    [[ $# -eq 3 ]] || usage
    for abi in arm64-v8a armeabi-v7a x86 x86_64; do build_android "$abi" "$2" "$3"; done
    ;;
  *) usage ;;
esac
