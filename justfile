_default:
  just -l

setup:
  cd web && npm install

wdev:
  cd web && npm run dev

wwatch:
  #!/bin/bash
  set -e
  touch .lock
  trap 'rm -f .lock' EXIT
  cd web && npm run build -- --watch --mode development --minify false

_check_watch:
  if [ ! -f .lock ]; then
    echo "Make sure to run `just wwatch` for fresh web assets!"
  fi

# re-install and re-launch the debug app on android (emulator or device)
adev: _check_watch
  cd android && ./gradlew installDebug && adb shell am start -n fail.acheron.floatview/.MainActivity

# re-install and re-launch the debug app on ios simulator
idev: _check_watch
  cd ios && xcodebuild -scheme float-view -configuration Debug -sdk iphonesimulator build \
    && xcrun simctl boot "iPhone 16" || true \
    && xcrun simctl launch --console --terminate-running-process booted fail.acheron.float-view