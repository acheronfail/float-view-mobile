#!/usr/bin/env bash
set -euo pipefail

core_root="$(cd "$(dirname "$0")/../.." && pwd)"
target_dir="$core_root/target"

cargo build --manifest-path "$core_root/Cargo.toml" --locked \
  --package float-view-native-abi --features abi-test-hooks

cc -std=c11 -Wall -Wextra -Werror -pthread \
  -I "$core_root/include" \
  "$core_root/tests/c/abi_harness.c" \
  "$target_dir/debug/libfloat_view_native_abi.a" \
  -framework CoreFoundation -framework Security -liconv -lSystem -lm \
  -o "$target_dir/abi-harness"

"$target_dir/abi-harness"
