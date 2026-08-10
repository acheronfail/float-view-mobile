_default:
    @just --list

core-lint:
    cd backends/core && cargo fmt --check
    cd backends/core && cargo clippy --locked --offline --workspace --all-targets -- -D warnings

apple-rust-codec configuration="Debug":
    #!/usr/bin/env bash
    set -euo pipefail
    backends/core/vesc-frame-apple-ffi/build-apple.sh \
      macosx "$(uname -m)" "{{ configuration }}" \
      "$PWD/backends/core/target/apple/macos"

apple-rust-firmware-identity configuration="Debug":
    #!/usr/bin/env bash
    set -euo pipefail
    backends/core/vesc-firmware-identity-apple-ffi/build-apple.sh \
      macosx "$(uname -m)" "{{ configuration }}" \
      "$PWD/backends/core/target/apple/macos"

apple-rust-bridges configuration="Debug":
    just apple-rust-codec "{{ configuration }}"
    just apple-rust-firmware-identity "{{ configuration }}"

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
    rust_abi="$(sed -n 's/^pub const ABI_VERSION: u32 = \([0-9][0-9]*\);$/\1/p' \
      backends/core/vesc-firmware-identity-apple-ffi/src/lib.rs)"
    header_abi="$(sed -n 's/^#define FLOAT_VIEW_VESC_FIRMWARE_IDENTITY_ABI_VERSION UINT32_C(\([0-9][0-9]*\))$/\1/p' \
      backends/apple/Sources/CVescFirmwareIdentity/include/CVescFirmwareIdentity.h)"
    swift_abi="$(sed -n 's/^[[:space:]]*private static let expectedABI: UInt32 = \([0-9][0-9]*\)$/\1/p' \
      backends/apple/Sources/FloatViewBackend/protocol/vesc/VescFirmwareIdentityDecoder.swift)"
    [[ -n "$rust_abi" && "$rust_abi" == "$header_abi" && "$rust_abi" == "$swift_abi" ]] || {
      echo "VESC firmware-identity ABI version mismatch: Rust=$rust_abi header=$header_abi Swift=$swift_abi" >&2
      exit 1
    }

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

core-test: core-abi-test core-identity-abi-test
    cd backends/core && cargo test --locked --offline

android-rust-host:
    backends/core/vesc-frame-android-jni/build-android.sh host debug \
      "$PWD/backends/core/target/android/host/debug"
    backends/core/vesc-firmware-identity-android-jni/build-android.sh host debug \
      "$PWD/backends/core/target/android/identity-host/debug"

android-rust-codec-test:
    cd backends/android && ./gradlew testDebugUnitTest \
      --tests fail.acheron.floatview.protocol.RustVescFrameBridgeTest \
      --tests fail.acheron.floatview.protocol.RustVescFirmwareIdentityBridgeTest \
      --tests fail.acheron.floatview.protocol.VescProtocolTest

android-rust-packaging:
    #!/usr/bin/env bash
    set -euo pipefail
    (cd backends/android && ./gradlew clean assembleDebug assembleRelease)
    expected="$(for library in \
      libfloat_view_vesc_frame_android_jni.so \
      libfloat_view_vesc_firmware_identity_android_jni.so; do
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
        | rg '(^|/)libfloat_view_vesc_(frame|firmware_identity)_android_jni\.so$' \
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
      --package float-view-vesc-frame-apple-ffi --edges normal --depth 1 --prefix none | wc -l | tr -d ' ')" == 2 ]]
    [[ "$(cd backends/core && cargo tree --locked --offline \
      --package float-view-vesc-firmware-identity-android-jni --edges normal --depth 1 --prefix none | wc -l | tr -d ' ')" == 3 ]]
    for manifest in \
      backends/core/vesc-frame-android-jni/Cargo.toml \
      backends/core/vesc-firmware-identity-android-jni/Cargo.toml; do
      rg -q '^jni-sys = "=0\.4\.1"$' "$manifest"
    done
    ! rg -n '^jni[[:space:]]*=' backends/core -g Cargo.toml
    rust_abi="$(sed -n 's/^pub const ABI_VERSION: jint = \([0-9][0-9]*\);$/\1/p' \
      backends/core/vesc-frame-android-jni/src/lib.rs)"
    kotlin_abi="$(sed -n 's/^[[:space:]]*private const val ABI_VERSION = \([0-9][0-9]*\)$/\1/p' \
      backends/android/app/src/main/java/fail/acheron/floatview/protocol/vesc/VescFrame.kt)"
    [[ -n "$rust_abi" && "$rust_abi" == "$kotlin_abi" ]] || {
      echo "Android VESC JNI ABI mismatch: Rust=$rust_abi Kotlin=$kotlin_abi" >&2
      exit 1
    }
    identity_rust_abi="$(sed -n 's/^pub const ABI_VERSION: jint = \([0-9][0-9]*\);$/\1/p' \
      backends/core/vesc-firmware-identity-android-jni/src/lib.rs)"
    identity_kotlin_abi="$(sed -n 's/^internal const val VESC_FIRMWARE_IDENTITY_ABI_VERSION = \([0-9][0-9]*\)$/\1/p' \
      backends/android/app/src/main/java/fail/acheron/floatview/protocol/vesc/RustVescFirmwareIdentityBridge.kt)"
    [[ -n "$identity_rust_abi" && "$identity_rust_abi" == "$identity_kotlin_abi" ]] || {
      echo "Android firmware-identity JNI ABI mismatch: Rust=$identity_rust_abi Kotlin=$identity_kotlin_abi" >&2
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
      backends/android/app/src/main -g '*.kt')" == \
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
    [[ "$(rg -c 'functions\.ExceptionCheck' \
      backends/core/vesc-firmware-identity-android-jni/src/lib.rs)" -ge 8 ]]
    if ANDROID_NDK_HOME=/private/tmp/float-view-missing-ndk \
      backends/core/vesc-frame-android-jni/build-android.sh \
      android arm64-v8a debug /private/tmp/float-view-unreachable-output >/dev/null 2>&1; then
      echo 'Android Rust helper accepted a missing pinned NDK.' >&2
      exit 1
    fi
    if ANDROID_NDK_HOME=/private/tmp/float-view-missing-ndk \
      backends/core/vesc-firmware-identity-android-jni/build-android.sh \
      android arm64-v8a debug /private/tmp/float-view-unreachable-identity-output >/dev/null 2>&1; then
      echo 'Android Rust identity helper accepted a missing pinned NDK.' >&2
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
    [[ -f backends/core/vesc-custom-configuration/Cargo.toml ]]
    [[ -f backends/core/vesc-firmware-identity/Cargo.toml ]]
    [[ -f backends/core/vesc-firmware-identity-android-jni/Cargo.toml ]]
    [[ -f backends/core/vesc-firmware-identity-apple-ffi/Cargo.toml ]]
    [[ -f backends/core/vesc-frame/Cargo.toml ]]
    [[ -f backends/core/vesc-frame-apple-ffi/Cargo.toml ]]
    [[ -f backends/core/vesc-frame-android-jni/Cargo.toml ]]
    [[ -f backends/apple/Sources/CVescFrame/include/CVescFrame.h ]]
    [[ -f backends/apple/Sources/CVescFrame/module.modulemap ]]
    [[ -f backends/apple/Sources/CVescFirmwareIdentity/include/CVescFirmwareIdentity.h ]]
    [[ -f backends/apple/Sources/CVescFirmwareIdentity/module.modulemap ]]
    unexpected_core="$(find backends/core -mindepth 1 -maxdepth 1 -type d \
      ! -name target ! -name vesc-custom-configuration ! -name vesc-firmware-identity \
      ! -name vesc-firmware-identity-android-jni ! -name vesc-firmware-identity-apple-ffi \
      ! -name vesc-frame ! -name vesc-frame-apple-ffi \
      ! -name vesc-frame-android-jni -print -quit)"
    [[ -z "$unexpected_core" ]] || {
      echo "Unexpected Rust core workspace member: $unexpected_core" >&2
      exit 1
    }
    [[ "$(find backends/core/vesc-custom-configuration -mindepth 1 -maxdepth 1 -type d \
      ! -name src -print -quit)" == "" ]]
    [[ "$(find backends/core/vesc-firmware-identity -mindepth 1 -maxdepth 1 -type d \
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
    ! rg -n 'extern[[:space:]]+"C"|no_mangle|cbindgen|uniffi|jni|host_callback' \
      backends/core/vesc-frame
    rg -q '^#!\[forbid\(unsafe_code\)\]$' \
      backends/core/vesc-custom-configuration/src/lib.rs
    ! rg -n 'extern[[:space:]]+"C"|no_mangle|cbindgen|uniffi|jni|host_callback' \
      backends/core/vesc-custom-configuration
    rg -q '^#!\[forbid\(unsafe_code\)\]$' backends/core/vesc-firmware-identity/src/lib.rs
    ! rg -n 'extern[[:space:]]+"C"|no_mangle|cbindgen|uniffi|jni|host_callback' \
      backends/core/vesc-firmware-identity
    ! rg -n 'vesc-firmware-identity-apple-ffi|CVescFirmwareIdentity|float_view_vesc_firmware_identity_(abi_version|decode)' \
      backends/android
    ! rg -n 'vesc-custom-configuration|float_view_vesc_custom_configuration' \
      backends/apple backends/android
    ! rg -n 'cbindgen|uniffi|jni|host_callback' backends/core/vesc-frame-apple-ffi
    ! rg -n 'cbindgen|uniffi|jni|host_callback' \
      backends/core/vesc-firmware-identity-apple-ffi
    [[ "$(cd backends/core && cargo tree --locked --package float-view-vesc-firmware-identity --edges normal --prefix none | wc -l | tr -d ' ')" == 1 ]]
    [[ "$(cd backends/core && cargo tree --locked --package float-view-vesc-custom-configuration --edges normal --prefix none | wc -l | tr -d ' ')" == 1 ]]
    [[ "$(cd backends/core && cargo tree --locked --package float-view-vesc-frame --edges normal --prefix none | wc -l | tr -d ' ')" == 1 ]]
    [[ "$(cd backends/core && cargo tree --locked --package float-view-vesc-frame-apple-ffi --edges normal --depth 1 --prefix none | wc -l | tr -d ' ')" == 2 ]]
    [[ "$(cd backends/core && cargo tree --locked --package float-view-vesc-firmware-identity-apple-ffi --edges normal --depth 1 --prefix none | wc -l | tr -d ' ')" == 2 ]]
    [[ "$(cd backends/core && cargo tree --locked --package float-view-vesc-firmware-identity-android-jni --edges normal --depth 1 --prefix none | wc -l | tr -d ' ')" == 3 ]]
    ! git ls-files | rg '\.(a|so|xcframework|dylib)$'
    [[ "$(find backends/apple/Sources -type f \( -name '*.h' -o -name '*.modulemap' \) | sort)" == $'backends/apple/Sources/CSQLite/module.modulemap\nbackends/apple/Sources/CSQLite/shim.h\nbackends/apple/Sources/CVescFirmwareIdentity/include/CVescFirmwareIdentity.h\nbackends/apple/Sources/CVescFirmwareIdentity/module.modulemap\nbackends/apple/Sources/CVescFrame/include/CVescFrame.h\nbackends/apple/Sources/CVescFrame/module.modulemap' ]]
    just core-abi-audit
    just core-identity-abi-audit
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
