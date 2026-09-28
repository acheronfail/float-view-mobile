#!/usr/bin/env bash
set -euo pipefail

workspace_root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$workspace_root"

count_lines() {
  local root="$1"
  shift
  local files=()
  while IFS= read -r file; do
    files+=("$file")
  done < <(
    git ls-files --cached --others --exclude-standard "$root" |
      grep -E '\.(rs|swift|kt|kts|c|h)$' |
      grep -Ev '(^|/)(Tests?|tests?|build|target)/|Test\.kt$|Tests\.swift$' |
      while IFS= read -r file; do
        [[ -f "$file" ]] && printf '%s\n' "$file"
      done || true
  )
  if ((${#files[@]} == 0)); then
    echo 0
    return
  fi
  wc -l "${files[@]}" | tail -n 1 | awk '{print $1}'
}

members="$(awk '
  /^members = \[/ { inside=1; next }
  inside && /^\]/ { inside=0; next }
  inside && /"/ { count++ }
  END { print count+0 }
' backends/core/Cargo.toml)"

binding_crates="$(find backends/core -mindepth 2 -maxdepth 2 \
  \( -path '*-apple-ffi/Cargo.toml' -o -path '*-android-jni/Cargo.toml' \) \
  | wc -l | tr -d ' ')"
temporary_bridge_crates="$(find backends/core -mindepth 2 -maxdepth 2 \
  \( -path '*-apple-ffi/Cargo.toml' -o -path '*-android-jni/Cargo.toml' \) \
  ! -path '*/application-runtime-*' | wc -l | tr -d ' ')"
apple_bridge_artifacts="$(find backends/core -mindepth 2 -maxdepth 2 \
  -path '*-apple-ffi/Cargo.toml' | wc -l | tr -d ' ')"
android_bridge_artifacts="$(find backends/core -mindepth 2 -maxdepth 2 \
  -path '*-android-jni/Cargo.toml' | wc -l | tr -d ' ')"

rust_lines="$(count_lines backends/core)"
apple_lines="$(count_lines backends/apple)"
android_lines="$(count_lines backends/android)"

cat <<EOF
rust_production_source_lines=$rust_lines
apple_production_source_lines=$apple_lines
android_production_source_lines=$android_lines
workspace_members=$members
host_binding_crates=$binding_crates
temporary_protocol_bridge_crates=$temporary_bridge_crates
apple_bridge_family_artifacts=$apple_bridge_artifacts
android_bridge_crates=$android_bridge_artifacts
android_runtime_target_artifacts=$((android_bridge_artifacts * 4 * 2))
EOF

[[ "$binding_crates" == 2 ]]
[[ "$temporary_bridge_crates" == 0 ]]
[[ "$apple_bridge_artifacts" == 1 ]]
[[ "$android_bridge_artifacts" == 1 ]]
[[ "$members" -le 28 ]]
((rust_lines + apple_lines + android_lines < 49922))

# Native code owns OS capabilities; product SQL and dispatch stay shared.
! rg -n 'SQLiteLogStore|LogSchema|class ApplicationService|class ServerOperations|class ServerFacts|enum ServerFacts|class RuntimeRecords' \
  backends/apple/Sources backends/apple/macos backends/apple/ios/float-view backends/android/app/src/main
! rg -n 'sqlite3_|android.database.sqlite|CREATE TABLE|session_catalog' \
  backends/apple/Sources backends/apple/macos backends/apple/ios/float-view backends/android/app/src/main -g '*.swift' -g '*.kt'
[[ ! -e backends/core/loopback-server/src/exchange.rs ]]
[[ ! -e backends/core/loopback-server/src/completion.rs ]]
! rg -n 'CommandEnvelope|pub fn create\(\)|pub fn apply\(handle' backends/core/application-runtime-host/src/lib.rs
rg -q 'SharedService\(' backends/apple/ios/float-view/application/AppBackend.swift
rg -q 'SharedService\(' backends/apple/macos/application/MacBackendRuntime.swift
rg -q 'SharedService\(' backends/android/app/src/main/java/fail/acheron/floatview/application/BackendRuntime.kt
