_default:
  @just --choose

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
