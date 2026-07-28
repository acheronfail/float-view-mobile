_default:
  @just --choose

doctor:
  #!/usr/bin/env bash
  set -euo pipefail
  node --version
  npm --version
  java -version
  swift --version
  xcodebuild -version
  echo "Android SDK: ${ANDROID_HOME:-${ANDROID_SDK_ROOT:-not configured}}"

verify-r0:
  node scripts/verify-r0.mjs

build-web:
  cd web && npm run check && npm run build

test-apple:
  cd apple && CLANG_MODULE_CACHE_PATH=/tmp/float-view-clang \
    swift test --disable-sandbox --scratch-path /tmp/float-view-swift-build

build-macos:
  cd apple && CLANG_MODULE_CACHE_PATH=/tmp/float-view-clang \
    swift build --disable-sandbox --scratch-path /tmp/float-view-swift-build --product FloatViewMac

# assemble the real macOS app bundle with its web UI and log-store migrations
build-macos-app: build-web build-macos
  #!/usr/bin/env bash
  set -euo pipefail
  bundle_root="/tmp/float-view-macos-app/Float View.app"
  contents="$bundle_root/Contents"
  install -d "$contents/MacOS" "$contents/Resources/web" "$contents/Resources/sql"
  install -m 755 \
    /tmp/float-view-swift-build/arm64-apple-macosx/debug/FloatViewMac \
    "$contents/MacOS/FloatViewMac"
  install -m 644 apple/FloatViewMac-Info.plist "$contents/Info.plist"
  ditto web/build "$contents/Resources/web"
  ditto logging/sql "$contents/Resources/sql"
  plutil -lint "$contents/Info.plist"
  codesign --force --deep --sign - \
    --identifier fail.acheron.float-view.macos \
    "$bundle_root"
  echo "$bundle_root"

# install the signed development bundle at a stable user-local path for TCC
macos-device-install: build-macos-app
  #!/usr/bin/env bash
  set -euo pipefail
  source_bundle="/tmp/float-view-macos-app/Float View.app"
  installed_bundle="$HOME/Applications/Float View.app"
  install -d "$HOME/Applications"
  ditto "$source_bundle" "$installed_bundle"
  codesign --force --deep --sign - \
    --identifier fail.acheron.float-view.macos \
    "$installed_bundle"
  /System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister \
    -f "$installed_bundle"
  echo "$installed_bundle"

# build, install, and launch the real macOS host attached to its console
macos-device-run: macos-device-install
  "$HOME/Applications/Float View.app/Contents/MacOS/FloatViewMac"

# launch the installed macOS app through Launch Services
macos-device-open: macos-device-install
  open -n "$HOME/Applications/Float View.app"

# copy and verify the newest local macOS session without touching the live store
macos-device-verify-latest-session:
  #!/usr/bin/env bash
  set -euo pipefail
  source_root="$HOME/Library/Application Support/FloatView/Sessions"
  if [[ ! -f "$source_root/catalog.sqlite" ]]; then
    echo "No macOS Float View log catalog found." >&2
    exit 1
  fi
  destination="$(mktemp -d /tmp/float-view-macos-session.XXXXXX)"
  cp -R "$source_root/." "$destination/"
  catalog="$destination/catalog.sqlite"
  latest_name="$(sqlite3 "$catalog" \
    'SELECT file_name FROM session_catalog ORDER BY started_at_ms DESC LIMIT 1;')"
  if [[ -z "$latest_name" || ! -f "$destination/$latest_name" ]]; then
    echo "No session database found. Connect once and disconnect cleanly first." >&2
    exit 1
  fi
  echo "Copied macOS logs to $destination"
  sqlite3 "$catalog" <<'SQL'
  PRAGMA integrity_check;
  SELECT
    state, close_reason, committed_sequence
  FROM session_catalog
  ORDER BY started_at_ms DESC
  LIMIT 1;
  SQL
  sqlite3 "$destination/$latest_name" <<'SQL'
  PRAGMA integrity_check;
  SELECT
    count(*) AS samples,
    min(sequence) AS first_sequence,
    max(sequence) AS last_sequence,
    max(monotonic_elapsed_ms) AS elapsed_ms
  FROM sample;
  SELECT extension_json FROM sample ORDER BY sequence DESC LIMIT 1;
  SELECT committed_sequence FROM commit_marker WHERE singleton = 1;
  SQL

build-android:
  cd android && ./gradlew test assembleDebug

build-ios: build-ios-app

build-ios-app:
  cd ios && xcodebuild -project float-view.xcodeproj -scheme float-view \
    -configuration Debug -sdk iphonesimulator \
    -derivedDataPath /tmp/float-view-ios-derived CODE_SIGNING_ALLOWED=NO build

dependency-check:
  node scripts/verify-r1.mjs

verify-r1: verify-r0
  just api-check
  just dependency-check
  just build-web
  just test-apple
  just build-macos
  just build-android
  just build-ios

test-contract:
  cd web && npm run api:check && npm run check && npm run lint && npm test

test-product:
  cd web && npm run api:conformance:mock

test-protocol:
  cd apple && CLANG_MODULE_CACHE_PATH=/tmp/float-view-clang \
    swift test --disable-sandbox --scratch-path /tmp/float-view-swift-build \
    --filter ProtocolFixtureTests
  cd android && ./gradlew :protocol:test

test-core:
  cd apple && CLANG_MODULE_CACHE_PATH=/tmp/float-view-clang \
    swift test --disable-sandbox --scratch-path /tmp/float-view-swift-build
  cd android && ./gradlew :core:test

test-storage:
  cd apple && CLANG_MODULE_CACHE_PATH=/tmp/float-view-clang \
    swift test --disable-sandbox --scratch-path /tmp/float-view-swift-build \
    --filter SQLiteLogStoreTests
  cd android && ./gradlew :logging:test :logging:assembleDebug

test-alerts:
  cd apple && CLANG_MODULE_CACHE_PATH=/tmp/float-view-clang \
    swift test --disable-sandbox --scratch-path /tmp/float-view-swift-build \
    --filter AdaptiveLoggingTests
  cd android && ./gradlew :core:test --tests '*AdaptiveLoggingTest*'

test-profiles:
  cd web && npm run check && npm run lint && npm test
  cd apple && CLANG_MODULE_CACHE_PATH=/tmp/float-view-clang \
    swift test --disable-sandbox --scratch-path /tmp/float-view-swift-build \
    --filter boardProfile
  cd android && ./gradlew :core:test :logging:compileDebugAndroidTestKotlin

test-sessions:
  cd web && npm run check && npx vitest --run src/lib/api/store.spec.ts
  cd apple && CLANG_MODULE_CACHE_PATH=/tmp/float-view-clang \
    swift test --disable-sandbox --scratch-path /tmp/float-view-swift-build \
    --filter session
  cd android && ./gradlew :app:compileDebugKotlin :logging:compileDebugAndroidTestKotlin
  just conformance-mock
  just conformance-macos

test-sessions-android-device:
  cd android && ./gradlew :logging:connectedDebugAndroidTest

conformance-mock:
  cd web && npm run api:conformance:mock

conformance-macos:
  #!/usr/bin/env bash
  set -euo pipefail
  (
    cd apple
    CLANG_MODULE_CACHE_PATH=/tmp/float-view-clang \
      swift build --disable-sandbox --scratch-path /tmp/float-view-swift-build \
      --product FloatViewMac
  )
  test_log_root="$(mktemp -d /tmp/float-view-macos-conformance.XXXXXX)"
  trap 'rm -rf "$test_log_root"' EXIT
  (
    cd web
    FLOAT_VIEW_APPLE_HOST=/tmp/float-view-swift-build/arm64-apple-macosx/debug/FloatViewMac \
      FLOAT_VIEW_LOG_ROOT="$test_log_root" \
      npm run api:conformance:apple
  )

verify-r2: verify-r1
  just conformance-macos

verify-r3: verify-r2
  just test-contract
  just test-product

verify-r4: verify-r3
  just test-protocol

verify-r5: verify-r4
  just test-core
  just test-storage
  just test-alerts
  just test-profiles
  just test-sessions

test-bluetooth:
  cd android && ./gradlew :bluetooth:test :bluetooth:assembleDebug
  cd apple && CLANG_MODULE_CACHE_PATH=/tmp/float-view-clang \
    swift test --disable-sandbox --scratch-path /tmp/float-view-swift-build
  just build-ios-app

test-hardware-smoke:
  node scripts/test-hardware-smoke.mjs

test-hardware-r6-failure:
  node scripts/test-hardware-r6-failure.mjs

# validate a redacted physical run against a strictly read-only hardware profile
hardware-smoke platform profile evidence="":
  node scripts/hardware-smoke.mjs "{{platform}}" "{{profile}}" "{{evidence}}"

# print the guided R6 failure-state procedure and optionally create a new,
# non-overwriting redacted evidence template
hardware-r6-guide platform profile output="":
  node scripts/hardware-r6-failure.mjs guide "{{platform}}" "{{profile}}" "{{output}}"

# validate a completed guided R6 failure-state evidence record
hardware-r6-verify platform profile evidence:
  node scripts/hardware-r6-failure.mjs verify "{{platform}}" "{{profile}}" "{{evidence}}"

verify-r6: verify-r5
  just test-bluetooth
  just test-hardware-smoke
  just test-hardware-r6-failure

# setup the repository for development
setup:
  cd web && npm install
  cd android && ./gradlew dependencies
  just generate

generate:
  cd web && npm run generate

# validate the versioned local API schemas and generated TypeScript models
api-check:
  cd web && npm run api:check

# run the black-box API v1 system suite against the development mock
api-conformance:
  cd web && npm run api:conformance:mock

# compile and round-trip test the generated Swift and Kotlin v1 models
api-native-check:
  swiftc api/generated/swift/FloatViewAPI.swift api/tests/swift/ContractSmoke.swift \
    -o /tmp/float-view-api-contract-smoke
  /tmp/float-view-api-contract-smoke api/examples/system.hello.response.json
  cd android && ./gradlew :api-models:test

# run the web app in dev mode (hot reload, etc)
wdev:
  cd web && npm run dev

# build the web app in watch mode (for development with native apps)
wwatch:
  #!/bin/bash
  set -e
  echo $$ > .wwatch.pid
  trap 'rm -f .wwatch.pid' EXIT INT TERM
  cd web && npm run build -- --watch --mode development --minify false

_check_watch:
  #!/bin/bash
  if [ ! -f .wwatch.pid ]; then
    echo -e "\033[1;33mWarning: Make sure to run \`just wwatch\` for fresh web assets!\033[0m"
    exit
  fi
  # Check if the process is actually running
  if ! kill -0 $(cat .wwatch.pid) 2>/dev/null; then
    echo -e "\033[1;33mWarning: wwatch process is not running (stale PID file). Please run \`just wwatch\` again!\033[0m"
    rm -f .wwatch.pid
    exit
  fi

# re-install and re-launch the debug app on android (emulator or device)
adev: _check_watch
  cd android && ./gradlew installDebug && adb shell am start -n fail.acheron.floatview/.MainActivity

# re-install and re-launch the debug app on ios simulator
idev: _check_watch
  cd ios && xcodebuild -scheme float-view -configuration Debug -sdk iphonesimulator build \
    && xcrun simctl boot "iPhone 16" || true \
    && xcrun simctl launch --console --terminate-running-process booted fail.acheron.float-view

# list physical iOS devices and the IDs accepted by xcodebuild
ios-device-list:
  xcrun xctrace list devices

# build and sign the current web bundle and iOS app for a physical device
ios-device-build device: build-web
  #!/usr/bin/env bash
  set -euo pipefail
  derived_data="${FLOAT_VIEW_IOS_DERIVED_DATA:-/tmp/float-view-ios-device}"
  xcodebuild \
    -project ios/float-view.xcodeproj \
    -scheme float-view \
    -configuration Debug \
    -destination "id={{device}}" \
    -derivedDataPath "$derived_data" \
    -allowProvisioningUpdates \
    build

# build, install, and launch the app on a physical iOS device
ios-device-run device:
  #!/usr/bin/env bash
  set -euo pipefail
  derived_data="${FLOAT_VIEW_IOS_DERIVED_DATA:-/tmp/float-view-ios-device}"
  app_path="$derived_data/Build/Products/Debug-iphoneos/float-view.app"
  just ios-device-build "{{device}}"
  xcrun devicectl device install app --device "{{device}}" "$app_path"
  xcrun devicectl device process launch \
    --device "{{device}}" \
    --terminate-existing \
    fail.acheron.float-view

# launch the installed iOS app attached to its console; Ctrl-C also terminates it
ios-device-console device:
  xcrun devicectl device process launch --device "{{device}}" \
    --terminate-existing --console fail.acheron.float-view

# copy the iOS app-private log store and verify the newest closed session
ios-device-verify-latest-session device destination="":
  #!/usr/bin/env bash
  set -euo pipefail
  destination="{{destination}}"
  if [[ -z "$destination" ]]; then
    destination="$(mktemp -d /tmp/float-view-ios-session.XXXXXX)"
  elif [[ -e "$destination" ]]; then
    echo "Destination already exists; choose a new directory: $destination" >&2
    exit 1
  fi
  xcrun devicectl device copy from \
    --device "{{device}}" \
    --domain-type appDataContainer \
    --domain-identifier fail.acheron.float-view \
    --source 'Library/Application Support/FloatView/Sessions' \
    --destination "$destination" \
    --timeout 30
  catalog="$destination/catalog.sqlite"
  if [[ ! -f "$catalog" ]]; then
    echo "Copied container did not contain the Float View catalog." >&2
    exit 1
  fi
  latest_name="$(sqlite3 "$catalog" \
    'SELECT file_name FROM session_catalog ORDER BY started_at_ms DESC LIMIT 1;')"
  if [[ -z "$latest_name" || ! -f "$destination/$latest_name" ]]; then
    echo "No session database found. Connect once and disconnect cleanly first." >&2
    exit 1
  fi
  echo "Copied iOS logs to $destination"
  sqlite3 "$catalog" <<'SQL'
  PRAGMA integrity_check;
  SELECT
    state, close_reason, committed_sequence
  FROM session_catalog
  ORDER BY started_at_ms DESC
  LIMIT 1;
  SQL
  sqlite3 "$destination/$latest_name" <<'SQL'
  PRAGMA integrity_check;
  SELECT
    count(*) AS samples,
    min(sequence) AS first_sequence,
    max(sequence) AS last_sequence,
    max(monotonic_elapsed_ms) AS elapsed_ms
  FROM sample;
  SELECT extension_json FROM sample ORDER BY sequence DESC LIMIT 1;
  SELECT committed_sequence FROM commit_marker WHERE singleton = 1;
  SQL

# list Android devices and their USB-debugging authorization state
android-device-list:
  adb devices -l

# build the current web bundle and Android debug APK for a physical device
android-device-build: build-web
  cd android && ./gradlew :app:assembleDebug :app:testDebugUnitTest

# build, install, and launch on an authorized Android device
#
# The serial is optional only when exactly one authorized device is attached.
android-device-run serial="":
  #!/usr/bin/env bash
  set -euo pipefail
  serial="{{serial}}"
  if [[ -z "$serial" ]]; then
    serial="$(adb devices | awk 'NR > 1 && $2 == "device" { print $1 }')"
    count="$(printf '%s\n' "$serial" | sed '/^$/d' | wc -l | tr -d ' ')"
    if [[ "$count" != "1" ]]; then
      echo "Expected exactly one authorized Android device; pass its serial explicitly:" >&2
      adb devices -l >&2
      exit 1
    fi
  fi

  just android-device-build
  adb -s "$serial" install -r android/app/build/outputs/apk/debug/app-debug.apk
  adb -s "$serial" logcat -c
  adb -s "$serial" shell am force-stop fail.acheron.floatview
  adb -s "$serial" shell am start -W \
    -n fail.acheron.floatview/fail.acheron.floatview.MainActivity

  pid="$(adb -s "$serial" shell pidof fail.acheron.floatview | tr -d '\r')"
  if [[ -z "$pid" ]]; then
    echo "Float View did not remain running; inspect with: just android-device-logs $serial" >&2
    exit 1
  fi
  echo "Float View is running on $serial with PID $pid"

# print logs from the running Android app, or recent fatal/WebView errors if stopped
android-device-logs serial="":
  #!/usr/bin/env bash
  set -euo pipefail
  serial="{{serial}}"
  if [[ -z "$serial" ]]; then
    serial="$(adb devices | awk 'NR > 1 && $2 == "device" { print $1 }')"
    count="$(printf '%s\n' "$serial" | sed '/^$/d' | wc -l | tr -d ' ')"
    if [[ "$count" != "1" ]]; then
      echo "Expected exactly one authorized Android device; pass its serial explicitly:" >&2
      adb devices -l >&2
      exit 1
    fi
  fi

  pid="$(adb -s "$serial" shell pidof fail.acheron.floatview | tr -d '\r')"
  if [[ -n "$pid" ]]; then
    adb -s "$serial" logcat --pid="$pid" -d -v time
  else
    adb -s "$serial" logcat -d -v time | \
      grep -E 'FATAL EXCEPTION|AndroidRuntime|fail\.acheron\.floatview|chromium'
  fi

# print the bounded, debug-only raw protocol capture used to create sanitized fixtures
android-device-protocol-capture serial="":
  #!/usr/bin/env bash
  set -euo pipefail
  serial="{{serial}}"
  if [[ -z "$serial" ]]; then
    serial="$(adb devices | awk 'NR > 1 && $2 == "device" { print $1 }')"
    count="$(printf '%s\n' "$serial" | sed '/^$/d' | wc -l | tr -d ' ')"
    if [[ "$count" != "1" ]]; then
      echo "Expected exactly one authorized Android device; pass its serial explicitly:" >&2
      adb devices -l >&2
      exit 1
    fi
  fi
  adb -s "$serial" logcat -d -v time \
    -s FloatViewProtocolCapture:D '*:S'

# pull and integrity-check the newest cleanly closed Android session database
android-device-verify-latest-session serial="" output="/tmp/float-view-android-session.sqlite":
  #!/usr/bin/env bash
  set -euo pipefail
  serial="{{serial}}"
  output="{{output}}"
  if [[ -z "$serial" ]]; then
    serial="$(adb devices | awk 'NR > 1 && $2 == "device" { print $1 }')"
    count="$(printf '%s\n' "$serial" | sed '/^$/d' | wc -l | tr -d ' ')"
    if [[ "$count" != "1" ]]; then
      echo "Expected exactly one authorized Android device; pass its serial explicitly:" >&2
      adb devices -l >&2
      exit 1
    fi
  fi
  latest_name="$(adb -s "$serial" shell run-as fail.acheron.floatview \
    ls -t files/float-view/logs | tr -d '\r' |
    awk '/^session-.*\.sqlite$/ { print; exit }')"
  if [[ -z "$latest_name" ]]; then
    echo "No session database found. Connect once and disconnect cleanly first." >&2
    exit 1
  fi
  latest="files/float-view/logs/$latest_name"
  adb -s "$serial" exec-out run-as fail.acheron.floatview cat "$latest" > "$output"
  echo "Pulled $latest to $output"
  catalog_copy="$(mktemp -d /tmp/float-view-android-catalog.XXXXXX)"
  for name in catalog.sqlite catalog.sqlite-wal catalog.sqlite-shm; do
    adb -s "$serial" exec-out run-as fail.acheron.floatview \
      cat "files/float-view/logs/$name" > "$catalog_copy/$name"
  done
  sqlite3 "$catalog_copy/catalog.sqlite" <<'SQL'
  PRAGMA integrity_check;
  SELECT
    state, close_reason, committed_sequence
  FROM session_catalog
  ORDER BY started_at_ms DESC
  LIMIT 1;
  SQL
  # Open normally because a database configured for WAL may need a transient
  # local -shm file even after the Android writer checkpointed and closed it.
  sqlite3 "$output" <<'SQL'
  PRAGMA integrity_check;
  SELECT
    count(*) AS samples,
    min(sequence) AS first_sequence,
    max(sequence) AS last_sequence,
    max(monotonic_elapsed_ms) AS elapsed_ms
  FROM sample;
  SELECT extension_json FROM sample ORDER BY sequence DESC LIMIT 1;
  SELECT committed_sequence FROM commit_marker WHERE singleton = 1;
  SQL

# inspect, deny, or restore the API-level-correct Android Bluetooth permission.
# Deny/restore are test-only reversible device setup actions; they never run as
# part of an unattended verification gate.
android-r6-permission action="status" serial="":
  bash scripts/android-r6-permission.sh "{{action}}" "{{serial}}"
