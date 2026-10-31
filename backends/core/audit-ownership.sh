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

fail() {
  echo "Ownership audit: $*" >&2
  exit 1
}

# Negated commands and compound assertions do not reliably trigger errexit on
# Bash 3.2. Handle both matches and search errors explicitly.
reject_matches() {
  local result=0
  rg -n "$@" || result=$?
  case "$result" in
    0) fail "Forbidden source or artifact matched the pattern above." ;;
    1) ;;
    *) fail "Source search failed (rg exit $result)." ;;
  esac
}

[[ "$binding_crates" == 2 ]] || fail "Expected exactly two host binding crates."
[[ "$temporary_bridge_crates" == 0 ]] || fail "Temporary protocol bindings remain."
[[ "$apple_bridge_artifacts" == 1 ]] || fail "Expected one Apple binding family."
[[ "$android_bridge_artifacts" == 1 ]] || fail "Expected one Android binding family."
# Source size and workspace counts describe the current tree; migration-era
# ceilings are not ongoing feature-development gates.

for path in \
  backends/core/application-runtime-host/Cargo.toml \
  backends/core/application-runtime-apple-ffi/Cargo.toml \
  backends/core/application-runtime-android-jni/Cargo.toml; do
  [[ -f "$path" ]] || fail "Missing runtime boundary: $path"
done

for path in \
  backends/core/bridge-build-catalog.sh \
  backends/core/build-android-bridge.sh \
  backends/core/loopback-server/src/exchange.rs \
  backends/core/loopback-server/src/completion.rs; do
  [[ ! -e "$path" ]] || fail "Superseded runtime path remains: $path"
done

for file in AdaptiveLogging AuxiliaryNodeDiscovery ControllerDiscovery \
  RefloatConfigurationService RefloatMutationCoordinator TelemetryPolling VescRequestScheduler; do
  path="backends/android/app/src/main/java/fail/acheron/floatview/application/$file.kt"
  [[ ! -e "$path" ]] || fail "Superseded native owner remains: $path"
done

modules="$(find backends/apple/Sources -mindepth 1 -maxdepth 1 -type d -name 'CVesc*' -print)"
[[ -z "$modules" ]] || fail "Temporary Apple C modules remain: $modules"
git ls-files | reject_matches '\.(a|so|dylib|xcframework)$'
reject_matches 'Rust(Vesc|Refloat).*Bridge|float_view_(vesc|refloat)_.*android_jni|android-jni-support' \
  backends/android backends/core/Cargo.toml backends/core/Cargo.lock
reject_matches 'VescProtocolPort|VescProtocolService' backends/android/app/src/main -g '*.kt'
reject_matches 'FlyingFox|io\.ktor|libs\.ktor|ktor-server' backends/apple backends/android \
  -g '*.swift' -g '*.kt' -g '*.kts' -g '*.toml' -g '*.pbxproj' -g 'Package.resolved'
reject_matches 'case "state.get"|"state.get" ->|GZIPOutputStream|SessionHistoryReducer' \
  backends/apple/Sources backends/android/app/src/main

# Native code owns OS capabilities; product SQL and dispatch stay shared.
reject_matches 'SQLiteLogStore|LogSchema|class ApplicationService|class ServerOperations|class ServerFacts|enum ServerFacts|class RuntimeRecords' \
  backends/apple/Sources backends/apple/macos backends/apple/ios/float-view backends/android/app/src/main
reject_matches 'sqlite3_|android.database.sqlite|CREATE TABLE|session_catalog' \
  backends/apple/Sources backends/apple/macos backends/apple/ios/float-view backends/android/app/src/main -g '*.swift' -g '*.kt'
reject_matches 'CommandEnvelope|pub fn create\(\)|pub fn apply\(handle' backends/core/application-runtime-host/src/lib.rs
for path in \
  backends/apple/ios/float-view/application/AppBackend.swift \
  backends/apple/macos/application/MacBackendRuntime.swift \
  backends/android/app/src/main/java/fail/acheron/floatview/application/BackendRuntime.kt; do
  rg -q 'SharedService\(' "$path" || fail "Missing shared service construction: $path"
done

# BLE policy and logical buffering belong to the shared service.
reject_matches 'VescBluetoothPolicy|pendingWrites|pendingWriteBytes|drainWriteQueue|scheduleSetupDeadline|scheduleConnectionTimeout' \
  backends/apple/Sources backends/android/app/src/main -g '*.swift' -g '*.kt'
