_default:
    @just --list

setup:
    cd frontend && npm ci
    cd conformance && npm ci
    pio pkg install -d tools/vesc-test-device

dev:
    cd frontend && npm run dev

build:
    cd frontend && npm run build

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
        destination='generic/platform=iOS Simulator'
        [[ -z "{{ target }}" ]] || destination="id={{ target }}"
        xcodebuild -project backends/apple/ios/float-view.xcodeproj -scheme float-view \
          -destination "$destination" -derivedDataPath /tmp/float-view-ios \
          CODE_SIGNING_ALLOWED=NO build
        ;;
      macos)
        (cd backends/apple && swift run --scratch-path /tmp/float-view-swift-build FloatViewMac)
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
      *) echo 'Usage: just fixture setup|check|ports|build|flash|monitor [port]' >&2; exit 2 ;;
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
    [[ "$(find frontend/build -type f | wc -l | tr -d ' ')" == 1 ]]
    [[ -f frontend/build/index.html ]]
    ! rg -n 'system\.hello|system\.getSnapshot|telemetrySubscriptions|loggingJobs' \
      frontend backends conformance

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
