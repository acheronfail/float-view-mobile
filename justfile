_default:
  just -l

setup:
  cd web && npm install

dev-web:
  cd web && npm run dev

dev-build:
  cd web && npm run build

# re-install and re-launch the debug app on android (emulator or device)
areload: dev-build
  cd android && ./gradlew installDebug && adb shell am start -n fail.acheron.floatview/.MainActivity

# re-install and re-launch the debug app on ios simulator
ireload: dev-build
  cd ios && xcodebuild -scheme float-view -configuration Debug -sdk iphonesimulator build \
    && xcrun simctl boot "iPhone 16" || true \
    && xcrun simctl launch --console --terminate-running-process booted fail.acheron.float-view