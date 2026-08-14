_default:
    @just --list

core-lint:
    cd backends/core && cargo fmt --check
    cd backends/core && cargo clippy --locked --offline --workspace --all-targets -- -D warnings

rust-bridge-build-tooling-audit:
    #!/usr/bin/env bash
    set -euo pipefail
    catalog='backends/core/bridge-build-catalog.sh'
    apple_helper='backends/core/build-apple-bridge.sh'
    android_helper='backends/core/build-android-bridge.sh'
    android_gradle='backends/android/app/build.gradle.kts'
    bash -n "$catalog" "$apple_helper" "$android_helper"
    [[ ! -x "$catalog" && -x "$apple_helper" && -x "$android_helper" ]]
    [[ "$(find backends/core -type f \( -name 'build-apple*.sh' -o -name 'build-android*.sh' \) | sort)" == \
      $'backends/core/build-android-bridge.sh\nbackends/core/build-apple-bridge.sh' ]]
    ! find backends/core -mindepth 2 -type f \
      \( -name 'build-apple.sh' -o -name 'build-android.sh' \) | grep -q .
    ! rg -n 'backends/core/vesc-[^[:space:]]+/(build-apple|build-android)\.sh' \
      justfile backends conformance
    expected=$'frame|float-view-vesc-frame-apple-ffi|libfloat_view_vesc_frame_apple_ffi.a|float-view-vesc-frame-android-jni|float_view_vesc_frame_android_jni|Java_fail_acheron_floatview_protocol_RustVescFrameBridge|Java_fail_acheron_floatview_protocol_RustVescFrameBridge_abiVersion Java_fail_acheron_floatview_protocol_RustVescFrameBridge_decodeExact Java_fail_acheron_floatview_protocol_RustVescFrameBridge_encode Java_fail_acheron_floatview_protocol_RustVescFrameBridge_scanNext\nfirmware-identity|float-view-vesc-firmware-identity-apple-ffi|libfloat_view_vesc_firmware_identity_apple_ffi.a|float-view-vesc-firmware-identity-android-jni|float_view_vesc_firmware_identity_android_jni|Java_fail_acheron_floatview_protocol_RustVescFirmwareIdentityBridge|Java_fail_acheron_floatview_protocol_RustVescFirmwareIdentityBridge_abiVersion Java_fail_acheron_floatview_protocol_RustVescFirmwareIdentityBridge_decode\ncustom-configuration|float-view-vesc-custom-configuration-apple-ffi|libfloat_view_vesc_custom_configuration_apple_ffi.a|float-view-vesc-custom-configuration-android-jni|float_view_vesc_custom_configuration_android_jni|Java_fail_acheron_floatview_protocol_RustVescCustomConfigurationBridge|Java_fail_acheron_floatview_protocol_RustVescCustomConfigurationBridge_abiVersion Java_fail_acheron_floatview_protocol_RustVescCustomConfigurationBridge_decode Java_fail_acheron_floatview_protocol_RustVescCustomConfigurationBridge_encode\nbms-values|float-view-vesc-bms-values-apple-ffi|libfloat_view_vesc_bms_values_apple_ffi.a|float-view-vesc-bms-values-android-jni|float_view_vesc_bms_values_android_jni|Java_fail_acheron_floatview_protocol_RustVescBmsValuesBridge|Java_fail_acheron_floatview_protocol_RustVescBmsValuesBridge_abiVersion Java_fail_acheron_floatview_protocol_RustVescBmsValuesBridge_decode\ncontroller-data|float-view-vesc-controller-data-apple-ffi|libfloat_view_vesc_controller_data_apple_ffi.a|float-view-vesc-controller-data-android-jni|float_view_vesc_controller_data_android_jni|Java_fail_acheron_floatview_protocol_RustVescControllerDataBridge|Java_fail_acheron_floatview_protocol_RustVescControllerDataBridge_abiVersion Java_fail_acheron_floatview_protocol_RustVescControllerDataBridge_decode\nrefloat-control|float-view-refloat-control-apple-ffi|libfloat_view_refloat_control_apple_ffi.a|float-view-refloat-control-android-jni|float_view_refloat_control_android_jni|Java_fail_acheron_floatview_protocol_RustRefloatControlBridge|Java_fail_acheron_floatview_protocol_RustRefloatControlBridge_abiVersion Java_fail_acheron_floatview_protocol_RustRefloatControlBridge_decodeLighting Java_fail_acheron_floatview_protocol_RustRefloatControlBridge_encode\nrefloat-legacy-telemetry|float-view-refloat-legacy-telemetry-apple-ffi|libfloat_view_refloat_legacy_telemetry_apple_ffi.a|float-view-refloat-legacy-telemetry-android-jni|float_view_refloat_legacy_telemetry_android_jni|Java_fail_acheron_floatview_protocol_RustRefloatLegacyTelemetryBridge|Java_fail_acheron_floatview_protocol_RustRefloatLegacyTelemetryBridge_abiVersion Java_fail_acheron_floatview_protocol_RustRefloatLegacyTelemetryBridge_decode'
    legacy_catalog_row="${expected##*$'\n'}"
    expected="${expected%$'\n'*}"$'\nrefloat-dynamic-telemetry|float-view-refloat-dynamic-telemetry-apple-ffi|libfloat_view_refloat_dynamic_telemetry_apple_ffi.a|float-view-refloat-dynamic-telemetry-android-jni|float_view_refloat_dynamic_telemetry_android_jni|Java_fail_acheron_floatview_protocol_RustRefloatDynamicTelemetryBridge|Java_fail_acheron_floatview_protocol_RustRefloatDynamicTelemetryBridge_abiVersion Java_fail_acheron_floatview_protocol_RustRefloatDynamicTelemetryBridge_decodeLayout Java_fail_acheron_floatview_protocol_RustRefloatDynamicTelemetryBridge_decodeRealtime\n'"$legacy_catalog_row"
    expected+=$'\nrefloat-compatibility|float-view-refloat-compatibility-apple-ffi|libfloat_view_refloat_compatibility_apple_ffi.a|float-view-refloat-compatibility-android-jni|float_view_refloat_compatibility_android_jni|Java_fail_acheron_floatview_protocol_RustRefloatCompatibilityBridge|Java_fail_acheron_floatview_protocol_RustRefloatCompatibilityBridge_abiVersion Java_fail_acheron_floatview_protocol_RustRefloatCompatibilityBridge_admitLayout Java_fail_acheron_floatview_protocol_RustRefloatCompatibilityBridge_decodeInfo Java_fail_acheron_floatview_protocol_RustRefloatCompatibilityBridge_infoRequest'
    expected+=$'\nrefloat-configuration-codec|float-view-refloat-configuration-codec-apple-ffi|libfloat_view_refloat_configuration_codec_apple_ffi.a|float-view-refloat-configuration-codec-android-jni|float_view_refloat_configuration_codec_android_jni|Java_fail_acheron_floatview_protocol_refloat_RustRefloatConfigurationCodecBridge|Java_fail_acheron_floatview_protocol_refloat_RustRefloatConfigurationCodecBridge_abiVersion Java_fail_acheron_floatview_protocol_refloat_RustRefloatConfigurationCodecBridge_decodeSchema Java_fail_acheron_floatview_protocol_refloat_RustRefloatConfigurationCodecBridge_decodeValues Java_fail_acheron_floatview_protocol_refloat_RustRefloatConfigurationCodecBridge_encodeValues'
    source "$catalog"
    actual="$(for bridge in frame firmware-identity custom-configuration bms-values controller-data refloat-control refloat-dynamic-telemetry refloat-legacy-telemetry refloat-compatibility refloat-configuration-codec; do
      float_view_select_bridge "$bridge"
      printf '%s|%s|%s|%s|%s|%s|%s\n' \
        "$bridge_identifier" "$bridge_apple_package" "$bridge_apple_archive" \
        "$bridge_android_package" "$bridge_android_library_stem" \
        "$bridge_android_jni_prefix" "${bridge_android_symbols[*]}"
    done)"
    [[ "$actual" == "$expected" ]]
    catalog_identifiers="$(for bridge in frame firmware-identity custom-configuration bms-values controller-data refloat-control refloat-compatibility refloat-dynamic-telemetry refloat-legacy-telemetry refloat-configuration-codec; do
      float_view_select_bridge "$bridge"
      printf '%s\n' "$bridge_identifier"
    done)"
    gradle_identifiers="$(sed -n 's/^        identifier = "\([^"]*\)",$/\1/p' "$android_gradle")"
    [[ "$gradle_identifiers" == "$catalog_identifiers" ]]
    float_view_select_bridge refloat-configuration-codec
    [[ "$bridge_identifier" == refloat-configuration-codec ]]
    [[ "$bridge_apple_package" == float-view-refloat-configuration-codec-apple-ffi ]]
    [[ "$bridge_apple_archive" == libfloat_view_refloat_configuration_codec_apple_ffi.a ]]
    [[ "$bridge_android_package" == float-view-refloat-configuration-codec-android-jni ]]
    [[ "$bridge_android_library_stem" == float_view_refloat_configuration_codec_android_jni ]]
    [[ "$bridge_android_jni_prefix" == Java_fail_acheron_floatview_protocol_refloat_RustRefloatConfigurationCodecBridge ]]
    [[ ${#bridge_android_symbols[@]} -eq 4 ]]
    [[ "$(rg -c '^data class RustAndroidBridgeSpec\(' "$android_gradle")" == 1 ]]
    [[ "$(rg -c '^    RustAndroidBridgeSpec\(' "$android_gradle")" == 10 ]]
    [[ "$(rg -c '^fun registerAndroidRust(Package|Host)Build\(' "$android_gradle")" == 2 ]]
    [[ "$(rg -c '^fun Exec\.registerRustBridgeInputs\(' "$android_gradle")" == 1 ]]
    [[ "$(rg -c 'inputs\.files\(' "$android_gradle")" == 1 ]]
    [[ "$(rg -c 'jniLibs\.srcDir\(' "$android_gradle")" == 2 ]]
    [[ "$(rg -c 'systemProperty\(spec\.testLibraryProperty' "$android_gradle")" == 1 ]]
    ! rg -n 'registerAndroidRust(Identity|CustomConfiguration|BmsValues|ControllerData|RefloatControl|RefloatCompatibility|RefloatDynamicTelemetry|RefloatLegacyTelemetry|RefloatConfigurationCodec)Build|val buildAndroidRust[A-Z]' \
      "$android_gradle"
    backends/android/gradlew -p backends/android --offline \
      :app:auditAndroidRustBridgeTasks
    audit_root="$(mktemp -d /private/tmp/float-view-bridge-tooling-audit.XXXXXX)"
    for command in \
      "$apple_helper unknown macosx arm64 Debug $audit_root/apple-unknown" \
      "$apple_helper frame invalid arm64 Debug $audit_root/apple-sdk" \
      "$apple_helper frame macosx '' Debug $audit_root/apple-arch" \
      "$apple_helper frame macosx arm64 Invalid $audit_root/apple-profile" \
      "$android_helper unknown host debug $audit_root/android-unknown" \
      "$android_helper frame invalid debug $audit_root/android-mode" \
      "$android_helper frame host invalid $audit_root/android-profile" \
      "$android_helper frame android invalid debug $audit_root/android-abi"; do
      if eval "$command" >/dev/null 2>&1; then
        echo "Rust bridge helper accepted invalid input: $command" >&2
        exit 1
      fi
    done
    [[ -z "$(find "$audit_root" -mindepth 1 -print -quit)" ]]
    for helper_command in \
      "$apple_helper frame macosx arm64 Debug $audit_root/apple-toolchain" \
      "$android_helper frame host debug $audit_root/android-toolchain"; do
      if env PATH=/usr/bin:/bin $helper_command >/dev/null 2>&1; then
        echo "Rust bridge helper accepted a missing pinned toolchain: $helper_command" >&2
        exit 1
      fi
    done
    [[ -z "$(find "$audit_root" -mindepth 1 -print -quit)" ]]
    if ANDROID_NDK_HOME="$audit_root/missing-ndk" \
      "$android_helper" frame android arm64-v8a debug "$audit_root/android-ndk" \
      >/dev/null 2>&1; then
      echo 'Shared Android Rust helper accepted a missing pinned NDK.' >&2
      exit 1
    fi
    [[ ! -e "$audit_root/android-ndk" ]]

apple-rust-codec configuration="Debug":
    #!/usr/bin/env bash
    set -euo pipefail
    backends/core/build-apple-bridge.sh frame \
      macosx "$(uname -m)" "{{ configuration }}" \
      "$PWD/backends/core/target/apple/macos"

apple-rust-firmware-identity configuration="Debug":
    #!/usr/bin/env bash
    set -euo pipefail
    backends/core/build-apple-bridge.sh firmware-identity \
      macosx "$(uname -m)" "{{ configuration }}" \
      "$PWD/backends/core/target/apple/macos"

apple-rust-custom-configuration configuration="Debug":
    #!/usr/bin/env bash
    set -euo pipefail
    backends/core/build-apple-bridge.sh custom-configuration \
      macosx "$(uname -m)" "{{ configuration }}" \
      "$PWD/backends/core/target/apple/macos"

apple-rust-bms-values configuration="Debug":
    #!/usr/bin/env bash
    set -euo pipefail
    backends/core/build-apple-bridge.sh bms-values \
      macosx "$(uname -m)" "{{ configuration }}" \
      "$PWD/backends/core/target/apple/macos"

apple-rust-controller-data configuration="Debug":
    #!/usr/bin/env bash
    set -euo pipefail
    backends/core/build-apple-bridge.sh controller-data \
      macosx "$(uname -m)" "{{ configuration }}" \
      "$PWD/backends/core/target/apple/macos"

apple-rust-refloat-control configuration="Debug":
    #!/usr/bin/env bash
    set -euo pipefail
    backends/core/build-apple-bridge.sh refloat-control \
      macosx "$(uname -m)" "{{ configuration }}" \
      "$PWD/backends/core/target/apple/macos"

apple-rust-refloat-compatibility configuration="Debug":
    #!/usr/bin/env bash
    set -euo pipefail
    backends/core/build-apple-bridge.sh refloat-compatibility \
      macosx "$(uname -m)" "{{ configuration }}" \
      "$PWD/backends/core/target/apple/macos"

apple-rust-refloat-dynamic-telemetry configuration="Debug":
    #!/usr/bin/env bash
    set -euo pipefail
    backends/core/build-apple-bridge.sh refloat-dynamic-telemetry \
      macosx "$(uname -m)" "{{ configuration }}" \
      "$PWD/backends/core/target/apple/macos"

apple-rust-refloat-legacy-telemetry configuration="Debug":
    #!/usr/bin/env bash
    set -euo pipefail
    backends/core/build-apple-bridge.sh refloat-legacy-telemetry \
      macosx "$(uname -m)" "{{ configuration }}" \
      "$PWD/backends/core/target/apple/macos"

apple-rust-refloat-configuration-codec configuration="Debug":
    #!/usr/bin/env bash
    set -euo pipefail
    backends/core/build-apple-bridge.sh refloat-configuration-codec \
      macosx "$(uname -m)" "{{ configuration }}" \
      "$PWD/backends/core/target/apple/macos"

apple-rust-bridges configuration="Debug":
    just apple-rust-codec "{{ configuration }}"
    just apple-rust-firmware-identity "{{ configuration }}"
    just apple-rust-custom-configuration "{{ configuration }}"
    just apple-rust-bms-values "{{ configuration }}"
    just apple-rust-controller-data "{{ configuration }}"
    just apple-rust-refloat-compatibility "{{ configuration }}"
    just apple-rust-refloat-control "{{ configuration }}"
    just apple-rust-refloat-dynamic-telemetry "{{ configuration }}"
    just apple-rust-refloat-legacy-telemetry "{{ configuration }}"
    just apple-rust-refloat-configuration-codec "{{ configuration }}"

core-abi-test: apple-rust-codec
    xcrun clang -std=c17 -Wall -Wextra -Werror \
      backends/core/vesc-frame-apple-ffi/tests/abi.c \
      -I backends/apple/Sources/CVescFrame/include \
      -L backends/core/target/apple/macos \
      -lfloat_view_vesc_frame_apple_ffi \
      -o /tmp/float-view-vesc-frame-abi-test
    /tmp/float-view-vesc-frame-abi-test

core-abi-audit: apple-rust-codec
    #!/usr/bin/env bash
    set -euo pipefail
    archive='backends/core/target/apple/macos/libfloat_view_vesc_frame_apple_ffi.a'
    actual="$({ nm -gjU "$archive" 2>/dev/null || true; } \
      | sed 's/^_//' | rg '^float_view_[a-z_]+$' | sort -u)"
    expected="$(printf '%s\n' \
      float_view_vesc_frame_abi_version \
      float_view_vesc_frame_decode_exact \
      float_view_vesc_frame_encode \
      float_view_vesc_frame_stream_scan_next)"
    [[ "$actual" == "$expected" ]] || {
      echo 'Unexpected Float View frame-codec ABI symbols:' >&2
      printf '%s\n' "$actual" >&2
      exit 1
    }
    rust_abi="$(sed -n 's/^pub const ABI_VERSION: u32 = \([0-9][0-9]*\);$/\1/p' \
      backends/core/vesc-frame-apple-ffi/src/lib.rs)"
    header_abi="$(sed -n 's/^#define FLOAT_VIEW_VESC_FRAME_ABI_VERSION UINT32_C(\([0-9][0-9]*\))$/\1/p' \
      backends/apple/Sources/CVescFrame/include/CVescFrame.h)"
    [[ -n "$rust_abi" && "$rust_abi" == "$header_abi" ]] || {
      echo "VESC frame-codec ABI version mismatch: Rust=$rust_abi header=$header_abi" >&2
      exit 1
    }

core-identity-abi-test: apple-rust-firmware-identity
    xcrun clang -std=c17 -Wall -Wextra -Werror \
      backends/core/vesc-firmware-identity-apple-ffi/tests/abi.c \
      -I backends/apple/Sources/CVescFirmwareIdentity/include \
      -L backends/core/target/apple/macos \
      -lfloat_view_vesc_firmware_identity_apple_ffi \
      -o /tmp/float-view-vesc-firmware-identity-abi-test
    /tmp/float-view-vesc-firmware-identity-abi-test

core-refloat-dynamic-telemetry-abi-test: apple-rust-refloat-dynamic-telemetry
    xcrun clang -std=c17 -Wall -Wextra -Werror \
      backends/core/refloat-dynamic-telemetry-apple-ffi/tests/abi.c \
      -I backends/apple/Sources/CVescRefloatDynamicTelemetry/include \
      -L backends/core/target/apple/macos \
      -lfloat_view_refloat_dynamic_telemetry_apple_ffi \
      -o /tmp/float-view-refloat-dynamic-telemetry-abi-test
    /tmp/float-view-refloat-dynamic-telemetry-abi-test

core-refloat-compatibility-abi-test: apple-rust-refloat-compatibility
    xcrun clang -std=c17 -Wall -Wextra -Werror \
      backends/core/refloat-compatibility-apple-ffi/tests/abi.c \
      -I backends/apple/Sources/CVescRefloatCompatibility/include \
      -L backends/core/target/apple/macos \
      -lfloat_view_refloat_compatibility_apple_ffi \
      -o /tmp/float-view-refloat-compatibility-abi-test
    /tmp/float-view-refloat-compatibility-abi-test

core-refloat-compatibility-abi-audit: apple-rust-refloat-compatibility
    #!/usr/bin/env bash
    set -euo pipefail
    archive='backends/core/target/apple/macos/libfloat_view_refloat_compatibility_apple_ffi.a'
    actual="$({ nm -gjU "$archive" 2>/dev/null || true; } \
      | sed 's/^_//' | rg '^float_view_[a-z_]+$' | sort -u)"
    expected="$(printf '%s\n' \
      float_view_refloat_compatibility_abi_version \
      float_view_refloat_compatibility_admit_layout \
      float_view_refloat_compatibility_decode_info \
      float_view_refloat_compatibility_info_request)"
    [[ "$actual" == "$expected" ]]
    rg -Fq 'pub const ABI_VERSION: u32 = u32::from_be_bytes([0, 0, 0, RECORD_VERSION]);' \
      backends/core/refloat-compatibility-apple-ffi/src/lib.rs
    rg -q '^#define FLOAT_VIEW_REFLOAT_COMPATIBILITY_ABI_VERSION 1u$' \
      backends/apple/Sources/CVescRefloatCompatibility/include/CVescRefloatCompatibility.h
    [[ "$(xcrun lipo -archs "$archive")" == "$(uname -m)" ]]

core-refloat-dynamic-telemetry-abi-audit: apple-rust-refloat-dynamic-telemetry
    #!/usr/bin/env bash
    set -euo pipefail
    archive='backends/core/target/apple/macos/libfloat_view_refloat_dynamic_telemetry_apple_ffi.a'
    actual="$({ nm -gjU "$archive" 2>/dev/null || true; } \
      | sed 's/^_//' | rg '^float_view_[a-z_]+$' | sort -u)"
    expected="$(printf '%s\n' \
      float_view_refloat_dynamic_telemetry_abi_version \
      float_view_refloat_dynamic_telemetry_decode_layout \
      float_view_refloat_dynamic_telemetry_decode_realtime)"
    [[ "$actual" == "$expected" ]]
    rg -Fq 'pub const ABI_VERSION: u32 = u32::from_be_bytes([0, 0, 0, RECORD_VERSION]);' \
      backends/core/refloat-dynamic-telemetry-apple-ffi/src/lib.rs
    [[ "$(xcrun lipo -archs "$archive")" == "$(uname -m)" ]]

core-refloat-legacy-telemetry-abi-test: apple-rust-refloat-legacy-telemetry
    xcrun clang -std=c17 -Wall -Wextra -Werror \
      backends/core/refloat-legacy-telemetry-apple-ffi/tests/abi.c \
      -I backends/apple/Sources/CVescRefloatLegacyTelemetry/include \
      -L backends/core/target/apple/macos \
      -lfloat_view_refloat_legacy_telemetry_apple_ffi \
      -o /tmp/float-view-refloat-legacy-telemetry-abi-test
    /tmp/float-view-refloat-legacy-telemetry-abi-test

core-refloat-legacy-telemetry-abi-audit: apple-rust-refloat-legacy-telemetry
    #!/usr/bin/env bash
    set -euo pipefail
    archive='backends/core/target/apple/macos/libfloat_view_refloat_legacy_telemetry_apple_ffi.a'
    actual="$({ nm -gjU "$archive" 2>/dev/null || true; } \
      | sed 's/^_//' | rg '^float_view_[a-z_]+$' | sort -u)"
    expected="$(printf '%s\n' \
      float_view_refloat_legacy_telemetry_abi_version \
      float_view_refloat_legacy_telemetry_decode)"
    [[ "$actual" == "$expected" ]]
    rg -q '^pub const ABI_VERSION: u32 = RECORD_VERSION;$' \
      backends/core/refloat-legacy-telemetry-apple-ffi/src/lib.rs
    [[ "$(xcrun lipo -archs "$archive")" == "$(uname -m)" ]]

core-refloat-configuration-codec-abi-test: apple-rust-refloat-configuration-codec
    xcrun clang -std=c17 -Wall -Wextra -Werror \
      backends/core/refloat-configuration-codec-apple-ffi/tests/abi.c \
      -I backends/apple/Sources/CVescRefloatConfigurationCodec/include \
      -L backends/core/target/apple/macos \
      -lfloat_view_refloat_configuration_codec_apple_ffi \
      -o /tmp/float-view-refloat-configuration-codec-abi-test
    /tmp/float-view-refloat-configuration-codec-abi-test

core-refloat-configuration-codec-abi-audit: apple-rust-refloat-configuration-codec
    #!/usr/bin/env bash
    set -euo pipefail
    archive='backends/core/target/apple/macos/libfloat_view_refloat_configuration_codec_apple_ffi.a'
    actual="$({ nm -gjU "$archive" 2>/dev/null || true; } \
      | sed 's/^_//' | rg '^float_view_[a-z_]+$' | sort -u)"
    expected="$(printf '%s\n' \
      float_view_refloat_configuration_codec_abi_version \
      float_view_refloat_configuration_codec_decode_schema \
      float_view_refloat_configuration_codec_decode_values \
      float_view_refloat_configuration_codec_encode_values)"
    [[ "$actual" == "$expected" ]]
    [[ "$(xcrun lipo -archs "$archive")" == "$(uname -m)" ]]

core-identity-abi-audit: apple-rust-firmware-identity
    #!/usr/bin/env bash
    set -euo pipefail
    archive='backends/core/target/apple/macos/libfloat_view_vesc_firmware_identity_apple_ffi.a'
    actual="$({ nm -gjU "$archive" 2>/dev/null || true; } \
      | sed 's/^_//' | rg '^float_view_[a-z_]+$' | sort -u)"
    expected="$(printf '%s\n' \
      float_view_vesc_firmware_identity_abi_version \
      float_view_vesc_firmware_identity_decode)"
    [[ "$actual" == "$expected" ]] || {
      echo 'Unexpected Float View firmware-identity ABI symbols:' >&2
      printf '%s\n' "$actual" >&2
      exit 1
    }
    rust_abi="$(sed -n 's/^pub const RECORD_VERSION: u32 = \([0-9][0-9]*\);$/\1/p' \
      backends/core/vesc-firmware-identity/src/lib.rs)"
    rg -q '^pub const ABI_VERSION: u32 = RECORD_VERSION;$' \
      backends/core/vesc-firmware-identity-apple-ffi/src/lib.rs
    header_abi="$(sed -n 's/^#define FLOAT_VIEW_VESC_FIRMWARE_IDENTITY_ABI_VERSION UINT32_C(\([0-9][0-9]*\))$/\1/p' \
      backends/apple/Sources/CVescFirmwareIdentity/include/CVescFirmwareIdentity.h)"
    swift_abi="$(sed -n 's/^[[:space:]]*private static let expectedABI: UInt32 = \([0-9][0-9]*\)$/\1/p' \
      backends/apple/Sources/FloatViewBackend/protocol/vesc/VescFirmwareIdentityDecoder.swift)"
    [[ -n "$rust_abi" && "$rust_abi" == "$header_abi" && "$rust_abi" == "$swift_abi" ]] || {
      echo "VESC firmware-identity ABI version mismatch: Rust=$rust_abi header=$header_abi Swift=$swift_abi" >&2
      exit 1
    }

core-custom-configuration-abi-test: apple-rust-custom-configuration
    xcrun clang -std=c17 -Wall -Wextra -Werror \
      backends/core/vesc-custom-configuration-apple-ffi/tests/abi.c \
      -I backends/apple/Sources/CVescCustomConfiguration/include \
      -L backends/core/target/apple/macos \
      -lfloat_view_vesc_custom_configuration_apple_ffi \
      -o /tmp/float-view-vesc-custom-configuration-abi-test
    /tmp/float-view-vesc-custom-configuration-abi-test

core-custom-configuration-abi-audit: apple-rust-custom-configuration
    #!/usr/bin/env bash
    set -euo pipefail
    archive='backends/core/target/apple/macos/libfloat_view_vesc_custom_configuration_apple_ffi.a'
    actual="$({ nm -gjU "$archive" 2>/dev/null || true; } \
      | sed 's/^_//' | rg '^float_view_[a-z_]+$' | sort -u)"
    expected="$(printf '%s\n' \
      float_view_vesc_custom_configuration_abi_version \
      float_view_vesc_custom_configuration_decode \
      float_view_vesc_custom_configuration_encode)"
    [[ "$actual" == "$expected" ]] || {
      echo 'Unexpected Float View custom-configuration ABI symbols:' >&2
      printf '%s\n' "$actual" >&2
      exit 1
    }
    rust_abi="$(sed -n 's/^pub const ABI_VERSION: u32 = \([0-9][0-9]*\);$/\1/p' \
      backends/core/vesc-custom-configuration-apple-ffi/src/lib.rs)"
    header_abi="$(sed -n 's/^#define FLOAT_VIEW_VESC_CUSTOM_CONFIGURATION_ABI_VERSION UINT32_C(\([0-9][0-9]*\))$/\1/p' \
      backends/apple/Sources/CVescCustomConfiguration/include/CVescCustomConfiguration.h)"
    swift_abi="$(sed -n 's/^[[:space:]]*private static let expectedABI: UInt32 = \([0-9][0-9]*\)$/\1/p' \
      backends/apple/Sources/FloatViewBackend/protocol/vesc/VescCustomConfigurationCodec.swift)"
    [[ -n "$rust_abi" && "$rust_abi" == "$header_abi" && "$rust_abi" == "$swift_abi" ]] || {
      echo "VESC custom-configuration ABI mismatch: Rust=$rust_abi header=$header_abi Swift=$swift_abi" >&2
      exit 1
    }

core-bms-values-abi-test: apple-rust-bms-values
    xcrun clang -std=c17 -Wall -Wextra -Werror \
      backends/core/vesc-bms-values-apple-ffi/tests/abi.c \
      -I backends/apple/Sources/CVescBmsValues/include \
      -L backends/core/target/apple/macos \
      -lfloat_view_vesc_bms_values_apple_ffi \
      -o /tmp/float-view-vesc-bms-values-abi-test
    /tmp/float-view-vesc-bms-values-abi-test

core-bms-values-abi-audit: apple-rust-bms-values
    #!/usr/bin/env bash
    set -euo pipefail
    archive='backends/core/target/apple/macos/libfloat_view_vesc_bms_values_apple_ffi.a'
    actual="$({ nm -gjU "$archive" 2>/dev/null || true; } \
      | sed 's/^_//' | rg '^float_view_[a-z_]+$' | sort -u)"
    expected="$(printf '%s\n' \
      float_view_vesc_bms_values_abi_version \
      float_view_vesc_bms_values_decode)"
    [[ "$actual" == "$expected" ]] || {
      echo 'Unexpected Float View BMS values ABI symbols:' >&2
      printf '%s\n' "$actual" >&2
      exit 1
    }
    rust_abi="$(sed -n 's/^pub const ABI_VERSION: u32 = \([0-9][0-9]*\);$/\1/p' \
      backends/core/vesc-bms-values-apple-ffi/src/lib.rs)"
    header_abi="$(sed -n 's/^#define FLOAT_VIEW_VESC_BMS_VALUES_ABI_VERSION UINT32_C(\([0-9][0-9]*\))$/\1/p' \
      backends/apple/Sources/CVescBmsValues/include/CVescBmsValues.h)"
    swift_abi="$(sed -n 's/^[[:space:]]*private static let expectedABI: UInt32 = \([0-9][0-9]*\)$/\1/p' \
      backends/apple/Sources/FloatViewBackend/protocol/vesc/VescBmsValuesDecoder.swift)"
    [[ -n "$rust_abi" && "$rust_abi" == "$header_abi" && "$rust_abi" == "$swift_abi" ]] || {
      echo "VESC BMS values ABI mismatch: Rust=$rust_abi header=$header_abi Swift=$swift_abi" >&2
      exit 1
    }
    [[ "$(xcrun lipo -archs "$archive")" == "$(uname -m)" ]]

core-controller-data-abi-test: apple-rust-controller-data
    xcrun clang -std=c17 -Wall -Wextra -Werror \
      backends/core/vesc-controller-data-apple-ffi/tests/abi.c \
      -I backends/apple/Sources/CVescControllerData/include \
      -L backends/core/target/apple/macos \
      -lfloat_view_vesc_controller_data_apple_ffi \
      -o /tmp/float-view-vesc-controller-data-abi-test
    /tmp/float-view-vesc-controller-data-abi-test

core-controller-data-abi-audit: apple-rust-controller-data
    #!/usr/bin/env bash
    set -euo pipefail
    archive='backends/core/target/apple/macos/libfloat_view_vesc_controller_data_apple_ffi.a'
    actual="$({ nm -gjU "$archive" 2>/dev/null || true; } \
      | sed 's/^_//' | rg '^float_view_[a-z_]+$' | sort -u)"
    expected="$(printf '%s\n' \
      float_view_vesc_controller_data_abi_version \
      float_view_vesc_controller_data_decode)"
    [[ "$actual" == "$expected" ]] || {
      echo 'Unexpected Float View controller-data ABI symbols:' >&2
      printf '%s\n' "$actual" >&2
      exit 1
    }
    rust_abi="$(sed -n 's/^pub const ABI_VERSION: u32 = \([0-9][0-9]*\);$/\1/p' \
      backends/core/vesc-controller-data-apple-ffi/src/lib.rs)"
    header_abi="$(sed -n 's/^#define FLOAT_VIEW_VESC_CONTROLLER_DATA_ABI_VERSION UINT32_C(\([0-9][0-9]*\))$/\1/p' \
      backends/apple/Sources/CVescControllerData/include/CVescControllerData.h)"
    swift_abi="$(sed -n 's/^[[:space:]]*private static let expectedABI: UInt32 = \([0-9][0-9]*\)$/\1/p' \
      backends/apple/Sources/FloatViewBackend/protocol/vesc/VescControllerDataDecoder.swift)"
    [[ -n "$rust_abi" && "$rust_abi" == "$header_abi" && "$rust_abi" == "$swift_abi" ]] || {
      echo "VESC controller-data ABI mismatch: Rust=$rust_abi header=$header_abi Swift=$swift_abi" >&2
      exit 1
    }
    [[ "$(xcrun lipo -archs "$archive")" == "$(uname -m)" ]]

core-refloat-control-abi-test: apple-rust-refloat-control
    xcrun clang -std=c17 -Wall -Wextra -Werror \
      backends/core/refloat-control-apple-ffi/tests/abi.c \
      -I backends/apple/Sources/CVescRefloatControl/include \
      -L backends/core/target/apple/macos \
      -lfloat_view_refloat_control_apple_ffi \
      -o /tmp/float-view-refloat-control-abi-test
    /tmp/float-view-refloat-control-abi-test

core-refloat-control-abi-audit: apple-rust-refloat-control
    #!/usr/bin/env bash
    set -euo pipefail
    archive='backends/core/target/apple/macos/libfloat_view_refloat_control_apple_ffi.a'
    actual="$({ nm -gjU "$archive" 2>/dev/null || true; } \
      | sed 's/^_//' | rg '^float_view_[a-z_]+$' | sort -u)"
    expected="$(printf '%s\n' \
      float_view_refloat_control_abi_version \
      float_view_refloat_control_decode_lighting \
      float_view_refloat_control_encode)"
    [[ "$actual" == "$expected" ]] || {
      echo 'Unexpected Float View Refloat-control ABI symbols:' >&2
      printf '%s\n' "$actual" >&2
      exit 1
    }
    rust_abi="$(sed -n 's/^pub const ABI_VERSION: u32 = \([0-9][0-9]*\);$/\1/p' \
      backends/core/refloat-control-apple-ffi/src/lib.rs)"
    header_abi="$(sed -n 's/^#define FLOAT_VIEW_REFLOAT_CONTROL_ABI_VERSION UINT32_C(\([0-9][0-9]*\))$/\1/p' \
      backends/apple/Sources/CVescRefloatControl/include/CVescRefloatControl.h)"
    swift_abi="$(sed -n 's/^[[:space:]]*private static let expectedABI: UInt32 = \([0-9][0-9]*\)$/\1/p' \
      backends/apple/Sources/FloatViewBackend/protocol/refloat/RefloatControlCodec.swift)"
    [[ -n "$rust_abi" && "$rust_abi" == "$header_abi" && "$rust_abi" == "$swift_abi" ]] || {
      echo "Refloat control ABI mismatch: Rust=$rust_abi header=$header_abi Swift=$swift_abi" >&2
      exit 1
    }
    [[ "$(xcrun lipo -archs "$archive")" == "$(uname -m)" ]]
apple-packaging:
    just apple-rust-bridges Release
    cd backends/apple && swift build -c release \
      --scratch-path /tmp/float-view-swift-release --product FloatViewMac
    xcodebuild -quiet -project backends/apple/ios/float-view.xcodeproj \
      -scheme float-view -configuration Release -sdk iphonesimulator \
      -destination 'generic/platform=iOS Simulator' \
      -derivedDataPath /tmp/float-view-ios-release CODE_SIGNING_ALLOWED=NO clean build
    xcodebuild -quiet -project backends/apple/ios/float-view.xcodeproj \
      -scheme float-view -configuration Debug -sdk iphoneos \
      -destination 'generic/platform=iOS' \
      -derivedDataPath /tmp/float-view-ios-device CODE_SIGNING_ALLOWED=NO clean build
    just apple-rust-bridges Debug

core-test: core-abi-test core-identity-abi-test core-custom-configuration-abi-test core-bms-values-abi-test core-controller-data-abi-test core-refloat-control-abi-test core-refloat-compatibility-abi-test core-refloat-dynamic-telemetry-abi-test core-refloat-legacy-telemetry-abi-test core-refloat-configuration-codec-abi-test
    cd backends/core && cargo test --locked --offline

core-refloat-configuration-codec-test:
    cd backends/core && cargo test --locked --offline \
      --package float-view-refloat-configuration-codec

core-refloat-configuration-service-test:
    cd backends/core && cargo test --locked --offline \
      --package float-view-refloat-configuration-service

core-refloat-configuration-service-audit:
    #!/usr/bin/env bash
    set -euo pipefail
    crate='backends/core/refloat-configuration-service'
    [[ -f "$crate/Cargo.toml" && -f "$crate/src/lib.rs" ]]
    [[ -f "$crate/tests/service.rs" && -f "$crate/tests/adversarial.rs" ]]
    rg -q '^#!\[forbid\(unsafe_code\)\]$' "$crate/src/lib.rs"
    [[ "$(sed -n '/^\[dependencies\]$/,$p' "$crate/Cargo.toml")" == \
      $'[dependencies]\nfloat-view-refloat-configuration-codec = { path = "../refloat-configuration-codec" }' ]]
    [[ "$(cd backends/core && cargo tree --locked --offline \
      --package float-view-refloat-configuration-service --edges normal \
      --depth 1 --prefix none | wc -l | tr -d ' ')" == 2 ]]
    ! rg -n 'unsafe[[:space:]]*(\{|fn|impl|trait)|extern[[:space:]]+"(C|system)"|no_mangle|Java_|cbindgen|bindgen|uniffi|jni|host_callback' \
      "$crate/src" "$crate/tests"
    [[ "$(find "$crate" -mindepth 1 -maxdepth 1 -type d \
      ! -name src ! -name tests -print -quit)" == "" ]]
    ! rg -n 'refloat-configuration-service' \
      backends/apple backends/android backends/core/bridge-build-catalog.sh \
      backends/core/build-apple-bridge.sh backends/core/build-android-bridge.sh
    [[ "$(git ls-files "$crate" | rg '\.(h|a|so|dylib|xcframework|jar)$' || true)" == "" ]]
    just core-refloat-configuration-service-test

core-vesc-request-scheduler-test:
    cd backends/core && cargo test --locked --offline \
      --package float-view-vesc-request-scheduler

core-vesc-request-scheduler-audit:
    #!/usr/bin/env bash
    set -euo pipefail
    crate='backends/core/vesc-request-scheduler'
    [[ -f "$crate/Cargo.toml" && -f "$crate/src/lib.rs" ]]
    rg -q '^#!\[forbid\(unsafe_code\)\]$' "$crate/src/lib.rs"
    [[ "$(sed -n '/^\[dependencies\]$/,$p' "$crate/Cargo.toml")" == '[dependencies]' ]]
    [[ "$(cd backends/core && cargo tree --locked --offline \
      --package float-view-vesc-request-scheduler --edges normal \
      --prefix none | wc -l | tr -d ' ')" == 1 ]]
    ! rg -n 'unsafe[[:space:]]*(\{|fn|impl|trait)|extern[[:space:]]+"(C|system)"|no_mangle|Java_|cbindgen|bindgen|uniffi|jni|host_callback|Box<dyn Fn|std::thread|tokio|rusqlite' \
      "$crate/src"
    [[ "$(find "$crate" -mindepth 1 -maxdepth 1 -type d \
      ! -name src -print -quit)" == "" ]]
    ! rg -n 'vesc-request-scheduler' \
      backends/apple backends/android backends/core/bridge-build-catalog.sh \
      backends/core/build-apple-bridge.sh backends/core/build-android-bridge.sh
    [[ "$(git ls-files "$crate" | rg '\.(h|a|so|dylib|xcframework|jar)$' || true)" == "" ]]
    just core-vesc-request-scheduler-test

core-vesc-compatibility-test:
    cd backends/core && cargo test --locked --offline \
      --package float-view-vesc-compatibility

core-vesc-compatibility-audit:
    #!/usr/bin/env bash
    set -euo pipefail
    crate='backends/core/vesc-compatibility'
    [[ -f "$crate/Cargo.toml" && -f "$crate/src/lib.rs" ]]
    rg -q '^#!\[forbid\(unsafe_code\)\]$' "$crate/src/lib.rs"
    [[ "$(sed -n '/^\[dependencies\]$/,/^\[dev-dependencies\]$/p' "$crate/Cargo.toml" | sed '$d')" == \
      $'[dependencies]\nfloat-view-vesc-firmware-identity = { path = "../vesc-firmware-identity" }' ]]
    [[ "$(cd backends/core && cargo tree --locked --offline \
      --package float-view-vesc-compatibility --edges normal \
      --depth 1 --prefix none | wc -l | tr -d ' ')" == 2 ]]
    ! rg -n 'unsafe[[:space:]]*(\{|fn|impl|trait)|extern[[:space:]]+"(C|system)"|no_mangle|Java_|cbindgen|bindgen|uniffi|jni|host_callback|Box<dyn Fn|std::thread|tokio|async_std|rusqlite|sqlite|TcpListener|UdpSocket|std::fs|std::net' \
      "$crate/src"
    [[ "$(find "$crate" -mindepth 1 -maxdepth 1 -type d \
      ! -name src -print -quit)" == "" ]]
    ! rg -n 'vesc-compatibility' \
      backends/apple backends/android backends/core/bridge-build-catalog.sh \
      backends/core/build-apple-bridge.sh backends/core/build-android-bridge.sh
    actual_bridges="$(sed -n 's/^    \([a-z][a-z-]*\))$/\1/p' \
      backends/core/bridge-build-catalog.sh)"
    expected_bridges="$(printf '%s\n' \
      frame firmware-identity custom-configuration bms-values controller-data \
      refloat-control refloat-compatibility refloat-dynamic-telemetry \
      refloat-legacy-telemetry refloat-configuration-codec)"
    [[ "$actual_bridges" == "$expected_bridges" ]]
    [[ -f backends/apple/Sources/FloatViewBackend/protocol/vesc/VescCompatibility.swift ]]
    [[ -f backends/android/app/src/main/java/fail/acheron/floatview/protocol/vesc/VescCompatibility.kt ]]
    [[ "$(git ls-files "$crate" | rg '\.(h|a|so|dylib|xcframework|jar)$' || true)" == "" ]]
    just core-vesc-compatibility-test

core-controller-discovery-test:
    cd backends/core && cargo test --locked --offline \
      --package float-view-controller-discovery

core-controller-discovery-audit:
    #!/usr/bin/env bash
    set -euo pipefail
    crate='backends/core/controller-discovery'
    [[ -f "$crate/Cargo.toml" && -f "$crate/src/lib.rs" ]]
    [[ -f "$crate/tests/vectors.rs" && -f "$crate/tests/adversarial.rs" ]]
    rg -q '^#!\[forbid\(unsafe_code\)\]$' "$crate/src/lib.rs"
    [[ "$(sed -n '/^\[dependencies\]$/,$p' "$crate/Cargo.toml")" == \
      $'[dependencies]\nfloat-view-refloat-compatibility = { path = "../refloat-compatibility" }\nfloat-view-vesc-compatibility = { path = "../vesc-compatibility" }\nfloat-view-vesc-firmware-identity = { path = "../vesc-firmware-identity" }' ]]
    [[ "$(cd backends/core && cargo tree --locked --offline \
      --package float-view-controller-discovery --edges normal \
      --depth 1 --prefix none | wc -l | tr -d ' ')" == 4 ]]
    ! rg -n 'unsafe[[:space:]]*(\{|fn|impl|trait)|extern[[:space:]]+"(C|system)"|no_mangle|Java_|cbindgen|bindgen|uniffi|jni|host_callback|Box<dyn Fn|std::thread|tokio|async_std|rusqlite|sqlite|TcpListener|UdpSocket|std::fs|std::net' \
      "$crate/src" "$crate/tests"
    [[ "$(find "$crate" -mindepth 1 -maxdepth 1 -type d \
      ! -name src ! -name tests -print -quit)" == "" ]]
    ! rg -n 'controller-discovery' \
      backends/apple/Package.swift \
      backends/apple/ios/float-view.xcodeproj/project.pbxproj \
      backends/android/app/build.gradle.kts \
      backends/core/bridge-build-catalog.sh \
      backends/core/build-apple-bridge.sh backends/core/build-android-bridge.sh
    actual_bridges="$(sed -n 's/^    \([a-z][a-z-]*\))$/\1/p' \
      backends/core/bridge-build-catalog.sh)"
    expected_bridges="$(printf '%s\n' \
      frame firmware-identity custom-configuration bms-values controller-data \
      refloat-control refloat-compatibility refloat-dynamic-telemetry \
      refloat-legacy-telemetry refloat-configuration-codec)"
    [[ "$actual_bridges" == "$expected_bridges" ]]
    [[ -f backends/apple/Sources/FloatViewBackend/application/ControllerDiscovery.swift ]]
    [[ -f backends/android/app/src/main/java/fail/acheron/floatview/application/ControllerDiscovery.kt ]]
    [[ "$(git ls-files "$crate" | rg '\.(h|a|so|dylib|xcframework|jar)$' || true)" == "" ]]
    just core-controller-discovery-test

core-auxiliary-node-discovery-test:
    cd backends/core && cargo test --locked --offline \
      --package float-view-auxiliary-node-discovery

core-auxiliary-node-discovery-audit:
    #!/usr/bin/env bash
    set -euo pipefail
    crate='backends/core/auxiliary-node-discovery'
    [[ -f "$crate/Cargo.toml" && -f "$crate/src/lib.rs" ]]
    [[ -f "$crate/tests/vectors.rs" && -f "$crate/tests/adversarial.rs" ]]
    [[ -f backends/fixtures/connection/auxiliary-node-discovery.tsv ]]
    [[ "$(sed '/^#/d;/^$/d' backends/fixtures/connection/auxiliary-node-discovery.tsv | wc -l | tr -d ' ')" == 13 ]]
    rg -q '^#!\[forbid\(unsafe_code\)\]$' "$crate/src/lib.rs"
    [[ "$(sed -n '/^\[dependencies\]$/,/^\[dev-dependencies\]$/p' "$crate/Cargo.toml" | sed '$d')" == \
      $'[dependencies]\nfloat-view-controller-discovery = { path = "../controller-discovery" }' ]]
    [[ "$(cd backends/core && cargo tree --locked --offline \
      --package float-view-auxiliary-node-discovery --edges normal \
      --depth 1 --prefix none | wc -l | tr -d ' ')" == 2 ]]
    ! rg -n 'unsafe[[:space:]]*(\{|fn|impl|trait)|extern[[:space:]]+"(C|system)"|no_mangle|Java_|cbindgen|bindgen|uniffi|jni|host_callback|Box<dyn Fn|std::thread|tokio|async_std|rusqlite|sqlite|TcpListener|UdpSocket|std::fs|std::net' \
      "$crate/src" "$crate/tests"
    [[ "$(find "$crate" -mindepth 1 -maxdepth 1 -type d \
      ! -name src ! -name tests -print -quit)" == "" ]]
    ! rg -n 'auxiliary-node-discovery' \
      backends/apple/Package.swift \
      backends/apple/ios/float-view.xcodeproj/project.pbxproj \
      backends/android/app/build.gradle.kts \
      backends/core/bridge-build-catalog.sh \
      backends/core/build-apple-bridge.sh backends/core/build-android-bridge.sh
    actual_bridges="$(sed -n 's/^    \([a-z][a-z-]*\))$/\1/p' \
      backends/core/bridge-build-catalog.sh)"
    expected_bridges="$(printf '%s\n' \
      frame firmware-identity custom-configuration bms-values controller-data \
      refloat-control refloat-compatibility refloat-dynamic-telemetry \
      refloat-legacy-telemetry refloat-configuration-codec)"
    [[ "$actual_bridges" == "$expected_bridges" ]]
    [[ -f backends/apple/Sources/FloatViewBackend/application/AuxiliaryNodeDiscovery.swift ]]
    [[ -f backends/android/app/src/main/java/fail/acheron/floatview/application/AuxiliaryNodeDiscovery.kt ]]
    [[ "$(git ls-files "$crate" | rg '\.(h|a|so|dylib|xcframework|jar)$' || true)" == "" ]]
    just core-auxiliary-node-discovery-test

core-vesc-bms-telemetry-service-test:
    cd backends/core && cargo test --locked --offline \
      --package float-view-vesc-bms-telemetry-service

core-vesc-bms-telemetry-service-audit:
    #!/usr/bin/env bash
    set -euo pipefail
    crate='backends/core/vesc-bms-telemetry-service'
    [[ -f "$crate/Cargo.toml" && -f "$crate/src/lib.rs" ]]
    [[ -f "$crate/tests/vectors.rs" && -f "$crate/tests/adversarial.rs" ]]
    [[ -f backends/fixtures/connection/vesc-bms-telemetry-service.tsv ]]
    [[ "$(sed '/^#/d;/^$/d' backends/fixtures/connection/vesc-bms-telemetry-service.tsv | wc -l | tr -d ' ')" == 12 ]]
    rg -q '^#!\[forbid\(unsafe_code\)\]$' "$crate/src/lib.rs"
    [[ "$(sed -n '/^\[dependencies\]$/,/^\[dev-dependencies\]$/p' "$crate/Cargo.toml" | sed '$d')" == \
      $'[dependencies]\nfloat-view-auxiliary-node-discovery = { path = "../auxiliary-node-discovery" }\nfloat-view-vesc-bms-values = { path = "../vesc-bms-values" }\nfloat-view-vesc-request-scheduler = { path = "../vesc-request-scheduler" }' ]]
    [[ "$(cd backends/core && cargo tree --locked --offline \
      --package float-view-vesc-bms-telemetry-service --edges normal \
      --depth 1 --prefix none | wc -l | tr -d ' ')" == 4 ]]
    ! rg -n 'unsafe[[:space:]]*(\{|fn|impl|trait)|extern[[:space:]]+"(C|system)"|no_mangle|Java_|cbindgen|bindgen|uniffi|jni|host_callback|Box<dyn Fn|std::thread|tokio|async_std|rusqlite|sqlite|TcpListener|UdpSocket|std::fs|std::net' \
      "$crate/src" "$crate/tests"
    [[ "$(find "$crate" -mindepth 1 -maxdepth 1 -type d \
      ! -name src ! -name tests -print -quit)" == "" ]]
    ! rg -n 'vesc-bms-telemetry-service' \
      backends/apple backends/android backends/core/bridge-build-catalog.sh \
      backends/core/build-apple-bridge.sh backends/core/build-android-bridge.sh
    actual_bridges="$(sed -n 's/^    \([a-z][a-z-]*\))$/\1/p' \
      backends/core/bridge-build-catalog.sh)"
    expected_bridges="$(printf '%s\n' \
      frame firmware-identity custom-configuration bms-values controller-data \
      refloat-control refloat-compatibility refloat-dynamic-telemetry \
      refloat-legacy-telemetry refloat-configuration-codec)"
    [[ "$actual_bridges" == "$expected_bridges" ]]
    [[ -f backends/apple/Sources/FloatViewBackend/application/ApplicationService.swift ]]
    [[ -f backends/apple/Sources/FloatViewBackend/application/AuxiliaryNodeDiscovery.swift ]]
    [[ -f backends/android/app/src/main/java/fail/acheron/floatview/application/ApplicationService.kt ]]
    [[ -f backends/android/app/src/main/java/fail/acheron/floatview/application/AuxiliaryNodeDiscovery.kt ]]
    [[ "$(git ls-files "$crate" | rg '\.(h|a|so|dylib|xcframework|jar)$' || true)" == "" ]]
    just core-vesc-bms-telemetry-service-test

core-refloat-telemetry-service-test:
    cd backends/core && cargo test --locked --offline \
      --package float-view-refloat-telemetry-service

core-refloat-telemetry-service-audit:
    #!/usr/bin/env bash
    set -euo pipefail
    crate='backends/core/refloat-telemetry-service'
    [[ -f "$crate/Cargo.toml" && -f "$crate/src/lib.rs" ]]
    rg -q '^#!\[forbid\(unsafe_code\)\]$' "$crate/src/lib.rs"
    [[ "$(sed -n '/^\[dependencies\]$/,$p' "$crate/Cargo.toml")" == \
      $'[dependencies]\nfloat-view-refloat-dynamic-telemetry = { path = "../refloat-dynamic-telemetry" }\nfloat-view-refloat-legacy-telemetry = { path = "../refloat-legacy-telemetry" }\nfloat-view-vesc-request-scheduler = { path = "../vesc-request-scheduler" }' ]]
    [[ "$(cd backends/core && cargo tree --locked --offline \
      --package float-view-refloat-telemetry-service --edges normal \
      --depth 1 --prefix none | wc -l | tr -d ' ')" == 4 ]]
    ! rg -n 'unsafe[[:space:]]*(\{|fn|impl|trait)|extern[[:space:]]+"(C|system)"|no_mangle|Java_|cbindgen|bindgen|uniffi|jni|host_callback|Box<dyn Fn|std::thread|tokio|async_std|rusqlite|sqlite|TcpListener|UdpSocket|std::fs|std::net' \
      "$crate/src"
    [[ "$(find "$crate" -mindepth 1 -maxdepth 1 -type d \
      ! -name src -print -quit)" == "" ]]
    ! rg -n 'refloat-telemetry-service' \
      backends/apple backends/android backends/core/bridge-build-catalog.sh \
      backends/core/build-apple-bridge.sh backends/core/build-android-bridge.sh
    [[ "$(git ls-files "$crate" | rg '\.(h|a|so|dylib|xcframework|jar)$' || true)" == "" ]]
    just core-refloat-telemetry-service-test

core-connection-application-coordinator-test:
    cd backends/core && cargo test --locked --offline \
      --package float-view-connection-application-coordinator

core-connection-application-coordinator-audit:
    #!/usr/bin/env bash
    set -euo pipefail
    crate='backends/core/connection-application-coordinator'
    [[ -f "$crate/Cargo.toml" && -f "$crate/src/lib.rs" ]]
    [[ -f "$crate/tests/vectors.rs" && -f "$crate/tests/adversarial.rs" && -f "$crate/tests/composition.rs" && -f "$crate/tests/current_limits.rs" ]]
    [[ -f backends/fixtures/connection/connection-application-coordinator.tsv ]]
    [[ "$(sed '/^#/d;/^$/d' backends/fixtures/connection/connection-application-coordinator.tsv | wc -l | tr -d ' ')" == 8 ]]
    [[ -f backends/fixtures/connection/device-configuration-composition.tsv ]]
    [[ "$(sed '/^#/d;/^$/d' backends/fixtures/connection/device-configuration-composition.tsv | wc -l | tr -d ' ')" == 12 ]]
    [[ -f backends/fixtures/connection/controller-current-limits.tsv ]]
    [[ "$(sed '/^#/d;/^$/d' backends/fixtures/connection/controller-current-limits.tsv | wc -l | tr -d ' ')" == 4 ]]
    rg -q '^#!\[forbid\(unsafe_code\)\]$' "$crate/src/lib.rs"
    [[ "$(sed -n '/^\[dependencies\]$/,$p' "$crate/Cargo.toml")" == \
      $'[dependencies]\nfloat-view-controller-discovery = { path = "../controller-discovery" }\nfloat-view-device-mutation-coordinator = { path = "../device-mutation-coordinator" }\nfloat-view-refloat-compatibility = { path = "../refloat-compatibility" }\nfloat-view-refloat-configuration-codec = { path = "../refloat-configuration-codec" }\nfloat-view-refloat-configuration-service = { path = "../refloat-configuration-service" }\nfloat-view-refloat-dynamic-telemetry = { path = "../refloat-dynamic-telemetry" }\nfloat-view-refloat-legacy-telemetry = { path = "../refloat-legacy-telemetry" }\nfloat-view-refloat-telemetry-service = { path = "../refloat-telemetry-service" }\nfloat-view-vesc-bms-telemetry-service = { path = "../vesc-bms-telemetry-service" }\nfloat-view-vesc-custom-configuration = { path = "../vesc-custom-configuration" }\nfloat-view-vesc-controller-data = { path = "../vesc-controller-data" }\nfloat-view-vesc-firmware-identity = { path = "../vesc-firmware-identity" }\nfloat-view-vesc-request-scheduler = { path = "../vesc-request-scheduler" }' ]]
    [[ "$(cd backends/core && cargo tree --locked --offline \
      --package float-view-connection-application-coordinator --edges normal \
      --depth 1 --prefix none | wc -l | tr -d ' ')" == 14 ]]
    ! rg -n 'unsafe[[:space:]]*(\{|fn|impl|trait)|extern[[:space:]]+"(C|system)"|no_mangle|Java_|cbindgen|bindgen|uniffi|jni|host_callback|Box<dyn Fn|std::thread|tokio|async_std|rusqlite|sqlite|TcpListener|UdpSocket|std::fs|std::net' \
      "$crate/src" "$crate/tests"
    ! rg -n 'const[[:space:]]+[A-Z0-9_]+.*(92|93|94|95)|fn[[:space:]]+(schema_request|values_request|write_request|decode_response)' \
      "$crate/src"
    ! rg -n 'fn[[:space:]]+(decode_current_limits|float32_auto)|CURRENT_LIMITS_CONSUMED_BYTES|BATTERY_SERIES_CELL_OFFSETS' \
      "$crate/src"
    [[ "$(find "$crate" -mindepth 1 -maxdepth 1 -type d \
      ! -name src ! -name tests -print -quit)" == "" ]]
    ! rg -n 'connection-application-coordinator' \
      backends/apple backends/android backends/core/bridge-build-catalog.sh \
      backends/core/build-apple-bridge.sh backends/core/build-android-bridge.sh
    [[ -f backends/apple/Sources/FloatViewBackend/application/ApplicationService.swift ]]
    [[ -f backends/android/app/src/main/java/fail/acheron/floatview/application/ApplicationService.kt ]]
    [[ -f backends/apple/Sources/FloatViewBackend/application/RefloatConfigurationService.swift ]]
    [[ -f backends/android/app/src/main/java/fail/acheron/floatview/application/RefloatConfigurationService.kt ]]
    [[ -f backends/apple/Sources/FloatViewBackend/application/RefloatMutationCoordinator.swift ]]
    [[ -f backends/android/app/src/main/java/fail/acheron/floatview/application/RefloatMutationCoordinator.kt ]]
    [[ "$(git ls-files "$crate" | rg '\.(h|a|so|dylib|xcframework|jar)$' || true)" == "" ]]
    just core-connection-application-coordinator-test

core-session-history-kernel-test:
    cd backends/core && cargo test --locked --offline \
      --package float-view-session-history-kernel

core-session-history-kernel-audit:
    #!/usr/bin/env bash
    set -euo pipefail
    crate='backends/core/session-history-kernel'
    fixtures='backends/fixtures/session-history'
    [[ -f "$crate/Cargo.toml" && -f "$crate/src/lib.rs" ]]
    [[ -f "$crate/tests/vectors.rs" && -f "$crate/tests/adversarial.rs" && -f "$crate/tests/support/mod.rs" ]]
    for fixture in series.tsv series-expected.tsv prune.tsv prune-expected.tsv \
      prune-eight-cap.tsv prune-eight-cap-expected.tsv; do
      [[ -f "$fixtures/$fixture" ]]
    done
    [[ "$(sed '/^#/d;/^$/d' "$fixtures/series.tsv" | wc -l | tr -d ' ')" == 9 ]]
    [[ "$(sed '/^#/d;/^$/d' "$fixtures/series-expected.tsv" | wc -l | tr -d ' ')" == 17 ]]
    [[ "$(sed '/^#/d;/^$/d' "$fixtures/prune.tsv" | wc -l | tr -d ' ')" == 24 ]]
    [[ "$(sed '/^#/d;/^$/d' "$fixtures/prune-expected.tsv" | wc -l | tr -d ' ')" == 10 ]]
    [[ "$(sed '/^#/d;/^$/d' "$fixtures/prune-eight-cap.tsv" | wc -l | tr -d ' ')" == 11 ]]
    [[ "$(sed '/^#/d;/^$/d' "$fixtures/prune-eight-cap-expected.tsv" | wc -l | tr -d ' ')" == 10 ]]
    rg -q '^#!\[forbid\(unsafe_code\)\]$' "$crate/src/lib.rs"
    [[ "$(sed -n '/^\[dependencies\]$/,$p' "$crate/Cargo.toml")" == '[dependencies]' ]]
    [[ "$(cd backends/core && cargo tree --locked --offline \
      --package float-view-session-history-kernel --edges normal \
      --prefix none | wc -l | tr -d ' ')" == 1 ]]
    ! rg -n 'unsafe[[:space:]]*(\{|fn|impl|trait)|extern[[:space:]]+"(C|system)"|no_mangle|Java_|cbindgen|bindgen|uniffi|jni|host_callback|Box<dyn Fn|std::thread|tokio|async_std|rusqlite|libsqlite|sqlite3_|TcpListener|UdpSocket|std::fs|std::net|serde|base64' \
      "$crate/src" "$crate/tests"
    ! rg -n 'SessionHistoryReducer\.(swift|kt)|session-reference\.ts|session-vectors\.ts|prune-vectors\.ts|include_(str|bytes)!.*(frontend|apple|android)' \
      "$crate"
    [[ "$(find "$crate" -mindepth 1 -maxdepth 1 -type d \
      ! -name src ! -name tests -print -quit)" == "" ]]
    ! rg -n 'session-history-kernel' \
      backends/apple/Package.swift \
      backends/apple/ios/float-view.xcodeproj/project.pbxproj \
      backends/android/app/build.gradle.kts \
      backends/core/bridge-build-catalog.sh \
      backends/core/build-apple-bridge.sh backends/core/build-android-bridge.sh
    [[ ! -e backends/core/session-history-apple-ffi ]]
    [[ ! -e backends/core/session-history-android-jni ]]
    [[ -f backends/apple/Sources/FloatViewBackend/application/SessionHistoryReducer.swift ]]
    [[ -f backends/android/app/src/main/java/fail/acheron/floatview/application/SessionHistoryReducer.kt ]]
    git diff --quiet HEAD -- backends/apple backends/android frontend backends/sql
    [[ "$(git ls-files "$crate" | rg '\.(h|a|so|dylib|xcframework|jar)$' || true)" == "" ]]
    just core-session-history-kernel-test

core-device-mutation-coordinator-test:
    cd backends/core && cargo test --locked --offline \
      --package float-view-device-mutation-coordinator

core-device-mutation-coordinator-audit:
    #!/usr/bin/env bash
    set -euo pipefail
    crate='backends/core/device-mutation-coordinator'
    [[ -f "$crate/Cargo.toml" && -f "$crate/src/lib.rs" ]]
    [[ -f "$crate/tests/vectors.rs" && -f "$crate/tests/adversarial.rs" ]]
    [[ -f backends/fixtures/connection/device-mutation-coordinator.tsv ]]
    [[ "$(sed '/^#/d;/^$/d' backends/fixtures/connection/device-mutation-coordinator.tsv | wc -l | tr -d ' ')" == 16 ]]
    rg -q '^#!\[forbid\(unsafe_code\)\]$' "$crate/src/lib.rs"
    [[ "$(sed -n '/^\[dependencies\]$/,$p' "$crate/Cargo.toml")" == \
      $'[dependencies]\nfloat-view-refloat-compatibility = { path = "../refloat-compatibility" }\nfloat-view-refloat-control = { path = "../refloat-control" }\nfloat-view-refloat-telemetry-service = { path = "../refloat-telemetry-service" }\nfloat-view-vesc-compatibility = { path = "../vesc-compatibility" }\nfloat-view-vesc-firmware-identity = { path = "../vesc-firmware-identity" }\nfloat-view-vesc-request-scheduler = { path = "../vesc-request-scheduler" }' ]]
    [[ "$(cd backends/core && cargo tree --locked --offline \
      --package float-view-device-mutation-coordinator --edges normal \
      --depth 1 --prefix none | wc -l | tr -d ' ')" == 7 ]]
    ! rg -n 'unsafe[[:space:]]*(\{|fn|impl|trait)|extern[[:space:]]+"(C|system)"|no_mangle|Java_|cbindgen|bindgen|uniffi|jni|host_callback|Box<dyn Fn|std::thread|tokio|async_std|rusqlite|sqlite|TcpListener|UdpSocket|std::fs|std::net' \
      "$crate/src" "$crate/tests"
    [[ "$(find "$crate" -mindepth 1 -maxdepth 1 -type d \
      ! -name src ! -name tests -print -quit)" == "" ]]
    ! rg -n 'device-mutation-coordinator' \
      backends/apple backends/android backends/core/bridge-build-catalog.sh \
      backends/core/build-apple-bridge.sh backends/core/build-android-bridge.sh
    [[ -f backends/apple/Sources/FloatViewBackend/application/RefloatMutationCoordinator.swift ]]
    [[ -f backends/android/app/src/main/java/fail/acheron/floatview/application/RefloatMutationCoordinator.kt ]]
    [[ -f backends/apple/Sources/FloatViewBackend/application/ApplicationService.swift ]]
    [[ -f backends/android/app/src/main/java/fail/acheron/floatview/application/ApplicationService.kt ]]
    [[ "$(git ls-files "$crate" | rg '\.(h|a|so|dylib|xcframework|jar)$' || true)" == "" ]]
    just core-device-mutation-coordinator-test

core-refloat-configuration-codec-audit:
    #!/usr/bin/env bash
    set -euo pipefail
    crate='backends/core/refloat-configuration-codec'
    [[ -f "$crate/Cargo.toml" && -f "$crate/src/lib.rs" ]]
    [[ -f "$crate/tests/fixtures.rs" && -f "$crate/tests/adversarial.rs" ]]
    rg -q '^#!\[forbid\(unsafe_code\)\]$' "$crate/src/lib.rs"
    [[ "$(sed -n '/^\[dependencies\]$/,/^\[/p' "$crate/Cargo.toml" | sed '$d')" == \
      $'[dependencies]\nflate2 = { version = "=1.1.9", default-features = false, features = ["rust_backend"] }\nquick-xml = { version = "=0.39.4", default-features = false }\nsha2 = { version = "=0.11.0", default-features = false }' ]]
    dependency_names="$(cd backends/core && cargo tree --locked --offline \
      --package float-view-refloat-configuration-codec --edges normal \
      --prefix none --format '{p}' | sed -E 's/ v.*$//' | sort -u)"
    [[ "$dependency_names" == \
      $'adler2\nblock-buffer\ncfg-if\ncpufeatures\ncrc32fast\ncrypto-common\ndigest\nflate2\nfloat-view-refloat-configuration-codec\nhybrid-array\nlibc\nmemchr\nminiz_oxide\nquick-xml\nsha2\nsimd-adler32\ntypenum' ]]
    [[ "$(cd backends/core && cargo tree --locked --offline \
      --package float-view-refloat-configuration-codec --edges normal \
      --depth 1 --prefix none | wc -l | tr -d ' ')" == 4 ]]
    ! (cd backends/core && cargo tree --locked --offline \
      --package float-view-refloat-configuration-codec --edges normal) \
      | rg 'libz-sys|cloudflare-zlib|zlib-rs'
    ! rg -n 'unsafe[[:space:]]*(\{|fn|impl|trait)|extern[[:space:]]+"(C|system)"|no_mangle|Java_|cbindgen|bindgen|uniffi|jni|host_callback' \
      "$crate/src" "$crate/tests"
    [[ "$(find "$crate" -mindepth 1 -maxdepth 1 -type d \
      ! -name src ! -name tests -print -quit)" == "" ]]
    [[ -f backends/core/refloat-configuration-codec-apple-ffi/Cargo.toml ]]
    [[ -f backends/core/refloat-configuration-codec-apple-ffi/src/lib.rs ]]
    [[ -f backends/core/refloat-configuration-codec-apple-ffi/tests/abi.c ]]
    [[ -f backends/core/refloat-configuration-codec-android-jni/Cargo.toml ]]
    [[ -f backends/core/refloat-configuration-codec-android-jni/src/lib.rs ]]
    ! rg -n 'CryptoKit|import zlib|XMLParser|inflateInit_|crc32c|ConfigurationByteReader' \
      backends/apple/Sources/FloatViewBackend/protocol/refloat/RefloatConfigurationCodec.swift
    rg -q '^public enum RefloatConfigurationSchemaCodec' \
      backends/apple/Sources/FloatViewBackend/protocol/refloat/RefloatConfigurationCodec.swift
    rg -q '^public enum RefloatConfigurationValueCodec' \
      backends/apple/Sources/FloatViewBackend/protocol/refloat/RefloatConfigurationCodec.swift
    apple_configuration_source='backends/apple/Sources/FloatViewBackend/protocol/refloat/RefloatConfigurationCodec.swift'
    [[ "$(rg -c 'float_view_refloat_configuration_codec_(abi_version|decode_schema|decode_values|encode_values)\(' \
      "$apple_configuration_source")" == 4 ]]
    [[ "$(rg -l '^import CVescRefloatConfigurationCodec$' \
      backends/apple/Sources -g '*.swift')" == "$apple_configuration_source" ]]
    [[ "$(rg -l 'float_view_refloat_configuration_codec_' \
      backends/apple/Sources -g '*.swift')" == "$apple_configuration_source" ]]
    [[ "$(rg -c '\.systemLibrary\(name: "CVescRefloatConfigurationCodec"\)' \
      backends/apple/Package.swift)" == 1 ]]
    [[ "$(rg -c '\.linkedLibrary\("float_view_refloat_configuration_codec_apple_ffi"\)' \
      backends/apple/Package.swift)" == 1 ]]
    [[ "$(rg -c 'Build Rust Refloat configuration codec' \
      backends/apple/ios/float-view.xcodeproj/project.pbxproj)" == 3 ]]
    [[ "$(rg -c 'refloat-configuration-codec/src/records.rs' \
      backends/apple/ios/float-view.xcodeproj/project.pbxproj)" == 1 ]]
    [[ "$(rg -c 'libfloat_view_refloat_configuration_codec_apple_ffi.a' \
      backends/apple/ios/float-view.xcodeproj/project.pbxproj)" == 1 ]]
    [[ "$(rg -c -- '-lfloat_view_refloat_configuration_codec_apple_ffi' \
      backends/apple/ios/float-view.xcodeproj/project.pbxproj)" == 2 ]]
    [[ "$(rg -c 'Sources/CVescRefloatConfigurationCodec' \
      backends/apple/ios/float-view.xcodeproj/project.pbxproj)" == 2 ]]
    rg -q '^object RefloatConfigurationSchemaCodec' \
      backends/android/app/src/main/java/fail/acheron/floatview/protocol/refloat/RefloatConfigurationCodec.kt
    rg -q '^object RefloatConfigurationValueCodec' \
      backends/android/app/src/main/java/fail/acheron/floatview/protocol/refloat/RefloatConfigurationCodec.kt
    android_configuration_source='backends/android/app/src/main/java/fail/acheron/floatview/protocol/refloat/RefloatConfigurationCodec.kt'
    ! rg -n 'Inflater|SAXParserFactory|MessageDigest|ConfigParamsHandler|RawField|ConfigurationByteReader|crc32c|fun signature\(' \
      "$android_configuration_source"
    [[ "$(rg -c 'RustRefloatConfigurationCodecBridge\.(decodeSchema|decodeValues|encodeValues)\(' \
      "$android_configuration_source")" == 3 ]]
    [[ "$(rg -l 'RustRefloatConfigurationCodecBridge\.(decodeSchema|decodeValues|encodeValues)' \
      backends/android/app/src/main -g '*.kt')" == "$android_configuration_source" ]]
    ! rg -n 'refloat-configuration-codec-android-jni|RustRefloatConfigurationCodecBridge|float_view_refloat_configuration_codec_android_jni' \
      backends/apple
    just core-refloat-configuration-codec-test

android-rust-host:
    backends/core/build-android-bridge.sh frame host debug \
      "$PWD/backends/core/target/android/host/debug"
    backends/core/build-android-bridge.sh firmware-identity host debug \
      "$PWD/backends/core/target/android/identity-host/debug"
    backends/core/build-android-bridge.sh custom-configuration host debug \
      "$PWD/backends/core/target/android/custom-configuration-host/debug"
    backends/core/build-android-bridge.sh bms-values host debug \
      "$PWD/backends/core/target/android/bms-values-host/debug"
    backends/core/build-android-bridge.sh controller-data host debug \
      "$PWD/backends/core/target/android/controller-data-host/debug"
    backends/core/build-android-bridge.sh refloat-control host debug \
      "$PWD/backends/core/target/android/refloat-control-host/debug"
    backends/core/build-android-bridge.sh refloat-compatibility host debug \
      "$PWD/backends/core/target/android/refloat-compatibility-host/debug"
    backends/core/build-android-bridge.sh refloat-dynamic-telemetry host debug \
      "$PWD/backends/core/target/android/refloat-dynamic-telemetry-host/debug"
    backends/core/build-android-bridge.sh refloat-legacy-telemetry host debug \
      "$PWD/backends/core/target/android/refloat-legacy-telemetry-host/debug"
    backends/core/build-android-bridge.sh refloat-configuration-codec host debug \
      "$PWD/backends/core/target/android/refloat-configuration-codec-host/debug"

android-rust-codec-test:
    cd backends/android && ./gradlew testDebugUnitTest \
      --tests fail.acheron.floatview.protocol.RustVescFrameBridgeTest \
      --tests fail.acheron.floatview.protocol.RustVescFirmwareIdentityBridgeTest \
      --tests fail.acheron.floatview.protocol.RustVescCustomConfigurationBridgeTest \
      --tests fail.acheron.floatview.protocol.RustVescBmsValuesBridgeTest \
      --tests fail.acheron.floatview.protocol.RustVescControllerDataBridgeTest \
      --tests fail.acheron.floatview.protocol.RustRefloatControlBridgeTest \
      --tests fail.acheron.floatview.protocol.RustRefloatCompatibilityBridgeTest \
      --tests fail.acheron.floatview.protocol.RustRefloatDynamicTelemetryBridgeTest \
      --tests fail.acheron.floatview.protocol.RustRefloatLegacyTelemetryBridgeTest \
      --tests fail.acheron.floatview.protocol.refloat.RefloatConfigurationCodecTest \
      --tests fail.acheron.floatview.protocol.RefloatControlCodecTest \
      --tests fail.acheron.floatview.protocol.VescProtocolTest

android-rust-packaging:
    #!/usr/bin/env bash
    set -euo pipefail
    (cd backends/android && ./gradlew clean assembleDebug assembleRelease)
    expected="$(for library in \
      libfloat_view_vesc_frame_android_jni.so \
      libfloat_view_vesc_firmware_identity_android_jni.so \
      libfloat_view_vesc_custom_configuration_android_jni.so \
      libfloat_view_vesc_bms_values_android_jni.so \
      libfloat_view_vesc_controller_data_android_jni.so \
      libfloat_view_refloat_control_android_jni.so \
      libfloat_view_refloat_compatibility_android_jni.so \
      libfloat_view_refloat_dynamic_telemetry_android_jni.so \
      libfloat_view_refloat_legacy_telemetry_android_jni.so \
      libfloat_view_refloat_configuration_codec_android_jni.so; do
        printf '%s\n' \
          "lib/arm64-v8a/$library" \
          "lib/armeabi-v7a/$library" \
          "lib/x86/$library" \
          "lib/x86_64/$library"
      done | sort)"
    for apk in \
      backends/android/app/build/outputs/apk/debug/app-debug.apk \
      backends/android/app/build/outputs/apk/release/app-release-unsigned.apk; do
      [[ -f "$apk" ]] || { echo "Missing Android package: $apk" >&2; exit 1; }
      unzip -tq "$apk" >/dev/null
      actual="$(unzip -Z1 "$apk" \
        | rg '(^|/)libfloat_view_(vesc_(frame|firmware_identity|custom_configuration|bms_values|controller_data)|refloat_(control|compatibility|dynamic_telemetry|legacy_telemetry|configuration_codec))_android_jni\.so$' \
        | sort)"
      [[ "$actual" == "$expected" ]] || {
        echo "Unexpected Rust JNI library set in $apk:" >&2
        printf '%s\n' "$actual" >&2
        exit 1
      }
    done

android-rust-audit: android-rust-host
    #!/usr/bin/env bash
    set -euo pipefail
    [[ "$(cd backends/core && cargo tree --locked --offline \
      --package float-view-vesc-frame --edges normal --prefix none | wc -l | tr -d ' ')" == 1 ]]
    [[ "$(cd backends/core && cargo tree --locked --offline \
      --package float-view-vesc-firmware-identity --edges normal --prefix none | wc -l | tr -d ' ')" == 1 ]]
    [[ "$(cd backends/core && cargo tree --locked --offline \
      --package float-view-vesc-frame-apple-ffi --edges normal --depth 1 --prefix none | wc -l | tr -d ' ')" == 3 ]]
    [[ "$(cd backends/core && cargo tree --locked --offline \
      --package float-view-vesc-firmware-identity-android-jni --edges normal --depth 1 --prefix none | wc -l | tr -d ' ')" == 3 ]]
    [[ "$(cd backends/core && cargo tree --locked --offline \
      --package float-view-vesc-custom-configuration-android-jni --edges normal --depth 1 --prefix none | wc -l | tr -d ' ')" == 3 ]]
    [[ "$(cd backends/core && cargo tree --locked --offline \
      --package float-view-vesc-bms-values-android-jni --edges normal --depth 1 --prefix none | wc -l | tr -d ' ')" == 3 ]]
    [[ "$(cd backends/core && cargo tree --locked --offline \
      --package float-view-vesc-controller-data-android-jni --edges normal --depth 1 --prefix none | wc -l | tr -d ' ')" == 3 ]]
    [[ "$(cd backends/core && cargo tree --locked --offline \
      --package float-view-refloat-control-android-jni --edges normal --depth 1 --prefix none | wc -l | tr -d ' ')" == 3 ]]
    [[ "$(cd backends/core && cargo tree --locked --offline \
      --package float-view-refloat-compatibility-android-jni --edges normal --depth 1 --prefix none | wc -l | tr -d ' ')" == 3 ]]
    [[ "$(cd backends/core && cargo tree --locked --offline \
      --package float-view-refloat-dynamic-telemetry-android-jni --edges normal --depth 1 --prefix none | wc -l | tr -d ' ')" == 3 ]]
    [[ "$(cd backends/core && cargo tree --locked --offline \
      --package float-view-refloat-legacy-telemetry-android-jni --edges normal --depth 1 --prefix none | wc -l | tr -d ' ')" == 3 ]]
    [[ "$(cd backends/core && cargo tree --locked --offline \
      --package float-view-refloat-configuration-codec-android-jni --edges normal --depth 1 --prefix none | wc -l | tr -d ' ')" == 3 ]]
    [[ "$(cd backends/core && cargo tree --locked --offline \
      --package float-view-android-jni-support --edges normal --depth 1 --prefix none | wc -l | tr -d ' ')" == 2 ]]
    for manifest in \
      backends/core/vesc-frame-android-jni/Cargo.toml \
      backends/core/vesc-firmware-identity-android-jni/Cargo.toml \
      backends/core/vesc-custom-configuration-android-jni/Cargo.toml \
      backends/core/vesc-bms-values-android-jni/Cargo.toml \
      backends/core/vesc-controller-data-android-jni/Cargo.toml \
      backends/core/refloat-control-android-jni/Cargo.toml \
      backends/core/refloat-compatibility-android-jni/Cargo.toml \
      backends/core/refloat-dynamic-telemetry-android-jni/Cargo.toml \
      backends/core/refloat-legacy-telemetry-android-jni/Cargo.toml \
      backends/core/refloat-configuration-codec-android-jni/Cargo.toml; do
      rg -q '^float-view-android-jni-support = \{ path = "\.\./android-jni-support" \}$' "$manifest"
      ! rg -n '^jni-sys[[:space:]]*=' "$manifest"
    done
    [[ "$(rg -l '^jni-sys = "=0\.4\.1"$' backends/core -g Cargo.toml)" == \
      'backends/core/android-jni-support/Cargo.toml' ]]
    ! rg -n '^jni[[:space:]]*=' backends/core -g Cargo.toml
    [[ "$(rg -c 'rustAndroidJniSupportInputs' backends/android/app/build.gradle.kts)" == 2 ]]
    ! rg -n 'jni_sys|JNINativeInterface|functions\.(ExceptionCheck|GetArrayLength|IsSameObject|GetByteArrayRegion|SetByteArrayRegion|SetIntArrayRegion)|AssertUnwindSafe|catch_unwind|unsafe fn environment|fn (references_alias|any_alias|copy_bytes_from_java|copy_bytes_to_java|write_ints|write_metadata|write_length|write_scan_fields)' \
      backends/core/*-android-jni/src/lib.rs
    rg -q 'functions\.ExceptionCheck' backends/core/android-jni-support/src/lib.rs
    rg -q 'functions\.GetArrayLength' backends/core/android-jni-support/src/lib.rs
    rg -q 'functions\.IsSameObject' backends/core/android-jni-support/src/lib.rs
    rg -q 'functions\.GetByteArrayRegion' backends/core/android-jni-support/src/lib.rs
    rg -q 'functions\.SetByteArrayRegion' backends/core/android-jni-support/src/lib.rs
    rg -q 'functions\.SetIntArrayRegion' backends/core/android-jni-support/src/lib.rs
    rg -q 'catch_unwind' backends/core/android-jni-support/src/lib.rs
    ! rg -n 'unsafe\(no_mangle\)|extern[[:space:]]+"system"|Java_' \
      backends/core/android-jni-support
    rust_abi="$(sed -n 's/^pub const ABI_VERSION: jint = \([0-9][0-9]*\);$/\1/p' \
      backends/core/vesc-frame-android-jni/src/lib.rs)"
    kotlin_abi="$(sed -n 's/^[[:space:]]*private const val ABI_VERSION = \([0-9][0-9]*\)$/\1/p' \
      backends/android/app/src/main/java/fail/acheron/floatview/protocol/vesc/VescFrame.kt)"
    [[ -n "$rust_abi" && "$rust_abi" == "$kotlin_abi" ]] || {
      echo "Android VESC JNI ABI mismatch: Rust=$rust_abi Kotlin=$kotlin_abi" >&2
      exit 1
    }
    identity_rust_abi="$(sed -n 's/^pub const RECORD_VERSION: u32 = \([0-9][0-9]*\);$/\1/p' \
      backends/core/vesc-firmware-identity/src/lib.rs)"
    rg -q '^pub const ABI_VERSION: jint = RECORD_VERSION as jint;$' \
      backends/core/vesc-firmware-identity-android-jni/src/lib.rs
    identity_kotlin_abi="$(sed -n 's/^internal const val VESC_FIRMWARE_IDENTITY_ABI_VERSION = \([0-9][0-9]*\)$/\1/p' \
      backends/android/app/src/main/java/fail/acheron/floatview/protocol/vesc/RustVescFirmwareIdentityBridge.kt)"
    [[ -n "$identity_rust_abi" && "$identity_rust_abi" == "$identity_kotlin_abi" ]] || {
      echo "Android firmware-identity JNI ABI mismatch: Rust=$identity_rust_abi Kotlin=$identity_kotlin_abi" >&2
      exit 1
    }
    configuration_rust_abi="$(sed -n 's/^pub const ABI_VERSION: jint = \([0-9][0-9]*\);$/\1/p' \
      backends/core/vesc-custom-configuration-android-jni/src/lib.rs)"
    configuration_kotlin_abi="$(sed -n 's/^internal const val VESC_CUSTOM_CONFIGURATION_ABI_VERSION = \([0-9][0-9]*\)$/\1/p' \
      backends/android/app/src/main/java/fail/acheron/floatview/protocol/vesc/RustVescCustomConfigurationBridge.kt)"
    [[ -n "$configuration_rust_abi" && "$configuration_rust_abi" == "$configuration_kotlin_abi" ]] || {
      echo "Android custom-configuration JNI ABI mismatch: Rust=$configuration_rust_abi Kotlin=$configuration_kotlin_abi" >&2
      exit 1
    }
    bms_rust_abi="$(sed -n 's/^pub const ABI_VERSION: jint = \([0-9][0-9]*\);$/\1/p' \
      backends/core/vesc-bms-values-android-jni/src/lib.rs)"
    bms_kotlin_abi="$(sed -n 's/^internal const val VESC_BMS_VALUES_ABI_VERSION = \([0-9][0-9]*\)$/\1/p' \
      backends/android/app/src/main/java/fail/acheron/floatview/protocol/vesc/RustVescBmsValuesBridge.kt)"
    [[ -n "$bms_rust_abi" && "$bms_rust_abi" == "$bms_kotlin_abi" ]] || {
      echo "Android BMS-values JNI ABI mismatch: Rust=$bms_rust_abi Kotlin=$bms_kotlin_abi" >&2
      exit 1
    }
    controller_data_rust_abi="$(sed -n 's/^pub const ABI_VERSION: jint = \([0-9][0-9]*\);$/\1/p' \
      backends/core/vesc-controller-data-android-jni/src/lib.rs)"
    controller_data_kotlin_abi="$(sed -n 's/^internal const val VESC_CONTROLLER_DATA_ABI_VERSION = \([0-9][0-9]*\)$/\1/p' \
      backends/android/app/src/main/java/fail/acheron/floatview/protocol/vesc/RustVescControllerDataBridge.kt)"
    [[ -n "$controller_data_rust_abi" && "$controller_data_rust_abi" == "$controller_data_kotlin_abi" ]] || {
      echo "Android controller-data JNI ABI mismatch: Rust=$controller_data_rust_abi Kotlin=$controller_data_kotlin_abi" >&2
      exit 1
    }
    refloat_rust_abi="$(sed -n 's/^pub const ABI_VERSION: jint = \([0-9][0-9]*\);$/\1/p' \
      backends/core/refloat-control-android-jni/src/lib.rs)"
    refloat_kotlin_abi="$(sed -n 's/^internal const val REFLOAT_CONTROL_ABI_VERSION = \([0-9][0-9]*\)$/\1/p' \
      backends/android/app/src/main/java/fail/acheron/floatview/protocol/vesc/RustRefloatControlBridge.kt)"
    [[ -n "$refloat_rust_abi" && "$refloat_rust_abi" == "$refloat_kotlin_abi" ]] || {
      echo "Android Refloat-control JNI ABI mismatch: Rust=$refloat_rust_abi Kotlin=$refloat_kotlin_abi" >&2
      exit 1
    }
    compatibility_rust_abi="$(sed -n 's/^pub const RECORD_VERSION: u8 = \([0-9][0-9]*\);$/\1/p' \
      backends/core/refloat-compatibility/src/lib.rs)"
    rg -q '^pub const ABI_VERSION: jint = RECORD_VERSION as jint;$' \
      backends/core/refloat-compatibility-android-jni/src/lib.rs
    compatibility_kotlin_abi="$(sed -n 's/^internal const val REFLOAT_COMPATIBILITY_ABI_VERSION = \([0-9][0-9]*\)$/\1/p' \
      backends/android/app/src/main/java/fail/acheron/floatview/protocol/RustRefloatCompatibilityBridge.kt)"
    [[ -n "$compatibility_rust_abi" && "$compatibility_rust_abi" == "$compatibility_kotlin_abi" ]] || {
      echo "Android Refloat compatibility JNI ABI mismatch: Rust=$compatibility_rust_abi Kotlin=$compatibility_kotlin_abi" >&2
      exit 1
    }
    dynamic_telemetry_rust_abi="$(sed -n 's/^pub const RECORD_VERSION: u8 = \([0-9][0-9]*\);$/\1/p' \
      backends/core/refloat-dynamic-telemetry/src/lib.rs)"
    rg -q '^pub const ABI_VERSION: jint = RECORD_VERSION as jint;$' \
      backends/core/refloat-dynamic-telemetry-android-jni/src/lib.rs
    dynamic_telemetry_kotlin_abi="$(sed -n 's/^internal const val REFLOAT_DYNAMIC_TELEMETRY_ABI_VERSION = \([0-9][0-9]*\)$/\1/p' \
      backends/android/app/src/main/java/fail/acheron/floatview/protocol/RustRefloatDynamicTelemetryBridge.kt)"
    [[ -n "$dynamic_telemetry_rust_abi" && "$dynamic_telemetry_rust_abi" == "$dynamic_telemetry_kotlin_abi" ]] || {
      echo "Android Refloat dynamic-telemetry JNI ABI mismatch: Rust=$dynamic_telemetry_rust_abi Kotlin=$dynamic_telemetry_kotlin_abi" >&2
      exit 1
    }
    legacy_rust_abi="$(sed -n 's/^pub const RECORD_VERSION: u32 = \([0-9][0-9]*\);$/\1/p' \
      backends/core/refloat-legacy-telemetry/src/lib.rs)"
    rg -q '^pub const ABI_VERSION: jint = RECORD_VERSION as jint;$' \
      backends/core/refloat-legacy-telemetry-android-jni/src/lib.rs
    legacy_kotlin_abi="$(sed -n 's/^internal const val REFLOAT_LEGACY_TELEMETRY_ABI_VERSION = \([0-9][0-9]*\)$/\1/p' \
      backends/android/app/src/main/java/fail/acheron/floatview/protocol/RustRefloatLegacyTelemetryBridge.kt)"
    [[ -n "$legacy_rust_abi" && "$legacy_rust_abi" == "$legacy_kotlin_abi" ]] || {
      echo "Android Refloat legacy-telemetry JNI ABI mismatch: Rust=$legacy_rust_abi Kotlin=$legacy_kotlin_abi" >&2
      exit 1
    }
    refloat_configuration_rust_abi="$(sed -n 's/^pub const ABI_VERSION: jint = \([0-9][0-9]*\);$/\1/p' \
      backends/core/refloat-configuration-codec-android-jni/src/lib.rs)"
    refloat_configuration_kotlin_abi="$(sed -n 's/^internal const val REFLOAT_CONFIGURATION_CODEC_ABI_VERSION = \([0-9][0-9]*\)$/\1/p' \
      backends/android/app/src/main/java/fail/acheron/floatview/protocol/refloat/RefloatConfigurationCodec.kt)"
    [[ -n "$refloat_configuration_rust_abi" && "$refloat_configuration_rust_abi" == "$refloat_configuration_kotlin_abi" ]] || {
      echo "Android Refloat configuration-codec JNI ABI mismatch: Rust=$refloat_configuration_rust_abi Kotlin=$refloat_configuration_kotlin_abi" >&2
      exit 1
    }
    ! git ls-files | rg '\.(a|so|dylib|xcframework)$'
    android_main='backends/android/app/src/main/java/fail/acheron/floatview'
    codec_source="$android_main/protocol/vesc/VescFrame.kt"
    bridge_source="$android_main/protocol/vesc/RustVescFrameBridge.kt"
    ! rg -n -e 'object VescCrc' -e 'VescCrc\.checksum' -e '0x1021' \
      -e 'fun checksum\(' -e 'VescFrameReference' -e 'CheckpointKotlinVesc' \
      -e 'PreCutoverKotlinStreamReference' -e 'buffer\.first\(\)' \
      -e 'headerSize' -e 'frameSize[[:space:]]*=' -e 'setOf\(2, 3\)' \
      -e 'buffer\[[12]\]\.toInt\(\)' \
      -e 'payload\.size <= 255' -e 'header \+ payload' \
      -e 'frame\.last\(\)\.toInt\(\) != 3' \
      -e 'frame\.copyOfRange\(headerSize' \
      backends/android/app/src/main backends/android/app/src/test -g '*.kt'
    [[ "$(rg -l 'object VescFrameCodec' "$android_main" -g '*.kt')" == "$codec_source" ]]
    [[ "$(rg -l 'class VescFrameStreamDecoder' "$android_main" -g '*.kt')" == "$codec_source" ]]
    [[ "$(rg -c 'RustVescFrameBridge\.encode\(' "$codec_source")" == 1 ]]
    [[ "$(rg -c 'RustVescFrameBridge\.decodeExact\(' "$codec_source")" == 1 ]]
    [[ "$(rg -c 'RustVescFrameBridge\.scanNext\(' "$codec_source")" == 1 ]]
    [[ "$(rg -l 'RustVescFrameBridge\.(encode|decodeExact|scanNext)' "$android_main" -g '*.kt')" == "$codec_source" ]]
    [[ "$(rg -l 'external fun (encode|decodeExact|scanNext)' \
      "$bridge_source")" == \
      "$bridge_source" ]]
    identity_source="$android_main/protocol/vesc/VescFirmwareIdentityDecoder.kt"
    identity_bridge_source="$android_main/protocol/vesc/RustVescFirmwareIdentityBridge.kt"
    ! rg -n -e 'terminatedBytes' -e 'extendedFirmwareName' -e 'uuidStart' \
      -e 'commonEnd' -e 'firstOrNull \{ payload\[it\]' -e 'tailLength' \
      "$identity_source"
    [[ "$(rg -c 'RustVescFirmwareIdentityBridge\.decode\(' "$identity_source")" == 1 ]]
    [[ "$(rg -l 'RustVescFirmwareIdentityBridge\.decode\(' "$android_main" -g '*.kt')" == \
      "$identity_source" ]]
    [[ "$(rg -l 'external fun (abiVersion|decode)' "$identity_bridge_source")" == \
      "$identity_bridge_source" ]]
    configuration_source="$android_main/protocol/vesc/VescCustomConfigurationCodec.kt"
    configuration_bridge_source="$android_main/protocol/vesc/RustVescCustomConfigurationBridge.kt"
    ! rg -n -e 'positiveI32' -e 'nonnegativeI32' -e 'private fun i32' \
      -e 'payload\.firstOrNull' -e 'payload\.copyOfRange\(10' \
      -e 'bytes\.size > MAXIMUM_CHUNK_BYTES' \
      "$configuration_source"
    [[ "$(rg -c 'RustVescCustomConfigurationBridge\.encode\(' "$configuration_source")" == 1 ]]
    [[ "$(rg -c 'RustVescCustomConfigurationBridge\.decode\(' "$configuration_source")" == 1 ]]
    [[ "$(rg -l 'RustVescCustomConfigurationBridge\.(encode|decode)' "$android_main" -g '*.kt')" == \
      "$configuration_source" ]]
    [[ "$(rg -l 'external fun (abiVersion|encode|decode)' "$configuration_bridge_source")" == \
      "$configuration_bridge_source" ]]
    bms_source="$android_main/protocol/vesc/VescBmsValuesDecoder.kt"
    bms_protocol_source="$android_main/protocol/vesc/VescProtocolService.kt"
    bms_bridge_source="$android_main/protocol/vesc/RustVescBmsValuesBridge.kt"
    ! rg -n -e 'cellCount[[:space:]]*= reader' -e 'temperatureCount[[:space:]]*= reader' \
      -e 'terminatedUtf8' -e 'maxContentBytes' -e 'lifetimeChargeAmpHours[[:space:]]*= reader' \
      -e 'reportedCanId[[:space:]]*= reader' \
      "$bms_protocol_source"
    [[ "$(rg -c 'VescBmsValuesDecoder\.decode\(' "$bms_protocol_source")" == 1 ]]
    [[ "$(rg -c 'RustVescBmsValuesBridge\.decode\(' "$bms_source")" == 1 ]]
    [[ "$(rg -l 'RustVescBmsValuesBridge\.decode\(' "$android_main" -g '*.kt')" == \
      "$bms_source" ]]
    [[ "$(rg -l 'external fun (abiVersion|decode)' "$bms_bridge_source")" == \
      "$bms_bridge_source" ]]
    controller_data_source="$android_main/protocol/vesc/VescControllerDataDecoder.kt"
    controller_data_protocol_source="$android_main/protocol/vesc/VescProtocolService.kt"
    controller_data_bridge_source="$android_main/protocol/vesc/RustVescControllerDataBridge.kt"
    ! rg -n -e 'private class ByteReader' -e 'float32Auto' -e 'batterySeriesCellOffsets' \
      -e 'Math\.scalb' -e 'VescFault\.fromCode' \
      "$controller_data_protocol_source"
    [[ "$(rg -c 'VescControllerDataDecoder\.realtime\(' "$controller_data_protocol_source")" == 1 ]]
    [[ "$(rg -c 'VescControllerDataDecoder\.currentLimits\(' "$controller_data_protocol_source")" == 1 ]]
    [[ "$(rg -c 'RustVescControllerDataBridge\.decode\(' "$controller_data_source")" == 1 ]]
    [[ "$(rg -l 'RustVescControllerDataBridge\.decode\(' "$android_main" -g '*.kt')" == \
      "$controller_data_source" ]]
    [[ "$(rg -l 'external fun (abiVersion|decode)' "$controller_data_bridge_source")" == \
      "$controller_data_bridge_source" ]]
    refloat_source="$android_main/protocol/refloat/RefloatControlCodec.kt"
    refloat_bridge_source="$android_main/protocol/vesc/RustRefloatControlBridge.kt"
    ! rg -n -e 'RefloatWire\.request\(' -e 'val mask = when' \
      -e 'payload\[2\].*RefloatCommand' -e 'payload\[3\].*0xfc' \
      "$refloat_source"
    [[ "$(rg -c 'RustRefloatControlBridge\.encode\(' "$refloat_source")" == 1 ]]
    [[ "$(rg -c 'RustRefloatControlBridge\.decodeLighting\(' "$refloat_source")" == 1 ]]
    [[ "$(rg -l 'RustRefloatControlBridge\.(encode|decodeLighting)' "$android_main" -g '*.kt')" == \
      "$refloat_source" ]]
    [[ "$(rg -l 'external fun (abiVersion|encode|decodeLighting)' "$refloat_bridge_source")" == \
      "$refloat_bridge_source" ]]
    compatibility_source="$android_main/protocol/refloat/RefloatCompatibility.kt"
    compatibility_bridge_source="$android_main/protocol/RustRefloatCompatibilityBridge.kt"
    [[ ! -f "$android_main/protocol/refloat/RefloatInfo.kt" ]]
    [[ "$(rg -c 'RustRefloatCompatibilityBridge\.infoRequest\(' "$compatibility_source")" == 1 ]]
    [[ "$(rg -c 'RustRefloatCompatibilityBridge\.decodeInfo\(' "$compatibility_source")" == 1 ]]
    [[ "$(rg -c 'RustRefloatCompatibilityBridge\.admitLayout\(' "$compatibility_source")" == 1 ]]
    [[ "$(rg -l 'RustRefloatCompatibilityBridge\.(infoRequest|decodeInfo|admitLayout)' \
      "$android_main" -g '*.kt')" == "$compatibility_source" ]]
    [[ "$(rg -l 'external fun (abiVersion|infoRequest|decodeInfo|admitLayout)' \
      "$compatibility_bridge_source")" == "$compatibility_bridge_source" ]]
    ! rg -n -e 'INFO_V2_(MAJOR|MINOR|PATCH)_OFFSET' -e 'requiredFields' \
      -e 'requiredRuntimeFields' -e 'supportsDynamicLayout' -e 'Set\(required' \
      -e 'sourceVersion' -e 'static (val|let) entries' \
      backends/android/app/src/main -g '*.kt'
    ! rg -n -e 'INFO_PROTOCOL_VERSION' -e 'INFO_FLAGS' \
      backends/android/app/src/main -g '*.kt'
    ! rg -n -e 'COMPATIBILITY_ENTRIES' -e 'fn (serialize|encode).*compatibility' \
      backends/core/refloat-compatibility-android-jni/src/lib.rs
    ! rg -n 'RustRefloatCompatibilityBridge|refloat-compatibility-android-jni|float_view_refloat_compatibility_android_jni' \
      backends/apple
    legacy_source="$android_main/protocol/refloat/RefloatLegacy.kt"
    legacy_bridge_source="$android_main/protocol/RustRefloatLegacyTelemetryBridge.kt"
    ! rg -n -e 'signed16' -e 'unsigned32' -e 'decodeFloat32' \
      -e 'legacySetpointAdjustment' -e 'legacySemanticState' \
      "$legacy_source"
    [[ "$(rg -c 'RustRefloatLegacyTelemetryBridge\.decode\(' "$legacy_source")" == 1 ]]
    [[ "$(rg -l 'RustRefloatLegacyTelemetryBridge\.decode\(' "$android_main" -g '*.kt')" == \
      "$legacy_source" ]]
    [[ "$(rg -l 'external fun (abiVersion|decode)' "$legacy_bridge_source")" == \
      "$legacy_bridge_source" ]]
    refloat_configuration_source="$android_main/protocol/refloat/RefloatConfigurationCodec.kt"
    ! rg -n -e 'Inflater' -e 'SAXParserFactory' -e 'MessageDigest' \
      -e 'ConfigParamsHandler' -e 'ConfigurationByteReader' -e 'crc32c' \
      -e 'fun signature\(' "$refloat_configuration_source"
    [[ "$(rg -c 'RustRefloatConfigurationCodecBridge\.decodeSchema\(' "$refloat_configuration_source")" == 1 ]]
    [[ "$(rg -c 'RustRefloatConfigurationCodecBridge\.decodeValues\(' "$refloat_configuration_source")" == 1 ]]
    [[ "$(rg -c 'RustRefloatConfigurationCodecBridge\.encodeValues\(' "$refloat_configuration_source")" == 1 ]]
    [[ "$(rg -l 'external fun (abiVersion|decodeSchema|decodeValues|encodeValues)' \
      "$refloat_configuration_source")" == "$refloat_configuration_source" ]]
    if ANDROID_NDK_HOME=/private/tmp/float-view-missing-ndk \
      backends/core/build-android-bridge.sh frame \
      android arm64-v8a debug /private/tmp/float-view-unreachable-output >/dev/null 2>&1; then
      echo 'Android Rust helper accepted a missing pinned NDK.' >&2
      exit 1
    fi
    if ANDROID_NDK_HOME=/private/tmp/float-view-missing-ndk \
      backends/core/build-android-bridge.sh custom-configuration \
      android arm64-v8a debug /private/tmp/float-view-unreachable-custom-configuration-output >/dev/null 2>&1; then
      echo 'Android Rust custom-configuration helper accepted a missing pinned NDK.' >&2
      exit 1
    fi
    if ANDROID_NDK_HOME=/private/tmp/float-view-missing-ndk \
      backends/core/build-android-bridge.sh firmware-identity \
      android arm64-v8a debug /private/tmp/float-view-unreachable-identity-output >/dev/null 2>&1; then
      echo 'Android Rust identity helper accepted a missing pinned NDK.' >&2
      exit 1
    fi
    if ANDROID_NDK_HOME=/private/tmp/float-view-missing-ndk \
      backends/core/build-android-bridge.sh bms-values \
      android arm64-v8a debug /private/tmp/float-view-unreachable-bms-values-output >/dev/null 2>&1; then
      echo 'Android Rust BMS-values helper accepted a missing pinned NDK.' >&2
      exit 1
    fi
    if ANDROID_NDK_HOME=/private/tmp/float-view-missing-ndk \
      backends/core/build-android-bridge.sh controller-data \
      android arm64-v8a debug /private/tmp/float-view-unreachable-controller-data-output >/dev/null 2>&1; then
      echo 'Android Rust controller-data helper accepted a missing pinned NDK.' >&2
      exit 1
    fi
    if ANDROID_NDK_HOME=/private/tmp/float-view-missing-ndk \
      backends/core/build-android-bridge.sh refloat-control \
      android arm64-v8a debug /private/tmp/float-view-unreachable-refloat-control-output >/dev/null 2>&1; then
      echo 'Android Rust Refloat-control helper accepted a missing pinned NDK.' >&2
      exit 1
    fi

setup:
    cd frontend && npm ci
    cd conformance && npm ci
    pio pkg install -d tools/vesc-test-device
    mkdir -p vendor
    [ -e "vendor/bldc" ] || git clone https://github.com/vedderb/bldc "vendor/bldc"
    [ -e "vendor/refloat" ] || git clone https://github.com/lukash/refloat "vendor/refloat"
    [ -e "vendor/vesc_tool" ] || git clone https://github.com/vedderb/vesc_tool "vendor/vesc_tool"
    [ -e "vendor/vesc_bms_fw" ] || git clone https://github.com/vedderb/vesc_bms_fw "vendor/vesc_bms_fw"
    [ -e "vendor/vesc_express" ] || git clone https://github.com/vedderb/vesc_express "vendor/vesc_express"
    [ -e "vendor/float-view" ] || git clone https://github.com/acheronfail/float-view "vendor/float-view"

lint:
    just core-lint
    cd frontend && npm run format:check
    cd conformance && npm run format:check
    swiftformat --config .swiftformat --lint backends/apple

fmt:
    cd frontend && npm run format:write
    cd conformance && npm run format:write
    swiftformat --config .swiftformat backends/apple
    cd backends/core && cargo fmt

dev platform="mock" target="":
    cd conformance && node --strip-types src/dev.ts "{{ platform }}" "{{ target }}"

list platform:
    #!/usr/bin/env bash
    set -euo pipefail
    case "{{ platform }}" in
      ios)
        xcrun xctrace list devices |
          awk '
            /^== Devices ==$/ { physical=1; next }
            /^== Simulators ==$/ { physical=0 }
            physical && /iPhone|iPad|iPod/ { print }
          '
        ;;
      android) adb devices -l;;
      *) echo 'Usage: just list android|ios' >&2; exit 2 ;;
    esac

emulate platform: build
    #!/usr/bin/env bash
    set -euo pipefail
    case "{{ platform }}" in
      android)
        emulator -avd Float_View_Conformance -no-window -no-audio
        ;;
      ios)
        simulator="${FLOAT_VIEW_IOS_SIMULATOR:-iPhone 17 Pro}"
        xcrun simctl boot "$simulator" >/dev/null 2>&1 || true
        xcrun simctl bootstatus "$simulator" -b
        ;;
      *) echo 'Usage: just emulate android|ios' >&2; exit 2 ;;
    esac

build:
    cd frontend && npm run build
    cp frontend/build/index.html backends/apple/macos/Resources/index.html
    mkdir -p backends/apple/macos/Resources/sql
    cp backends/sql/catalog.sql backends/apple/macos/Resources/sql/catalog.sql
    cp backends/sql/session.sql backends/apple/macos/Resources/sql/session.sql

run platform target="": build
    #!/usr/bin/env bash
    set -euo pipefail
    case "{{ platform }}" in
      android)
        (cd backends/android && ./gradlew assembleDebug)
        serial="{{ target }}"
        adb_args=()
        [[ -z "$serial" ]] || adb_args=(-s "$serial")
        adb "${adb_args[@]}" install -r backends/android/app/build/outputs/apk/debug/app-debug.apk
        adb "${adb_args[@]}" logcat -c || true
        adb "${adb_args[@]}" shell am start -W \
          -n fail.acheron.floatview/fail.acheron.floatview.ui.MainActivity
        process_id="$(adb "${adb_args[@]}" shell pidof -s fail.acheron.floatview | tr -d '\r')"
        [[ "$process_id" =~ ^[0-9]+$ ]] || { echo 'Float View did not start' >&2; exit 1; }
        echo "Streaming Float View logs from Android process $process_id; press Ctrl-C to stop."
        exec adb "${adb_args[@]}" logcat --pid="$process_id" -v color
        ;;
      ios)
        project="$PWD/backends/apple/ios/float-view.xcodeproj"
        target="{{ target }}"
        if [[ -z "$target" ]]; then
          xcodebuild -project "$project" -scheme float-view \
            -destination 'generic/platform=iOS Simulator' -derivedDataPath /tmp/float-view-ios \
            CODE_SIGNING_ALLOWED=NO build
        elif xcrun simctl list devices available | grep -F "$target" >/dev/null; then
          derived='/tmp/float-view-ios'
          xcrun simctl boot "$target" >/dev/null 2>&1 || true
          xcrun simctl bootstatus "$target" -b
          xcodebuild -project "$project" -scheme float-view -sdk iphonesimulator \
            -destination "id=$target" -derivedDataPath "$derived" \
            CODE_SIGNING_ALLOWED=NO build
          xcrun simctl install "$target" \
            "$derived/Build/Products/Debug-iphonesimulator/float-view.app"
          xcrun simctl launch --terminate-running-process "$target" fail.acheron.float-view
        else
          derived='/tmp/float-view-ios-physical'
          xcodebuild -project "$project" -scheme float-view -configuration Debug \
            -destination "platform=iOS,id=$target" -destination-timeout 30 \
            -derivedDataPath "$derived" -allowProvisioningUpdates build
          xcrun devicectl device install app --device "$target" \
            "$derived/Build/Products/Debug-iphoneos/float-view.app"
          xcrun devicectl device process launch --device "$target" \
            --terminate-existing --console fail.acheron.float-view
        fi
        ;;
      macos)
        frontend_html="$PWD/frontend/build/index.html"
        just apple-rust-bridges
        (cd backends/apple && \
          FLOAT_VIEW_FRONTEND_HTML="$frontend_html" \
          swift run --scratch-path /tmp/float-view-swift-build FloatViewMac)
        ;;
      *) echo 'Usage: just run android|ios|macos [target]' >&2; exit 2 ;;
    esac

test suite="all":
    #!/usr/bin/env bash
    set -euo pipefail
    case "{{ suite }}" in
      frontend) (cd frontend && npm run check && npm test) ;;
      conformance) (cd conformance && npm run check && npm test && npm run mock) ;;
      apple)
        just apple-rust-bridges
        (cd backends/apple && swift test --scratch-path /tmp/float-view-swift-build)
        ;;
      android) (cd backends/android && ./gradlew testDebugUnitTest) ;;
      core) just core-test ;;
      fixture) sh tools/vesc-test-device/test/run.sh ;;
      all)
        just test frontend
        just test conformance
        just test apple
        just test android
        just test core
        just test fixture
        ;;
      *) echo 'Usage: just test frontend|conformance|apple|android|core|fixture|all' >&2; exit 2 ;;
    esac

conformance target:
    cd conformance && npm run {{ target }}

conformance-physical platform fixture_port target="": build
    #!/usr/bin/env bash
    set -euo pipefail
    cd conformance
    npm run physical -- "{{ platform }}" "{{ fixture_port }}" "{{ target }}"

fixture action port="":
    #!/usr/bin/env bash
    set -euo pipefail
    case "{{ action }}" in
      check) sh tools/vesc-test-device/test/run.sh ;;
      setup) pio pkg install -d tools/vesc-test-device ;;
      ports) pio device list ;;
      build) pio run -d tools/vesc-test-device ;;
      flash)
        [[ -n "{{ port }}" ]] || { echo 'Pass the fixture serial port.' >&2; exit 2; }
        pio run -d tools/vesc-test-device -t upload --upload-port "{{ port }}"
        ;;
      monitor)
        [[ -n "{{ port }}" ]] || { echo 'Pass the fixture serial port.' >&2; exit 2; }
        pio device monitor --port "{{ port }}" --baud 115200 --echo
        ;;
      dev)
        cd conformance
        FLOAT_VIEW_FIXTURE_PORT="{{ port }}" npm run fixture:dev
        ;;
      *) echo 'Usage: just fixture setup|check|ports|build|flash|monitor|dev [port]' >&2; exit 2 ;;
    esac

audit:
    #!/usr/bin/env bash
    set -euo pipefail
    for removed in api web apple ios android logging fixtures; do
      [[ ! -e "$removed" ]] || { echo "Obsolete path remains: $removed" >&2; exit 1; }
    done
    [[ ! -d backends/android/api-models ]]
    [[ ! -d backends/android/app/src/main/java/fail/acheron/floatview/backend ]]
    [[ ! -d backends/apple/Sources/FloatViewBackend/server ]]
    [[ ! -d backends/apple/Sources/FloatViewMac ]]
    [[ -f rust-toolchain.toml ]]
    [[ -f backends/core/Cargo.lock ]]
    [[ -f backends/core/android-jni-support/Cargo.toml ]]
    [[ -f backends/core/android-jni-support/src/lib.rs ]]
    [[ -f backends/core/apple-ffi-support/Cargo.toml ]]
    [[ -f backends/core/apple-ffi-support/src/lib.rs ]]
    [[ -f backends/core/bridge-build-catalog.sh ]]
    [[ -f backends/core/build-apple-bridge.sh ]]
    [[ -f backends/core/build-android-bridge.sh ]]
    [[ -f backends/core/refloat-compatibility/Cargo.toml ]]
    [[ -f backends/core/refloat-compatibility-android-jni/Cargo.toml ]]
    [[ -f backends/core/refloat-compatibility-apple-ffi/Cargo.toml ]]
    [[ -f backends/core/refloat-compatibility-apple-ffi/tests/abi.c ]]
    [[ -f backends/core/refloat-configuration-codec/Cargo.toml ]]
    [[ -f backends/core/refloat-configuration-service/Cargo.toml ]]
    [[ -f backends/core/refloat-configuration-codec-android-jni/Cargo.toml ]]
    [[ -f backends/core/refloat-configuration-codec-apple-ffi/Cargo.toml ]]
    [[ -f backends/core/refloat-configuration-codec-apple-ffi/tests/abi.c ]]
    [[ -f backends/core/vesc-controller-data/Cargo.toml ]]
    [[ -f backends/core/vesc-controller-data-android-jni/Cargo.toml ]]
    [[ -f backends/core/vesc-controller-data-apple-ffi/Cargo.toml ]]
    [[ -f backends/core/refloat-control/Cargo.toml ]]
    [[ -f backends/core/refloat-control-android-jni/Cargo.toml ]]
    [[ -f backends/core/refloat-control-apple-ffi/Cargo.toml ]]
    [[ -f backends/core/refloat-dynamic-telemetry/Cargo.toml ]]
    [[ -f backends/core/refloat-dynamic-telemetry-android-jni/Cargo.toml ]]
    [[ -f backends/core/refloat-dynamic-telemetry-apple-ffi/Cargo.toml ]]
    [[ -f backends/core/refloat-dynamic-telemetry-apple-ffi/tests/abi.c ]]
    [[ -f backends/core/refloat-legacy-telemetry/Cargo.toml ]]
    [[ -f backends/core/refloat-legacy-telemetry-android-jni/Cargo.toml ]]
    [[ -f backends/core/refloat-legacy-telemetry-apple-ffi/Cargo.toml ]]
    [[ -f backends/core/refloat-legacy-telemetry-apple-ffi/tests/abi.c ]]
    [[ -f backends/core/refloat-telemetry-service/Cargo.toml ]]
    [[ -f backends/core/vesc-bms-values/Cargo.toml ]]
    [[ -f backends/core/vesc-bms-values-android-jni/Cargo.toml ]]
    [[ -f backends/core/vesc-bms-values-apple-ffi/Cargo.toml ]]
    [[ -f backends/core/vesc-custom-configuration/Cargo.toml ]]
    [[ -f backends/core/vesc-custom-configuration-android-jni/Cargo.toml ]]
    [[ -f backends/core/vesc-custom-configuration-apple-ffi/Cargo.toml ]]
    [[ -f backends/core/vesc-firmware-identity/Cargo.toml ]]
    [[ -f backends/core/vesc-compatibility/Cargo.toml ]]
    [[ -f backends/core/vesc-firmware-identity-android-jni/Cargo.toml ]]
    [[ -f backends/core/vesc-firmware-identity-apple-ffi/Cargo.toml ]]
    [[ -f backends/core/vesc-frame/Cargo.toml ]]
    [[ -f backends/core/vesc-frame-apple-ffi/Cargo.toml ]]
    [[ -f backends/core/vesc-frame-android-jni/Cargo.toml ]]
    [[ -f backends/core/vesc-request-scheduler/Cargo.toml ]]
    [[ -f backends/core/controller-discovery/Cargo.toml ]]
    [[ -f backends/core/auxiliary-node-discovery/Cargo.toml ]]
    [[ -f backends/core/vesc-bms-telemetry-service/Cargo.toml ]]
    [[ -f backends/core/connection-application-coordinator/Cargo.toml ]]
    [[ -f backends/core/session-history-kernel/Cargo.toml ]]
    [[ -f backends/core/device-mutation-coordinator/Cargo.toml ]]
    [[ -f backends/apple/Sources/CVescFrame/include/CVescFrame.h ]]
    [[ -f backends/apple/Sources/CVescFrame/module.modulemap ]]
    [[ -f backends/apple/Sources/CVescCustomConfiguration/include/CVescCustomConfiguration.h ]]
    [[ -f backends/apple/Sources/CVescCustomConfiguration/module.modulemap ]]
    [[ -f backends/apple/Sources/CVescBmsValues/include/CVescBmsValues.h ]]
    [[ -f backends/apple/Sources/CVescBmsValues/module.modulemap ]]
    [[ -f backends/apple/Sources/CVescControllerData/include/CVescControllerData.h ]]
    [[ -f backends/apple/Sources/CVescControllerData/module.modulemap ]]
    [[ -f backends/apple/Sources/CVescFirmwareIdentity/include/CVescFirmwareIdentity.h ]]
    [[ -f backends/apple/Sources/CVescFirmwareIdentity/module.modulemap ]]
    [[ -f backends/apple/Sources/CVescRefloatCompatibility/include/CVescRefloatCompatibility.h ]]
    [[ -f backends/apple/Sources/CVescRefloatCompatibility/module.modulemap ]]
    [[ -f backends/apple/Sources/CVescRefloatConfigurationCodec/include/CVescRefloatConfigurationCodec.h ]]
    [[ -f backends/apple/Sources/CVescRefloatConfigurationCodec/module.modulemap ]]
    [[ -f backends/apple/Sources/CVescRefloatControl/include/CVescRefloatControl.h ]]
    [[ -f backends/apple/Sources/CVescRefloatControl/module.modulemap ]]
    [[ -f backends/apple/Sources/CVescRefloatDynamicTelemetry/include/CVescRefloatDynamicTelemetry.h ]]
    [[ -f backends/apple/Sources/CVescRefloatDynamicTelemetry/module.modulemap ]]
    [[ -f backends/apple/Sources/CVescRefloatLegacyTelemetry/include/CVescRefloatLegacyTelemetry.h ]]
    [[ -f backends/apple/Sources/CVescRefloatLegacyTelemetry/module.modulemap ]]
    unexpected_core="$(find backends/core -mindepth 1 -maxdepth 1 -type d \
      ! -name target ! -name android-jni-support ! -name apple-ffi-support \
      ! -name refloat-compatibility ! -name refloat-compatibility-android-jni \
      ! -name refloat-compatibility-apple-ffi \
      ! -name refloat-configuration-codec \
      ! -name refloat-configuration-service \
      ! -name refloat-configuration-codec-android-jni \
      ! -name refloat-configuration-codec-apple-ffi \
      ! -name refloat-control ! -name refloat-control-android-jni \
      ! -name refloat-control-apple-ffi ! -name refloat-dynamic-telemetry \
      ! -name refloat-dynamic-telemetry-android-jni \
      ! -name refloat-dynamic-telemetry-apple-ffi \
      ! -name refloat-legacy-telemetry \
      ! -name refloat-legacy-telemetry-apple-ffi \
      ! -name refloat-legacy-telemetry-android-jni \
      ! -name refloat-telemetry-service \
      ! -name vesc-bms-values ! -name vesc-bms-values-android-jni \
      ! -name vesc-bms-values-apple-ffi \
      ! -name vesc-controller-data ! -name vesc-controller-data-android-jni \
      ! -name vesc-controller-data-apple-ffi \
      ! -name vesc-custom-configuration ! -name vesc-firmware-identity \
      ! -name vesc-compatibility \
      ! -name vesc-custom-configuration-android-jni \
      ! -name vesc-custom-configuration-apple-ffi \
      ! -name vesc-firmware-identity-android-jni ! -name vesc-firmware-identity-apple-ffi \
      ! -name vesc-frame ! -name vesc-frame-apple-ffi \
      ! -name vesc-frame-android-jni ! -name vesc-request-scheduler \
      ! -name controller-discovery ! -name auxiliary-node-discovery \
      ! -name vesc-bms-telemetry-service \
      ! -name connection-application-coordinator \
      ! -name session-history-kernel \
      ! -name device-mutation-coordinator -print -quit)"
    [[ -z "$unexpected_core" ]] || {
      echo "Unexpected Rust core workspace member: $unexpected_core" >&2
      exit 1
    }
    [[ "$(find backends/core/apple-ffi-support -mindepth 1 -maxdepth 1 -type d \
      ! -name src -print -quit)" == "" ]]
    [[ "$(find backends/core/android-jni-support -mindepth 1 -maxdepth 1 -type d \
      ! -name src -print -quit)" == "" ]]
    [[ "$(find backends/core/refloat-compatibility -mindepth 1 -maxdepth 1 -type d \
      ! -name src -print -quit)" == "" ]]
    [[ "$(find backends/core/refloat-compatibility-android-jni -mindepth 1 -maxdepth 1 -type d \
      ! -name src -print -quit)" == "" ]]
    [[ "$(find backends/core/refloat-compatibility-apple-ffi -mindepth 1 -maxdepth 1 -type d \
      ! -name src ! -name tests -print -quit)" == "" ]]
    [[ "$(find backends/core/refloat-configuration-codec -mindepth 1 -maxdepth 1 -type d \
      ! -name src ! -name tests -print -quit)" == "" ]]
    [[ "$(find backends/core/refloat-configuration-service -mindepth 1 -maxdepth 1 -type d \
      ! -name src ! -name tests -print -quit)" == "" ]]
    [[ "$(find backends/core/controller-discovery -mindepth 1 -maxdepth 1 -type d \
      ! -name src ! -name tests -print -quit)" == "" ]]
    [[ "$(find backends/core/auxiliary-node-discovery -mindepth 1 -maxdepth 1 -type d \
      ! -name src ! -name tests -print -quit)" == "" ]]
    [[ "$(find backends/core/vesc-bms-telemetry-service -mindepth 1 -maxdepth 1 -type d \
      ! -name src ! -name tests -print -quit)" == "" ]]
    [[ "$(find backends/core/connection-application-coordinator -mindepth 1 -maxdepth 1 -type d \
      ! -name src ! -name tests -print -quit)" == "" ]]
    [[ "$(find backends/core/session-history-kernel -mindepth 1 -maxdepth 1 -type d \
      ! -name src ! -name tests -print -quit)" == "" ]]
    [[ "$(find backends/core/device-mutation-coordinator -mindepth 1 -maxdepth 1 -type d \
      ! -name src ! -name tests -print -quit)" == "" ]]
    [[ "$(find backends/core/refloat-configuration-codec-apple-ffi -mindepth 1 -maxdepth 1 -type d \
      ! -name src ! -name tests -print -quit)" == "" ]]
    [[ "$(find backends/core/refloat-configuration-codec-android-jni -mindepth 1 -maxdepth 1 -type d \
      ! -name src -print -quit)" == "" ]]
    [[ "$(find backends/core/refloat-control -mindepth 1 -maxdepth 1 -type d \
      ! -name src -print -quit)" == "" ]]
    [[ "$(find backends/core/refloat-control-android-jni -mindepth 1 -maxdepth 1 -type d \
      ! -name src -print -quit)" == "" ]]
    [[ "$(find backends/core/refloat-control-apple-ffi -mindepth 1 -maxdepth 1 -type d \
      ! -name src ! -name tests -print -quit)" == "" ]]
    [[ "$(find backends/core/refloat-dynamic-telemetry -mindepth 1 -maxdepth 1 -type d \
      ! -name src -print -quit)" == "" ]]
    [[ "$(find backends/core/refloat-dynamic-telemetry-android-jni -mindepth 1 -maxdepth 1 -type d \
      ! -name src -print -quit)" == "" ]]
    [[ "$(find backends/core/refloat-dynamic-telemetry-apple-ffi -mindepth 1 -maxdepth 1 -type d \
      ! -name src ! -name tests -print -quit)" == "" ]]
    [[ "$(find backends/core/refloat-legacy-telemetry -mindepth 1 -maxdepth 1 -type d \
      ! -name src -print -quit)" == "" ]]
    [[ "$(find backends/core/refloat-legacy-telemetry-apple-ffi -mindepth 1 -maxdepth 1 -type d \
      ! -name src ! -name tests -print -quit)" == "" ]]
    [[ "$(find backends/core/refloat-legacy-telemetry-android-jni -mindepth 1 -maxdepth 1 -type d \
      ! -name src -print -quit)" == "" ]]
    [[ "$(find backends/core/vesc-bms-values -mindepth 1 -maxdepth 1 -type d \
      ! -name src -print -quit)" == "" ]]
    [[ "$(find backends/core/vesc-bms-values-android-jni -mindepth 1 -maxdepth 1 -type d \
      ! -name src -print -quit)" == "" ]]
    [[ "$(find backends/core/vesc-bms-values-apple-ffi -mindepth 1 -maxdepth 1 -type d \
      ! -name src ! -name tests -print -quit)" == "" ]]
    [[ "$(find backends/core/vesc-controller-data -mindepth 1 -maxdepth 1 -type d \
      ! -name src -print -quit)" == "" ]]
    [[ "$(find backends/core/vesc-controller-data-android-jni -mindepth 1 -maxdepth 1 -type d \
      ! -name src -print -quit)" == "" ]]
    [[ "$(find backends/core/vesc-controller-data-apple-ffi -mindepth 1 -maxdepth 1 -type d \
      ! -name src ! -name tests -print -quit)" == "" ]]
    [[ "$(find backends/core/vesc-custom-configuration -mindepth 1 -maxdepth 1 -type d \
      ! -name src -print -quit)" == "" ]]
    [[ "$(find backends/core/vesc-custom-configuration-apple-ffi -mindepth 1 -maxdepth 1 -type d \
      ! -name src ! -name tests -print -quit)" == "" ]]
    [[ "$(find backends/core/vesc-firmware-identity -mindepth 1 -maxdepth 1 -type d \
      ! -name src -print -quit)" == "" ]]
    [[ "$(find backends/core/vesc-compatibility -mindepth 1 -maxdepth 1 -type d \
      ! -name src -print -quit)" == "" ]]
    [[ "$(find backends/core/vesc-frame -mindepth 1 -maxdepth 1 -type d \
      ! -name src -print -quit)" == "" ]]
    [[ "$(find backends/core/vesc-frame-apple-ffi -mindepth 1 -maxdepth 1 -type d \
      ! -name src ! -name tests -print -quit)" == "" ]]
    [[ "$(find backends/core/vesc-firmware-identity-apple-ffi -mindepth 1 -maxdepth 1 -type d \
      ! -name src ! -name tests -print -quit)" == "" ]]
    [[ "$(find backends/core/vesc-firmware-identity-android-jni -mindepth 1 -maxdepth 1 -type d \
      ! -name src -print -quit)" == "" ]]
    [[ "$(find backends/core/vesc-frame-android-jni -mindepth 1 -maxdepth 1 -type d \
      ! -name src -print -quit)" == "" ]]
    ! find backends/core -type f \
      \( -name 'build.rs' -o -name '*.h' -o -name '*.hpp' -o -name '*.modulemap' \) \
      | grep -q .
    ! rg -n 'extern[[:space:]]+"C"|no_mangle|cbindgen|bindgen|uniffi|jni|host_callback' \
      backends/core/apple-ffi-support
    ! rg -n 'unsafe\(no_mangle\)|extern[[:space:]]+"system"|Java_|cbindgen|bindgen|uniffi|host_callback' \
      backends/core/android-jni-support
    [[ "$(cd backends/core && cargo tree --locked --package float-view-apple-ffi-support \
      --edges normal --prefix none | wc -l | tr -d ' ')" == 1 ]]
    [[ "$(rg -c 'core/apple-ffi-support/Cargo.toml' \
      backends/apple/ios/float-view.xcodeproj/project.pbxproj)" == 10 ]]
    [[ "$(rg -c 'core/apple-ffi-support/src/lib.rs' \
      backends/apple/ios/float-view.xcodeproj/project.pbxproj)" == 10 ]]
    ! rg -n 'slice::from_raw_parts|ptr::copy_nonoverlapping|catch_unwind|AssertUnwindSafe|fn address_region|fn regions_overlap' \
      backends/core/*-apple-ffi/src/lib.rs
    rg -q 'slice::from_raw_parts' backends/core/apple-ffi-support/src/lib.rs
    rg -q 'ptr::copy_nonoverlapping' backends/core/apple-ffi-support/src/lib.rs
    rg -q 'catch_unwind' backends/core/apple-ffi-support/src/lib.rs
    rg -q '^#!\[forbid\(unsafe_code\)\]$' backends/core/refloat-compatibility/src/lib.rs
    ! rg -n 'extern[[:space:]]+"C"|extern[[:space:]]+"system"|no_mangle|Java_|cbindgen|bindgen|uniffi|jni|host_callback' \
      backends/core/refloat-compatibility
    ! rg -n 'refloat-compatibility-apple-ffi|CVescRefloatCompatibility|float_view_refloat_compatibility' \
      backends/android
    compatibility_source='backends/apple/Sources/FloatViewBackend/protocol/refloat/RefloatCompatibility.swift'
    [[ ! -f backends/apple/Sources/FloatViewBackend/protocol/refloat/RefloatInfo.swift ]]
    [[ "$(rg -c 'float_view_refloat_compatibility_info_request\(' "$compatibility_source")" == 1 ]]
    [[ "$(rg -c 'float_view_refloat_compatibility_decode_info\(' "$compatibility_source")" == 1 ]]
    [[ "$(rg -c 'float_view_refloat_compatibility_admit_layout\(' "$compatibility_source")" == 1 ]]
    [[ "$(rg -l '^import CVescRefloatCompatibility$' \
      backends/apple/Sources -g '*.swift')" == "$compatibility_source" ]]
    ! rg -n -e 'INFO_V2_(MAJOR|MINOR|PATCH)_OFFSET' -e 'requiredFields' \
      -e 'requiredRuntimeFields' -e 'supportsDynamicLayout' -e 'Set\(required' \
      backends/apple/Sources -g '*.swift'
    ! rg -n -e 'sourceVersion' -e 'static let entries' "$compatibility_source"
    [[ "$(rg -c 'core/refloat-compatibility/src/lib.rs' \
      backends/apple/ios/float-view.xcodeproj/project.pbxproj)" == 1 ]]
    rg -q '^#!\[forbid\(unsafe_code\)\]$' backends/core/refloat-control/src/lib.rs
    ! rg -n 'extern[[:space:]]+"C"|no_mangle|cbindgen|uniffi|jni|host_callback' \
      backends/core/refloat-control
    ! rg -n 'refloat-control-apple-ffi|CVescRefloatControl|float_view_refloat_control_(abi_version|encode|decode_lighting)' \
      backends/android
    ! rg -n 'refloat-control-android-jni|RustRefloatControlBridge|float_view_refloat_control_android_jni' \
      backends/apple
    rg -q '^#!\[forbid\(unsafe_code\)\]$' \
      backends/core/refloat-dynamic-telemetry/src/lib.rs
    ! rg -n 'extern[[:space:]]+"C"|extern[[:space:]]+"system"|no_mangle|Java_|cbindgen|bindgen|uniffi|jni|host_callback' \
      backends/core/refloat-dynamic-telemetry
    ! rg -n 'refloat-dynamic-telemetry-apple-ffi|CVescRefloatDynamicTelemetry|float_view_refloat_dynamic_telemetry_(abi_version|decode_layout|decode_realtime)' \
      backends/android
    ! rg -n 'RustRefloatDynamicTelemetryBridge|refloat-dynamic-telemetry-android-jni' \
      backends/apple
    ! rg -n 'fn (serialize_layout|serialize_sample)|struct (LayoutRecord|SampleRecord)\(' \
      backends/core/refloat-dynamic-telemetry-{apple-ffi,android-jni}/src/lib.rs
    [[ "$(rg -l 'LayoutRecordOutcome' \
      backends/core/refloat-dynamic-telemetry-{apple-ffi,android-jni}/src/lib.rs | wc -l | tr -d ' ')" == 2 ]]
    [[ "$(rg -l 'SampleRecordOutcome' \
      backends/core/refloat-dynamic-telemetry-{apple-ffi,android-jni}/src/lib.rs | wc -l | tr -d ' ')" == 2 ]]
    [[ "$(rg -c 'float_view_refloat_dynamic_telemetry_decode_layout\(' \
      backends/apple/Sources/FloatViewBackend/protocol/refloat/RefloatDynamic.swift)" == 1 ]]
    [[ "$(rg -c 'float_view_refloat_dynamic_telemetry_decode_realtime\(' \
      backends/apple/Sources/FloatViewBackend/protocol/refloat/RefloatDynamic.swift)" == 1 ]]
    [[ "$(rg -l '^import CVescRefloatDynamicTelemetry$' \
      backends/apple/Sources -g '*.swift')" == \
      'backends/apple/Sources/FloatViewBackend/protocol/refloat/RefloatDynamic.swift' ]]
    [[ ! -e backends/apple/Sources/FloatViewBackend/protocol/refloat/RefloatTelemetryMappings.swift ]]
    ! rg -n -e 'decodeFloat16' -e 'SUPPORTED_MASK' -e 'MASK_RUNTIME_VALUES' \
      -e 'MAX_AGGREGATE_FIELD_BYTES' -e 'RefloatTelemetryMappings' \
      backends/apple -g '*.swift'
    [[ "$(rg -c 'RefloatDynamicTelemetryDecoder\.decodeFields' \
      backends/apple/Sources/FloatViewBackend/protocol/vesc/VescProtocolService.swift)" == 1 ]]
    [[ "$(rg -c 'dynamicTelemetry\.decodeFields' \
      backends/android/app/src/main/java/fail/acheron/floatview/protocol/vesc/VescProtocolService.kt)" == 1 ]]
    android_dynamic='backends/android/app/src/main/java/fail/acheron/floatview/protocol/refloat/RefloatDynamic.kt'
    [[ "$(rg -c 'RustRefloatDynamicTelemetryBridge\.decodeLayout' "$android_dynamic")" == 1 ]]
    [[ "$(rg -c 'RustRefloatDynamicTelemetryBridge\.decodeRealtime' "$android_dynamic")" == 1 ]]
    [[ ! -e backends/android/app/src/main/java/fail/acheron/floatview/protocol/refloat/RefloatTelemetryMappings.kt ]]
    ! rg -n -e 'decodeFloat16' -e 'kotlin\.math\.pow' -e 'includedRuntimeFields' \
      -e 'expected = 13' -e 'payload\[index \+ 8\]' -e 'RefloatTelemetryMappings' \
      backends/android/app/src/main -g '*.kt'
    rg -q '^#!\[forbid\(unsafe_code\)\]$' \
      backends/core/refloat-legacy-telemetry/src/lib.rs
    ! rg -n 'extern[[:space:]]+"C"|no_mangle|cbindgen|uniffi|jni|host_callback' \
      backends/core/refloat-legacy-telemetry
    [[ "$(rg -c '^fn encode_record\(' \
      backends/core/refloat-legacy-telemetry/src/lib.rs)" == 1 ]]
    [[ "$(rg -c '^pub fn decode_record\(' \
      backends/core/refloat-legacy-telemetry/src/lib.rs)" == 1 ]]
    ! rg -n 'fn encode_record|struct LegacySample|pub const (MAX_PAYLOAD_BYTES|RECORD_HEADER_BYTES|SAMPLE_RECORD_BYTES|MAX_RECORD_BYTES):' \
      backends/core/refloat-legacy-telemetry-{apple-ffi,android-jni}/src/lib.rs
    [[ "$(rg -l 'LegacyTelemetryRecordOutcome' \
      backends/core/refloat-legacy-telemetry-{apple-ffi,android-jni}/src/lib.rs | wc -l | tr -d ' ')" == 2 ]]
    [[ "$(rg -l 'decode_record' \
      backends/core/refloat-legacy-telemetry-{apple-ffi,android-jni}/src/lib.rs | wc -l | tr -d ' ')" == 2 ]]
    ! rg -n 'refloat-legacy-telemetry-apple-ffi|CVescRefloatLegacyTelemetry|float_view_refloat_legacy_telemetry_(abi_version|decode)' \
      backends/android
    ! rg -n 'refloat-legacy-telemetry-android-jni|RustRefloatLegacyTelemetryBridge|float_view_refloat_legacy_telemetry_android_jni' \
      backends/apple
    [[ "$(rg -c 'RefloatLegacyTelemetryDecoder\.decode' \
      backends/apple/Sources/FloatViewBackend/protocol/vesc/VescProtocolService.swift)" == 1 ]]
    [[ "$(rg -c 'RefloatLegacyTelemetryDecoder\.decode' \
      backends/android/app/src/main/java/fail/acheron/floatview/protocol/vesc/VescProtocolService.kt)" == 1 ]]
    ! rg -n 'extern[[:space:]]+"C"|no_mangle|cbindgen|uniffi|jni|host_callback' \
      backends/core/vesc-frame
    rg -q '^#!\[forbid\(unsafe_code\)\]$' \
      backends/core/vesc-custom-configuration/src/lib.rs
    ! rg -n 'extern[[:space:]]+"C"|no_mangle|cbindgen|uniffi|jni|host_callback' \
      backends/core/vesc-custom-configuration
    rg -q '^#!\[forbid\(unsafe_code\)\]$' backends/core/vesc-bms-values/src/lib.rs
    ! rg -n 'extern[[:space:]]+"C"|no_mangle|cbindgen|uniffi|jni|host_callback' \
      backends/core/vesc-bms-values
    ! rg -n 'vesc-bms-values-apple-ffi|CVescBmsValues|float_view_vesc_bms_values_(abi_version|decode)' \
      backends/android
    ! rg -n 'vesc-bms-values-android-jni|RustVescBmsValuesBridge|float_view_vesc_bms_values_android_jni' \
      backends/apple
    rg -q '^#!\[forbid\(unsafe_code\)\]$' backends/core/vesc-controller-data/src/lib.rs
    ! rg -n 'extern[[:space:]]+"C"|no_mangle|cbindgen|uniffi|jni|host_callback' \
      backends/core/vesc-controller-data
    ! rg -n 'vesc-controller-data-apple-ffi|CVescControllerData|float_view_vesc_controller_data_(abi_version|decode)' \
      backends/android
    ! rg -n 'vesc-controller-data-android-jni|RustVescControllerDataBridge|float_view_vesc_controller_data_android_jni' \
      backends/apple
    rg -q 'public static func realtime\(' \
      backends/apple/Sources/FloatViewBackend/protocol/vesc/VescProtocolService.swift
    rg -q 'public static func currentLimits\(' \
      backends/apple/Sources/FloatViewBackend/protocol/vesc/VescProtocolService.swift
    rg -q 'fun realtime\(' \
      backends/android/app/src/main/java/fail/acheron/floatview/protocol/vesc/VescProtocolService.kt
    rg -q 'fun currentLimits\(' \
      backends/android/app/src/main/java/fail/acheron/floatview/protocol/vesc/VescProtocolService.kt
    rg -q '^#!\[forbid\(unsafe_code\)\]$' backends/core/vesc-firmware-identity/src/lib.rs
    ! rg -n 'extern[[:space:]]+"C"|no_mangle|cbindgen|uniffi|jni|host_callback' \
      backends/core/vesc-firmware-identity
    [[ "$(rg -c '^fn encode_record\(' \
      backends/core/vesc-firmware-identity/src/lib.rs)" == 1 ]]
    [[ "$(rg -c '^pub fn decode_record\(' \
      backends/core/vesc-firmware-identity/src/lib.rs)" == 1 ]]
    ! rg -n 'fn encode_record|struct DecodeResult|FLAG_FIRMWARE_NAME_PRESENT|FLAG_HARDWARE_CRC_PRESENT|pub const (MAX_PAYLOAD_BYTES|RECORD_HEADER_BYTES|MAX_ENCODED_NAME_BYTES|MAX_RECORD_BYTES):' \
      backends/core/vesc-firmware-identity-apple-ffi/src/lib.rs \
      backends/core/vesc-firmware-identity-android-jni/src/lib.rs
    [[ "$(rg -l 'FirmwareIdentityRecordOutcome' \
      backends/core/vesc-firmware-identity-{apple-ffi,android-jni}/src/lib.rs | wc -l | tr -d ' ')" == 2 ]]
    [[ "$(rg -l 'decode_record' \
      backends/core/vesc-firmware-identity-{apple-ffi,android-jni}/src/lib.rs | wc -l | tr -d ' ')" == 2 ]]
    apple_identity_lines="$(sed '/^#\[cfg(test)\]/,$d' \
      backends/core/vesc-firmware-identity-apple-ffi/src/lib.rs | wc -l | tr -d ' ')"
    android_identity_lines="$(sed '/^#\[cfg(test)\]/,$d' \
      backends/core/vesc-firmware-identity-android-jni/src/lib.rs | wc -l | tr -d ' ')"
    (( apple_identity_lines + android_identity_lines <= 300 ))
    [[ "$(rg -c 'core/vesc-firmware-identity/src/lib.rs' \
      backends/apple/ios/float-view.xcodeproj/project.pbxproj)" == 1 ]]
    rg -q 'fileTree\(rustCoreDirectory.resolve\("\$crateDirectory/src"\)\)' \
      backends/android/app/build.gradle.kts
    ! rg -n 'vesc-firmware-identity-apple-ffi|CVescFirmwareIdentity|float_view_vesc_firmware_identity_(abi_version|decode)' \
      backends/android
    ! rg -n 'vesc-custom-configuration-apple-ffi|CVescCustomConfiguration|float_view_vesc_custom_configuration' \
      backends/android
    ! rg -n 'cbindgen|uniffi|jni|host_callback' backends/core/vesc-frame-apple-ffi
    ! rg -n 'cbindgen|uniffi|jni|host_callback' \
      backends/core/vesc-firmware-identity-apple-ffi
    ! rg -n 'cbindgen|uniffi|jni|host_callback' \
      backends/core/vesc-custom-configuration-apple-ffi
    ! rg -n 'cbindgen|uniffi|jni|host_callback' \
      backends/core/vesc-bms-values-apple-ffi
    ! rg -n 'cbindgen|uniffi|jni|host_callback' \
      backends/core/vesc-controller-data-apple-ffi
    ! rg -n 'cbindgen|uniffi|jni|host_callback' \
      backends/core/refloat-control-apple-ffi
    ! rg -n 'cbindgen|uniffi|jni|host_callback' \
      backends/core/refloat-compatibility-apple-ffi
    ! rg -n 'cbindgen|uniffi|jni|host_callback' \
      backends/core/refloat-dynamic-telemetry-apple-ffi
    ! rg -n 'cbindgen|uniffi|jni|host_callback' \
      backends/core/refloat-legacy-telemetry-apple-ffi
    ! rg -n 'cbindgen|uniffi|jni|host_callback' \
      backends/core/refloat-configuration-codec-apple-ffi
    [[ "$(cd backends/core && cargo tree --locked --package float-view-refloat-control --edges normal --prefix none | wc -l | tr -d ' ')" == 1 ]]
    [[ "$(cd backends/core && cargo tree --locked --package float-view-refloat-compatibility --edges normal --prefix none | wc -l | tr -d ' ')" == 2 ]]
    [[ "$(cd backends/core && cargo tree --locked --package float-view-refloat-compatibility-apple-ffi --edges normal --depth 1 --prefix none | wc -l | tr -d ' ')" == 3 ]]
    [[ "$(cd backends/core && cargo tree --locked --package float-view-refloat-compatibility-android-jni --edges normal --depth 1 --prefix none | wc -l | tr -d ' ')" == 3 ]]
    [[ "$(cd backends/core && cargo tree --locked --package float-view-refloat-control-apple-ffi --edges normal --depth 1 --prefix none | wc -l | tr -d ' ')" == 3 ]]
    [[ "$(cd backends/core && cargo tree --locked --package float-view-refloat-control-android-jni --edges normal --depth 1 --prefix none | wc -l | tr -d ' ')" == 3 ]]
    [[ "$(cd backends/core && cargo tree --locked --package float-view-refloat-dynamic-telemetry --edges normal --prefix none | wc -l | tr -d ' ')" == 1 ]]
    [[ "$(cd backends/core && cargo tree --locked --package float-view-refloat-dynamic-telemetry-apple-ffi --edges normal --depth 1 --prefix none | wc -l | tr -d ' ')" == 3 ]]
    [[ "$(cd backends/core && cargo tree --locked --package float-view-refloat-dynamic-telemetry-android-jni --edges normal --depth 1 --prefix none | wc -l | tr -d ' ')" == 3 ]]
    [[ "$(cd backends/core && cargo tree --locked --package float-view-refloat-legacy-telemetry --edges normal --prefix none | wc -l | tr -d ' ')" == 1 ]]
    [[ "$(cd backends/core && cargo tree --locked --package float-view-refloat-legacy-telemetry-apple-ffi --edges normal --depth 1 --prefix none | wc -l | tr -d ' ')" == 3 ]]
    [[ "$(cd backends/core && cargo tree --locked --package float-view-refloat-legacy-telemetry-android-jni --edges normal --depth 1 --prefix none | wc -l | tr -d ' ')" == 3 ]]
    [[ "$(cd backends/core && cargo tree --locked --package float-view-refloat-configuration-codec-apple-ffi --edges normal --depth 1 --prefix none | wc -l | tr -d ' ')" == 3 ]]
    [[ "$(cd backends/core && cargo tree --locked --package float-view-refloat-configuration-codec-android-jni --edges normal --depth 1 --prefix none | wc -l | tr -d ' ')" == 3 ]]
    [[ "$(cd backends/core && cargo tree --locked --package float-view-vesc-firmware-identity --edges normal --prefix none | wc -l | tr -d ' ')" == 1 ]]
    [[ "$(cd backends/core && cargo tree --locked --package float-view-vesc-bms-values --edges normal --prefix none | wc -l | tr -d ' ')" == 1 ]]
    [[ "$(cd backends/core && cargo tree --locked --package float-view-vesc-bms-values-apple-ffi --edges normal --depth 1 --prefix none | wc -l | tr -d ' ')" == 3 ]]
    [[ "$(cd backends/core && cargo tree --locked --package float-view-vesc-bms-values-android-jni --edges normal --depth 1 --prefix none | wc -l | tr -d ' ')" == 3 ]]
    [[ "$(cd backends/core && cargo tree --locked --package float-view-vesc-controller-data --edges normal --prefix none | wc -l | tr -d ' ')" == 1 ]]
    [[ "$(cd backends/core && cargo tree --locked --package float-view-vesc-controller-data-apple-ffi --edges normal --depth 1 --prefix none | wc -l | tr -d ' ')" == 3 ]]
    [[ "$(cd backends/core && cargo tree --locked --package float-view-vesc-controller-data-android-jni --edges normal --depth 1 --prefix none | wc -l | tr -d ' ')" == 3 ]]
    [[ "$(cd backends/core && cargo tree --locked --package float-view-vesc-custom-configuration --edges normal --prefix none | wc -l | tr -d ' ')" == 1 ]]
    [[ "$(cd backends/core && cargo tree --locked --package float-view-vesc-custom-configuration-apple-ffi --edges normal --depth 1 --prefix none | wc -l | tr -d ' ')" == 3 ]]
    [[ "$(cd backends/core && cargo tree --locked --package float-view-vesc-frame --edges normal --prefix none | wc -l | tr -d ' ')" == 1 ]]
    [[ "$(cd backends/core && cargo tree --locked --package float-view-vesc-frame-apple-ffi --edges normal --depth 1 --prefix none | wc -l | tr -d ' ')" == 3 ]]
    [[ "$(cd backends/core && cargo tree --locked --package float-view-vesc-firmware-identity-apple-ffi --edges normal --depth 1 --prefix none | wc -l | tr -d ' ')" == 3 ]]
    [[ "$(cd backends/core && cargo tree --locked --package float-view-vesc-firmware-identity-android-jni --edges normal --depth 1 --prefix none | wc -l | tr -d ' ')" == 3 ]]
    ! git ls-files | rg '\.(a|so|xcframework|dylib)$'
    [[ "$(find backends/apple/Sources -type f \( -name '*.h' -o -name '*.modulemap' \) | sort)" == $'backends/apple/Sources/CSQLite/module.modulemap\nbackends/apple/Sources/CSQLite/shim.h\nbackends/apple/Sources/CVescBmsValues/include/CVescBmsValues.h\nbackends/apple/Sources/CVescBmsValues/module.modulemap\nbackends/apple/Sources/CVescControllerData/include/CVescControllerData.h\nbackends/apple/Sources/CVescControllerData/module.modulemap\nbackends/apple/Sources/CVescCustomConfiguration/include/CVescCustomConfiguration.h\nbackends/apple/Sources/CVescCustomConfiguration/module.modulemap\nbackends/apple/Sources/CVescFirmwareIdentity/include/CVescFirmwareIdentity.h\nbackends/apple/Sources/CVescFirmwareIdentity/module.modulemap\nbackends/apple/Sources/CVescFrame/include/CVescFrame.h\nbackends/apple/Sources/CVescFrame/module.modulemap\nbackends/apple/Sources/CVescRefloatCompatibility/include/CVescRefloatCompatibility.h\nbackends/apple/Sources/CVescRefloatCompatibility/module.modulemap\nbackends/apple/Sources/CVescRefloatConfigurationCodec/include/CVescRefloatConfigurationCodec.h\nbackends/apple/Sources/CVescRefloatConfigurationCodec/module.modulemap\nbackends/apple/Sources/CVescRefloatControl/include/CVescRefloatControl.h\nbackends/apple/Sources/CVescRefloatControl/module.modulemap\nbackends/apple/Sources/CVescRefloatDynamicTelemetry/include/CVescRefloatDynamicTelemetry.h\nbackends/apple/Sources/CVescRefloatDynamicTelemetry/module.modulemap\nbackends/apple/Sources/CVescRefloatLegacyTelemetry/include/CVescRefloatLegacyTelemetry.h\nbackends/apple/Sources/CVescRefloatLegacyTelemetry/module.modulemap' ]]
    just rust-bridge-build-tooling-audit
    just core-abi-audit
    just core-identity-abi-audit
    just core-custom-configuration-abi-audit
    just core-bms-values-abi-audit
    just core-controller-data-abi-audit
    just core-refloat-control-abi-audit
    just core-refloat-compatibility-abi-audit
    just core-refloat-configuration-codec-audit
    just core-refloat-configuration-service-audit
    just core-vesc-request-scheduler-audit
    just core-vesc-compatibility-audit
    just core-controller-discovery-audit
    just core-auxiliary-node-discovery-audit
    just core-vesc-bms-telemetry-service-audit
    just core-refloat-telemetry-service-audit
    just core-connection-application-coordinator-audit
    just core-session-history-kernel-audit
    just core-device-mutation-coordinator-audit
    just core-refloat-configuration-codec-abi-audit
    just core-refloat-dynamic-telemetry-abi-audit
    just core-refloat-legacy-telemetry-abi-audit
    just android-rust-audit
    ! rg -n -e 'CheckpointSwiftVescFrameReference' -e 'CheckpointSwiftVescStreamReference' \
      -e 'VescCRC' -e '0x1021' -e 'first != 2, first != 3' \
      -e 'header(Bytes|Count) = first == 2' -e 'payloadBytes = first == 2' \
      -e 'frame\.append' -e 'checksum\(' -e 'crc.*UInt16' \
      backends/apple -g '*.swift'
    [[ "$(rg -l 'float_view_vesc_frame_(encode|decode_exact|stream_scan_next)' \
      backends/apple -g '*.swift')" == \
      'backends/apple/Sources/FloatViewBackend/protocol/vesc/VescFrameCodec.swift' ]]
    identity_source='backends/apple/Sources/FloatViewBackend/protocol/vesc/VescFirmwareIdentityDecoder.swift'
    ! rg -n -e 'terminatedBytes' -e 'extendedFirmwareName' -e 'uuidStart' \
      -e 'commonEnd' -e 'firstIndex\(of: 0\)' -e 'tailLength' \
      "$identity_source"
    [[ "$(rg -c 'float_view_vesc_firmware_identity_decode\(' "$identity_source")" == 1 ]]
    [[ "$(rg -l 'float_view_vesc_firmware_identity_decode\(' \
      backends/apple/Sources -g '*.swift')" == "$identity_source" ]]
    [[ "$(rg -l '^import CVescFirmwareIdentity$' \
      backends/apple/Sources -g '*.swift')" == "$identity_source" ]]
    ! rg -n -e 'SwiftFirmwareIdentityReference' -e 'CheckpointSwiftFirmwareIdentity' \
      -e 'FLOAT_VIEW_SWIFT_FIRMWARE_IDENTITY' backends/apple -g '*.swift'
    custom_configuration_source='backends/apple/Sources/FloatViewBackend/protocol/vesc/VescCustomConfigurationCodec.swift'
    ! rg -n -e 'positiveI32' -e 'nonnegativeI32' -e 'private static func i32' \
      -e 'dropFirst\(10\)' -e 'dropFirst\(2\)' \
      "$custom_configuration_source"
    [[ "$(rg -c 'float_view_vesc_custom_configuration_encode\(' "$custom_configuration_source")" == 1 ]]
    [[ "$(rg -c 'float_view_vesc_custom_configuration_decode\(' "$custom_configuration_source")" == 1 ]]
    [[ "$(rg -l 'float_view_vesc_custom_configuration_(encode|decode)\(' \
      backends/apple/Sources -g '*.swift')" == "$custom_configuration_source" ]]
    [[ "$(rg -l '^import CVescCustomConfiguration$' \
      backends/apple/Sources -g '*.swift')" == "$custom_configuration_source" ]]
    bms_source='backends/apple/Sources/FloatViewBackend/protocol/vesc/VescBmsValuesDecoder.swift'
    bms_protocol_source='backends/apple/Sources/FloatViewBackend/protocol/vesc/VescProtocolService.swift'
    ! rg -n -e 'cellCount' -e 'temperatureCount' -e 'terminatedUTF8' \
      -e 'maxContentBytes' -e 'lifetimeChargeAmpHours' -e 'reportedCanID' \
      "$bms_protocol_source"
    [[ "$(rg -c 'VescBmsValuesDecoder\.decode\(' "$bms_protocol_source")" == 1 ]]
    [[ "$(rg -c 'float_view_vesc_bms_values_decode\(' "$bms_source")" == 1 ]]
    [[ "$(rg -l 'float_view_vesc_bms_values_decode\(' \
      backends/apple/Sources -g '*.swift')" == "$bms_source" ]]
    [[ "$(rg -l '^import CVescBmsValues$' \
      backends/apple/Sources -g '*.swift')" == "$bms_source" ]]
    ! rg -n -e 'SwiftBmsValuesReference' -e 'CheckpointSwiftBmsValues' \
      -e 'FLOAT_VIEW_SWIFT_BMS_VALUES' backends/apple -g '*.swift'
    controller_data_source='backends/apple/Sources/FloatViewBackend/protocol/vesc/VescControllerDataDecoder.swift'
    controller_data_protocol_source='backends/apple/Sources/FloatViewBackend/protocol/vesc/VescProtocolService.swift'
    ! rg -n -e 'ByteReader' -e 'float32Auto' -e 'batterySeriesCellOffsets' \
      -e 'i16\(scale:' -e 'i32\(scale:' \
      "$controller_data_protocol_source"
    [[ "$(rg -c 'VescControllerDataDecoder\.realtime\(' "$controller_data_protocol_source")" == 1 ]]
    [[ "$(rg -c 'VescControllerDataDecoder\.currentLimits\(' "$controller_data_protocol_source")" == 1 ]]
    [[ "$(rg -c 'float_view_vesc_controller_data_decode\(' "$controller_data_source")" == 1 ]]
    [[ "$(rg -l 'float_view_vesc_controller_data_decode\(' \
      backends/apple/Sources -g '*.swift')" == "$controller_data_source" ]]
    [[ "$(rg -l '^import CVescControllerData$' \
      backends/apple/Sources -g '*.swift')" == "$controller_data_source" ]]
    ! rg -n -e 'SwiftControllerDataReference' -e 'CheckpointSwiftControllerData' \
      -e 'FLOAT_VIEW_SWIFT_CONTROLLER_DATA' backends/apple -g '*.swift'
    android_controller_data_source='backends/android/app/src/main/java/fail/acheron/floatview/protocol/vesc/VescControllerDataDecoder.kt'
    android_controller_data_protocol_source='backends/android/app/src/main/java/fail/acheron/floatview/protocol/vesc/VescProtocolService.kt'
    ! rg -n -e 'private class ByteReader' -e 'float32Auto' -e 'batterySeriesCellOffsets' \
      -e 'Math\.scalb' -e 'VescFault\.fromCode' \
      "$android_controller_data_protocol_source"
    [[ "$(rg -c 'VescControllerDataDecoder\.realtime\(' "$android_controller_data_protocol_source")" == 1 ]]
    [[ "$(rg -c 'VescControllerDataDecoder\.currentLimits\(' "$android_controller_data_protocol_source")" == 1 ]]
    [[ "$(rg -c 'RustVescControllerDataBridge\.decode\(' "$android_controller_data_source")" == 1 ]]
    control_source='backends/apple/Sources/FloatViewBackend/protocol/refloat/RefloatControlCodec.swift'
    ! rg -n -e 'RefloatWire\.request\(' -e 'mask: UInt8' -e 'flags = enabled \?' \
      -e 'payload\[0\].*customAppData' -e 'payload\[1\].*packageID' \
      -e 'payload\[2\].*command' \
      "$control_source"
    [[ "$(rg -c 'float_view_refloat_control_encode\(' "$control_source")" == 1 ]]
    [[ "$(rg -c 'float_view_refloat_control_decode_lighting\(' "$control_source")" == 1 ]]
    [[ "$(rg -l 'float_view_refloat_control_(encode|decode_lighting)\(' \
      backends/apple/Sources -g '*.swift')" == "$control_source" ]]
    [[ "$(rg -l '^import CVescRefloatControl$' \
      backends/apple/Sources -g '*.swift')" == "$control_source" ]]
    telemetry_source='backends/apple/Sources/FloatViewBackend/protocol/refloat/RefloatLegacy.swift'
    [[ "$(rg -c 'float_view_refloat_legacy_telemetry_decode\(' "$telemetry_source")" == 1 ]]
    [[ "$(rg -l '^import CVescRefloatLegacyTelemetry$' \
      backends/apple/Sources -g '*.swift')" == "$telemetry_source" ]]
    ! rg -n -e 'signed16' -e 'unsigned32' -e 'legacySemanticState' \
      -e 'legacySetpointAdjustment' -e 'decodeFloat32' "$telemetry_source"
    [[ -d backends/apple/ios ]]
    [[ -d backends/apple/macos ]]
    ! find backends/android/app/src/main/java/fail/acheron/floatview \
      -maxdepth 1 -type f -name '*.kt' | grep -q .
    ! find backends/apple/ios/float-view -maxdepth 1 -type f -name '*.swift' | grep -q .
    ! find backends/apple/macos -maxdepth 1 -type f -name '*.swift' | grep -q .
    unexpected_android="$(find backends/android/app/src/main/java/fail/acheron/floatview \
      -mindepth 1 -maxdepth 1 -type d \
      ! -name application ! -name bluetooth ! -name files ! -name lifecycle \
      ! -name location ! -name protocol ! -name storage ! -name theme \
      ! -name transport ! -name ui -print -quit)"
    [[ -z "$unexpected_android" ]] || {
      echo "Unexpected Android capability path: $unexpected_android" >&2
      exit 1
    }
    unexpected_apple="$(find backends/apple/Sources/FloatViewBackend \
      -mindepth 1 -maxdepth 1 -type d \
      ! -name application ! -name bluetooth ! -name location ! -name protocol \
      ! -name storage ! -name transport -print -quit)"
    [[ -z "$unexpected_apple" ]] || {
      echo "Unexpected shared Apple capability path: $unexpected_apple" >&2
      exit 1
    }
    unexpected_macos="$(find backends/apple/macos \
      -mindepth 1 -maxdepth 1 -type d \
      ! -name application ! -name files ! -name lifecycle ! -name Resources \
      ! -name transport ! -name ui -print -quit)"
    [[ -z "$unexpected_macos" ]] || {
      echo "Unexpected macOS capability path: $unexpected_macos" >&2
      exit 1
    }
    unexpected_ios="$(find backends/apple/ios/float-view \
      -mindepth 1 -maxdepth 1 -type d \
      ! -name application ! -name files ! -name lifecycle ! -name theme \
      ! -name ui ! -name Assets.xcassets -print -quit)"
    [[ -z "$unexpected_ios" ]] || {
      echo "Unexpected iOS capability path: $unexpected_ios" >&2
      exit 1
    }
    [[ ! -d tools/vesc-test-device/host ]]
    ! find tools/vesc-test-device -type f \( -name '*.ts' -o -name '*.js' \) | grep -q .
    ! rg -n "from ['\"]svelte" conformance/fixture
    [[ ! -e conformance/fixture-dev/package.json ]]
    [[ ! -e conformance/fixture-dev/package-lock.json ]]
    [[ ! -d conformance/fixture-dev/build ]]
    ! find conformance/fixture-dev/src/lib -type d \
      \( -name components -o -name services -o -name types -o -name utils \) | grep -q .
    ! rg -n '@sveltejs/adapter|svelte-kit build|vite build|vite preview' \
      conformance/fixture-dev conformance/package.json
    [[ "$(find frontend/build -type f | wc -l | tr -d ' ')" == 1 ]]
    [[ -f frontend/build/index.html ]]
    cmp -s frontend/build/index.html backends/apple/macos/Resources/index.html
    cmp -s backends/sql/catalog.sql \
      backends/apple/macos/Resources/sql/catalog.sql
    cmp -s backends/sql/session.sql \
      backends/apple/macos/Resources/sql/session.sql
    ! rg -n 'system\.hello|system\.getSnapshot|telemetrySubscriptions|loggingJobs' \
      frontend backends conformance
    ! rg -n '\b([3]001|[5]173|[8]765|[8]766)\b' \
      frontend backends conformance justfile \
      -g '!frontend/build/**' -g '!**/Resources/index.html'

verify: build
    just core-lint
    just apple-packaging
    just test all
    just android-rust-packaging
    just apple-rust-bridges
    cd backends/apple && swift build --scratch-path /tmp/float-view-swift-build --product FloatViewMac
    xcodebuild -project backends/apple/ios/float-view.xcodeproj -scheme float-view \
      -sdk iphonesimulator -destination 'generic/platform=iOS Simulator' \
      -derivedDataPath /tmp/float-view-ios CODE_SIGNING_ALLOWED=NO build
    cd conformance && npm run macos && npm run ios && npm run android
    just audit
