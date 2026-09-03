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
      grep -Ev '(^|/)(Tests?|tests?|build|target)/|Test\.kt$|Tests\.swift$' || true
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

bridge_crates="$(find backends/core -mindepth 1 -maxdepth 1 -type d \
  \( -name '*-apple-ffi' -o -name '*-android-jni' \) | wc -l | tr -d ' ')"

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
temporary_protocol_bridge_crates=$bridge_crates
selected_protocol_families=$selected_families
apple_bridge_family_artifacts=$selected_families
android_bridge_target_artifacts=$((selected_families * 4 * 2))
EOF

[[ "$bridge_crates" == 20 ]]
[[ "$selected_families" == 10 ]]
[[ "$members" == 43 || "$members" == 44 ]]

expected_candidates='application-runtime auxiliary-node-discovery connection-application-coordinator controller-discovery device-mutation-coordinator refloat-configuration-service refloat-telemetry-service session-history-kernel session-sqlite vesc-bms-telemetry-service vesc-compatibility vesc-request-scheduler'
actual_candidates="$(awk -F'|' '/^candidate\|/ { print $2 }' backends/core/migration-baseline.txt | sort | xargs)"
[[ "$actual_candidates" == "$expected_candidates" ]]
