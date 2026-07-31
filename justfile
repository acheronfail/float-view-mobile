_default:
    @just --list

setup:
    cd frontend && npm ci
    cd conformance && npm ci
    pio pkg install -d tools/vesc-test-device
    mkdir -p vendor
    [ -e "vendor/bldc" ] || git clone https://github.com/vedderb/bldc "vendor/bldc"
    [ -e "vendor/refloat" ] || git clone https://github.com/lukash/refloat "vendor/refloat"
    [ -e "vendor/vesc_tool" ] || git clone https://github.com/vedderb/vesc_tool "vendor/vesc_tool"
    [ -e "vendor/vesc_bms_fw" ] || git clone https://github.com/vedderb/vesc_bms_fw "vendor/vesc_bms_fw"
    [ -e "vendor/float-view" ] || git clone https://github.com/acheronfail/float-view "vendor/float-view"

dev platform="mock" target="":
    cd conformance && node --strip-types src/dev.ts "{{ platform }}" "{{ target }}"

build:
    cd frontend && npm run build
    cp frontend/build/index.html backends/apple/Sources/FloatViewMac/Resources/index.html
    mkdir -p backends/apple/Sources/FloatViewMac/Resources/sql
    cp backends/sql/catalog.sql backends/apple/Sources/FloatViewMac/Resources/sql/catalog.sql
    cp backends/sql/session.sql backends/apple/Sources/FloatViewMac/Resources/sql/session.sql

run platform target="":
    #!/usr/bin/env bash
    set -euo pipefail
    just build
    case "{{ platform }}" in
      android)
        (cd backends/android && ./gradlew assembleDebug)
        serial="{{ target }}"
        adb_args=()
        [[ -z "$serial" ]] || adb_args=(-s "$serial")
        adb "${adb_args[@]}" install -r backends/android/app/build/outputs/apk/debug/app-debug.apk
        adb "${adb_args[@]}" shell am start -W \
          -n fail.acheron.floatview/fail.acheron.floatview.MainActivity
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
            --terminate-existing --quiet fail.acheron.float-view
        fi
        ;;
      macos)
        frontend_html="$PWD/frontend/build/index.html"
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
      apple) (cd backends/apple && swift test --scratch-path /tmp/float-view-swift-build) ;;
      android) (cd backends/android && ./gradlew testDebugUnitTest) ;;
      fixture) sh tools/vesc-test-device/test/run.sh ;;
      all)
        just test frontend
        just test conformance
        just test apple
        just test android
        just test fixture
        ;;
      *) echo 'Usage: just test frontend|conformance|apple|android|fixture|all' >&2; exit 2 ;;
    esac

conformance target:
    cd conformance && npm run {{ target }}

conformance-physical platform fixture_port target="":
    #!/usr/bin/env bash
    set -euo pipefail
    just build
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
    cmp -s frontend/build/index.html backends/apple/Sources/FloatViewMac/Resources/index.html
    cmp -s backends/sql/catalog.sql \
      backends/apple/Sources/FloatViewMac/Resources/sql/catalog.sql
    cmp -s backends/sql/session.sql \
      backends/apple/Sources/FloatViewMac/Resources/sql/session.sql
    ! rg -n 'system\.hello|system\.getSnapshot|telemetrySubscriptions|loggingJobs' \
      frontend backends conformance
    ! rg -n '\b([3]001|[5]173|[8]765|[8]766)\b' \
      frontend backends conformance justfile \
      -g '!frontend/build/**' -g '!**/Resources/index.html'

verify:
    just build
    just test all
    cd backends/android && ./gradlew assembleDebug
    cd backends/apple && swift build --scratch-path /tmp/float-view-swift-build --product FloatViewMac
    xcodebuild -project backends/apple/ios/float-view.xcodeproj -scheme float-view \
      -sdk iphonesimulator -destination 'generic/platform=iOS Simulator' \
      -derivedDataPath /tmp/float-view-ios CODE_SIGNING_ALLOWED=NO build
    cd conformance && npm run macos && npm run ios && npm run android
    just audit
