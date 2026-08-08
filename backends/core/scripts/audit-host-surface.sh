#!/usr/bin/env bash
set -euo pipefail

root=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd)
cd "$root"

count_sources() {
    for source_root in "$@"; do
        if [[ -d "$source_root" ]]; then
            find "$source_root" -type f \( -name '*.swift' -o -name '*.kt' \) \
                ! -name 'resource_bundle_accessor.swift'
        fi
    done | wc -l | tr -d ' '
}

swift_production_count=$(count_sources \
    backends/apple/common backends/apple/macos backends/apple/ios/float-view)
swift_test_count=$(count_sources \
    backends/apple/Tests backends/apple/ios/float-viewTests)
kotlin_production_count=$(count_sources backends/android/app/src/main)
kotlin_test_count=$(count_sources \
    backends/android/app/src/test backends/android/app/src/androidTest)

printf 'host-surface swift production=%s test=%s\n' \
    "$swift_production_count" "$swift_test_count"
printf 'host-surface kotlin production=%s test=%s\n' \
    "$kotlin_production_count" "$kotlin_test_count"

capability_inventory=backends/HOST_CAPABILITIES.md
[[ -f "$capability_inventory" ]] || {
    echo 'native capability inventory is missing' >&2
    exit 1
}

required_entries=(
    FloatViewBluetooth
    FloatViewBackgroundExecution
    FloatViewLocation
    FloatViewDocuments
    FloatViewAppearance
    FloatViewLifecycle
    FloatViewWebView
    FloatViewRuntimeBridge
)

check_capability_host() {
    local label=$1
    local source_root=$2
    local readme=$3
    local extension=$4
    [[ -f "$readme" ]] || {
        echo "$label capability README is missing: $readme" >&2
        exit 1
    }
    for entry in "${required_entries[@]}"; do
        local path="$source_root/$entry.$extension"
        [[ -f "$path" ]] || {
            echo "$label capability entry is missing: $path" >&2
            exit 1
        }
        [[ $(find "$source_root" -type f -name "$entry.$extension" | wc -l | tr -d ' ') -eq 1 ]] || {
            echo "$label has duplicate $entry capability entries" >&2
            exit 1
        }
        rg -q 'Capability status: (implemented|unsupported|incomplete)' "$path" || {
            echo "$label $entry does not declare a valid capability status" >&2
            exit 1
        }
        rg -q "\`$entry\\.$extension\`" "$readme" || {
            echo "$label README does not inventory $entry.$extension" >&2
            exit 1
        }
        if rg -q 'Capability status: unsupported' "$path"; then
            rg -qi 'unavailable|not supported|unsupported' "$path" || {
                echo "$label $entry unsupported stub does not explain stable unavailability" >&2
                exit 1
            }
        else
            rg -q 'Concrete (OS owner|owner|owners|source)' "$path" || {
                echo "$label $entry does not name its concrete owner or source" >&2
                exit 1
            }
        fi
    done
}

check_capability_host ios backends/apple/ios/float-view backends/apple/ios/README.md swift
check_capability_host macos backends/apple/macos backends/apple/macos/README.md swift
check_capability_host android \
    backends/android/app/src/main/java/fail/acheron/floatview/host \
    backends/android/app/src/main/java/fail/acheron/floatview/host/README.md kt

for capability in \
    'Bluetooth transport' \
    'Background execution' \
    'Location' \
    'Native documents' \
    'System appearance' \
    'Lifecycle and WebView' \
    'Runtime bridge'; do
    matrix_count=$(awk -F '|' -v capability="$capability" '
        $0 == "## Current inventory" { current = 1; next }
        current && $0 ~ /^\|/ {
            value = $2
            gsub(/^[[:space:]]+|[[:space:]]+$/, "", value)
            if (value == capability) count += 1
        }
        END { print count + 0 }
    ' "$capability_inventory")
    [[ $matrix_count -eq 1 ]] || {
        echo "capability matrix must contain exactly one $capability row" >&2
        exit 1
    }
done

[[ ! -e backends/apple/Sources ]] || {
    echo 'obsolete backends/apple/Sources layout remains' >&2
    exit 1
}
for obsolete_bucket in \
    backends/apple/common/FloatViewBackend/application/PlatformPorts.swift \
    backends/apple/common/FloatViewBackend/application/RustCorePlatformAdapter.swift \
    backends/apple/common/FloatViewBackend/application/ApplePlatformHostServices.swift \
    backends/android/app/src/main/java/fail/acheron/floatview/backend/Domain.kt \
    backends/android/app/src/main/java/fail/acheron/floatview/core/RustAndroidPlatformAdapter.kt \
    backends/android/app/src/main/java/fail/acheron/floatview/core/AndroidPlatformHostServices.kt; do
    [[ ! -e "$obsolete_bucket" ]]
done

if [[ ${1:-} == '--final' ]] && rg -n '\bincomplete\b' \
    "$capability_inventory" backends/apple/ios/README.md backends/apple/macos/README.md \
    backends/android/app/src/main/java/fail/acheron/floatview/host/README.md \
    backends/apple/ios/float-view/FloatView*.swift backends/apple/macos/FloatView*.swift \
    backends/android/app/src/main/java/fail/acheron/floatview/host/FloatView*.kt; then
    echo 'final capability audit rejects incomplete entries' >&2
    exit 1
fi

for removed in \
    backends/apple/common/FloatViewBackend/bluetooth/SoftwareBluetoothAdapter.swift \
    backends/apple/common/FloatViewBackend/bluetooth/SoftwareVescPeripheral.swift \
    backends/android/app/src/main/java/fail/acheron/floatview/bluetooth/SoftwareBluetoothAdapter.kt \
    backends/android/app/src/main/java/fail/acheron/floatview/bluetooth/SoftwareVescPeripheral.kt; do
    [[ ! -e "$removed" ]] || { echo "superseded native software host remains: $removed" >&2; exit 1; }
done

native_roots=(
    backends/apple/common
    backends/apple/macos
    backends/apple/ios/float-view
    backends/android/app/src/main
)

reject() {
    local pattern=$1
    local message=$2
    if rg -n "$pattern" "${native_roots[@]}"; then
        echo "$message" >&2
        exit 1
    fi
}

reject 'Software(VescPeripheral|BluetoothAdapter)' 'native software transport reference remains'
reject 'configureSavedDeviceID' 'unreachable saved-device host API remains'
reject 'AndroidDeviceIdentity|installation-salt|opaqueIds' 'host-generated transport alias remains'
reject 'PlatformCommandEnvelope[^\n]*encode|FloatViewPlatformCommandEnvelope[^\n]*encode' \
    'reverse production command codec remains'
reject 'ReadResource|StageDocument|ResolveStorageRoot|AvailableSpace|MonotonicClock|WallClock|SecureRandom|PresentTheme|PresentServiceState|ResourceNamespace|StorageRootKind' \
    'unreachable host command or bridge model remains'

if rg -n -i 'firmware(device)?uuid|controlleruuid|opaqueplatformid|bleaddress|peripheraluuid' \
    frontend/src/lib/backend/contract.ts frontend/src/lib/backend/client.ts; then
    echo 'private transport or firmware identity leaked into the frontend contract' >&2
    exit 1
fi
if rg -n -i '(print|println|eprintln|log\.|trace\().*(firmware.*uuid|controller.*uuid|association|deviceid\.value)' \
    backends/core/crates/app-backend/src backends/core/crates/app-coordinator/src "${native_roots[@]}"; then
    echo 'ordinary diagnostics interpolate a private transport or firmware identity' >&2
    exit 1
fi
if rg -n 'firmware_device_uuid|opaque_platform_id' \
    backends/core/crates/infra-sqlite-storage/src/archive.rs; then
    echo 'private saved identity entered archive/export construction' >&2
    exit 1
fi

manifest=backends/core/HOST_COMMANDS.tsv
[[ $(awk 'NR > 1 { count += 1 } END { print count + 0 }' "$manifest") -eq 13 ]]
awk -F '\t' 'NR > 1 {
    if ($1 != NR - 1 || $3 == "" || $4 == "" || $5 == "" || $6 !~ /<=64KiB/ || $7 == "") exit 1
}' "$manifest"

for kind in $(seq 1 13); do
    rg -q "COMMAND_[A-Z_]+: u32 = $kind;" backends/core/crates/host-ports/src/lib.rs
    rg -q "case $kind:" backends/apple/common/FloatViewCoreBridge/FloatViewPlatformBridgeCodec.swift
done
rg -q 'in 6\.\.13' backends/android/app/src/main/java/fail/acheron/floatview/core/PlatformBridgeCodec.kt
for kind in $(seq 6 13); do
    rg -q "$kind ->" backends/android/app/src/main/java/fail/acheron/floatview/core/PlatformBridgeCodec.kt
done

echo 'host command reachability: 13/13 retained kinds accounted for'
