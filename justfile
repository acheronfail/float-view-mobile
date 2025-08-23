_default:
  just -l

areload:
  cd android && ./gradlew installDebug && adb shell am start -n fail.acheron.floatview/.MainActivity