#!/usr/bin/env bash
set -euo pipefail

root=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd)
cd "$root"

count_lines() {
    if [[ $# -eq 0 ]]; then
        echo 0
    else
        wc -l "$@" | tail -1 | awk '{ print $1 }'
    fi
}

mapfile -t swift_production < <(find backends/apple/Sources backends/apple/ios/float-view \
    -type f -name '*.swift' ! -name 'resource_bundle_accessor.swift' | sort)
mapfile -t swift_tests < <(find backends/apple/Tests backends/apple/ios/float-viewTests \
    -type f -name '*.swift' 2>/dev/null | sort)
mapfile -t kotlin_production < <(find backends/android/app/src/main -type f -name '*.kt' | sort)
mapfile -t kotlin_tests < <(find backends/android/app/src/test backends/android/app/src/androidTest \
    -type f -name '*.kt' 2>/dev/null | sort)

printf 'host-surface swift production=%s test=%s\n' \
    "$(count_lines "${swift_production[@]}")" "$(count_lines "${swift_tests[@]}")"
printf 'host-surface kotlin production=%s test=%s\n' \
    "$(count_lines "${kotlin_production[@]}")" "$(count_lines "${kotlin_tests[@]}")"

for removed in \
    backends/apple/Sources/FloatViewBackend/bluetooth/SoftwareBluetoothAdapter.swift \
    backends/apple/Sources/FloatViewBackend/bluetooth/SoftwareVescPeripheral.swift \
    backends/android/app/src/main/java/fail/acheron/floatview/bluetooth/SoftwareBluetoothAdapter.kt \
    backends/android/app/src/main/java/fail/acheron/floatview/bluetooth/SoftwareVescPeripheral.kt; do
    [[ ! -e "$removed" ]] || { echo "superseded native software host remains: $removed" >&2; exit 1; }
done

native_roots=(
    backends/apple/Sources
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
reject 'ReadResource|StageDocument|ResolveStorageRoot|AvailableSpace|MonotonicClock|WallClock|SecureRandom|PresentTheme|PresentServiceState|PresentExport|ResourceNamespace|StorageRootKind' \
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
[[ $(awk 'NR > 1 { count += 1 } END { print count + 0 }' "$manifest") -eq 11 ]]
awk -F '\t' 'NR > 1 {
    if ($1 != NR - 1 || $3 == "" || $4 == "" || $5 == "" || $6 !~ /<=64KiB/ || $7 == "") exit 1
}' "$manifest"

for kind in $(seq 1 11); do
    rg -q "COMMAND_[A-Z_]+: u32 = $kind;" backends/core/crates/host-ports/src/lib.rs
    rg -q "case $kind:" backends/apple/Sources/FloatViewCoreBridge/FloatViewPlatformBridgeCodec.swift
done
rg -q 'in 6\.\.11' backends/android/app/src/main/java/fail/acheron/floatview/core/PlatformBridgeCodec.kt
for kind in $(seq 6 11); do
    rg -q "$kind ->" backends/android/app/src/main/java/fail/acheron/floatview/core/PlatformBridgeCodec.kt
done

echo 'host command reachability: 11/11 retained kinds accounted for'
