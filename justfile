_default:
    @just --list

# Install JavaScript dependencies and regenerate committed API models.
setup:
    cd web && npm ci
    cd tools/vesc-test-device/host && npm ci
    cd web && npm run generate

# Run the browser UI with the deterministic mock backend.
dev:
    cd web && npm run dev

# Build and run one platform: android [serial], ios [device-id], or macos.
run platform target="":
    #!/usr/bin/env bash
    set -euo pipefail
    platform="{{ platform }}"
    target="{{ target }}"
    (cd web && npm run build)
    case "$platform" in
      android)
        adb_args=()
        if [[ -n "$target" ]]; then adb_args=(-s "$target"); fi
        (cd android && ./gradlew :app:assembleDebug)
        adb "${adb_args[@]}" install -r \
          android/app/build/outputs/apk/debug/app-debug.apk
        adb "${adb_args[@]}" shell am force-stop fail.acheron.floatview
        adb "${adb_args[@]}" shell am start -W \
          -n fail.acheron.floatview/fail.acheron.floatview.MainActivity
        ;;
      ios)
        if [[ -n "$target" ]]; then
          derived="/tmp/float-view-ios-device"
          xcodebuild -project ios/float-view.xcodeproj -scheme float-view \
            -configuration Debug -destination "id=$target" \
            -derivedDataPath "$derived" -allowProvisioningUpdates build
          app="$derived/Build/Products/Debug-iphoneos/float-view.app"
          xcrun devicectl device install app --device "$target" "$app"
          xcrun devicectl device process launch --device "$target" \
            --terminate-existing fail.acheron.float-view
        else
          derived="/tmp/float-view-ios-derived"
          xcodebuild -project ios/float-view.xcodeproj -scheme float-view \
            -configuration Debug -sdk iphonesimulator \
            -derivedDataPath "$derived" CODE_SIGNING_ALLOWED=NO build
          xcrun simctl install booted \
            "$derived/Build/Products/Debug-iphonesimulator/float-view.app"
          xcrun simctl launch --terminate-running-process \
            booted fail.acheron.float-view
        fi
        ;;
      macos)
        (cd apple && CLANG_MODULE_CACHE_PATH=/tmp/float-view-clang \
          FLOAT_VIEW_WEB_ROOT="$PWD/../web/build" \
          swift run --disable-sandbox --scratch-path /tmp/float-view-swift-build \
          FloatViewMac)
        ;;
      *)
        echo "Usage: just run android [serial] | ios [device-id] | macos" >&2
        exit 2
        ;;
    esac

# Run a focused suite: web, core, protocol, storage, or all.
test suite="all":
    #!/usr/bin/env bash
    set -euo pipefail
    suite="{{ suite }}"
    swift_test() {
      (cd apple && CLANG_MODULE_CACHE_PATH=/tmp/float-view-clang \
        swift test --disable-sandbox --scratch-path /tmp/float-view-swift-build "$@")
    }
    case "$suite" in
      web)
        (cd web && npm run check && npm run lint && npm test)
        ;;
      core)
        swift_test
        (cd android && ./gradlew :core:test)
        ;;
      protocol)
        swift_test --filter ProtocolFixtureTests
        (cd android && ./gradlew :protocol:test)
        ;;
      storage)
        swift_test --filter SQLiteLogStoreTests
        (cd android && ./gradlew :logging:test :logging:compileDebugAndroidTestKotlin)
        ;;
      all)
        (cd web && npm run check && npm run lint && npm test)
        swift_test
        (cd android && ./gradlew test :logging:compileDebugAndroidTestKotlin)
        ;;
      *)
        echo "Usage: just test [web|core|protocol|storage|all]" >&2
        exit 2
        ;;
    esac

# Work with the programmable VESC fixture.
# Actions: check, doctor, flash [port], android [scenario], ios <device>, macos [scenario].
vesc action target="" scenario="ride":
    #!/usr/bin/env bash
    set -euo pipefail
    action="{{ action }}"
    target="{{ target }}"
    scenario="{{ scenario }}"
    host="tools/vesc-test-device/host"
    case "$action" in
      check)
        (cd "$host" && npm run check && npm test)
        ;;
      doctor)
        (cd "$host" && npm run doctor)
        ;;
      flash)
        port="${target:-${FLOAT_VIEW_ESP_PORT:-}}"
        [[ -n "$port" ]] || { echo "Pass a port or set FLOAT_VIEW_ESP_PORT." >&2; exit 2; }
        command -v idf.py >/dev/null || {
          echo "ESP-IDF is unavailable; source its export.sh first." >&2
          exit 2
        }
        idf.py -C tools/vesc-test-device/firmware build flash -p "$port"
        ;;
      android)
        selected="${target:-ride}"
        (cd "$host" && npm run "verify:android:$selected")
        ;;
      ios)
        [[ -n "$target" ]] || { echo "Pass the iOS device ID." >&2; exit 2; }
        (cd "$host" && npm run "verify:ios:$scenario" -- --device "$target")
        ;;
      macos)
        selected="${target:-ride}"
        (cd "$host" && npm run "verify:macos:$selected")
        ;;
      *)
        echo "Usage: just vesc check|doctor|flash|android|ios|macos" >&2
        exit 2
        ;;
    esac

# Regenerate and validate contracts, then run all tests and platform builds.
verify:
    cd web && npm run api:check
    cd web && npm run build
    swiftc api/generated/swift/FloatViewAPI.swift api/tests/swift/ContractSmoke.swift \
      -o /tmp/float-view-api-contract-smoke
    /tmp/float-view-api-contract-smoke api/examples/system.hello.response.json
    just test all
    just vesc check
    cd web && npm run api:conformance:mock
    cd android && ./gradlew :app:assembleDebug
    cd apple && CLANG_MODULE_CACHE_PATH=/tmp/float-view-clang \
      swift build --disable-sandbox --scratch-path /tmp/float-view-swift-build \
      --product FloatViewMac
    cd web && FLOAT_VIEW_APPLE_HOST=/tmp/float-view-swift-build/arm64-apple-macosx/debug/FloatViewMac \
      npm run api:conformance:apple
    cd ios && xcodebuild -project float-view.xcodeproj -scheme float-view \
      -configuration Debug -sdk iphonesimulator \
      -derivedDataPath /tmp/float-view-ios-derived CODE_SIGNING_ALLOWED=NO build
