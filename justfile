_default:
  just -l

# re-install and re-launch the debug app on android (emulator or device)
areload:
  cd android && ./gradlew installDebug && adb shell am start -n fail.acheron.floatview/.MainActivity
