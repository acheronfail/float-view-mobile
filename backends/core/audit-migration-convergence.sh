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

selected_families="$(awk '
  /^bridge_family\|/ { count++ }
  END { print count+0 }
' backends/core/migration-baseline.txt)"

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
selected_protocol_families=$selected_families
apple_bridge_family_artifacts=$apple_bridge_artifacts
android_bridge_crates=$android_bridge_artifacts
android_runtime_target_artifacts=$((android_bridge_artifacts * 4 * 2))
EOF

[[ "$binding_crates" == 2 ]]
[[ "$temporary_bridge_crates" == 0 ]]
[[ "$apple_bridge_artifacts" == 1 ]]
[[ "$android_bridge_artifacts" == 1 ]]
[[ "$selected_families" == 10 ]]
[[ "$members" -lt 34 ]]
((rust_lines + apple_lines + android_lines < 59280))

expected_candidates='application-runtime auxiliary-node-discovery connection-application-coordinator controller-discovery device-mutation-coordinator refloat-configuration-service refloat-telemetry-service session-history-kernel session-sqlite vesc-bms-telemetry-service vesc-compatibility vesc-request-scheduler'
actual_candidates="$(awk -F'|' '/^candidate\|/ { print $2 }' backends/core/migration-baseline.txt | sort | xargs)"
[[ "$actual_candidates" == "$expected_candidates" ]]
