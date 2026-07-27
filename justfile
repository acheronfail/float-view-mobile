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

build-android:
  cd android && ./gradlew test assembleDebug

build-ios:
  CLANG_MODULE_CACHE_PATH=/tmp/float-view-clang swiftc -typecheck \
    -target arm64-apple-ios17.0-simulator \
    -sdk /Applications/Xcode.app/Contents/Developer/Platforms/iPhoneSimulator.platform/Developer/SDKs/iPhoneSimulator26.2.sdk \
    ios/float-view/ContentView.swift ios/float-view/FloatViewApp.swift ios/float-view/WebView.swift \
    apple/Sources/FloatViewCore/Domain.swift apple/Sources/FloatViewCore/AdaptiveLogging.swift \
    apple/Sources/FloatViewCore/ApplicationService.swift \
    apple/Sources/FloatViewHost/LoopbackServer.swift

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

conformance-mock:
  cd web && npm run api:conformance:mock

conformance-macos:
  cd apple && CLANG_MODULE_CACHE_PATH=/tmp/float-view-clang \
    swift build --disable-sandbox --scratch-path /tmp/float-view-swift-build --product FloatViewMac
  cd web && FLOAT_VIEW_APPLE_HOST=/tmp/float-view-swift-build/arm64-apple-macosx/debug/FloatViewMac \
    npm run api:conformance:apple

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
