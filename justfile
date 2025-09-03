_default:
  @just --choose

# setup the repository for development
setup:
  npm install
  cd web && npm install
  cd android && ./gradlew dependencies
  just proto

# TODO: make sure `brew install swift-protobuf` is installed before running
protodir_swift := "ios/float-view/proto"
protodir_kotlin := "android/app/src/main/java"
protodir_typescript := "web/src/lib/proto"
proto:
  rm -rf "{{protodir_typescript}}"
  mkdir -p "{{protodir_typescript}}"
  protoc \
    --plugin=node_modules/ts-proto/protoc-gen-ts_proto \
    --proto_path=proto \
    --ts_proto_out="{{protodir_typescript}}" \
    --ts_proto_opt=env=both \
    --ts_proto_opt=oneof=unions-value \
    --ts_proto_opt=outputTypeRegistry=false \
    --ts_proto_opt=outputServices=false \
    --ts_proto_opt=outputClientImpl=false \
    --ts_proto_opt=esModuleInterop=false \
    proto/*.proto

  rm -rf "{{protodir_kotlin}}/fail/acheron/floatview/proto"
  mkdir -p "{{protodir_kotlin}}/fail/acheron/floatview/proto"
  protoc \
    --java_out=lite:"{{protodir_kotlin}}" \
    --kotlin_out=lite:"{{protodir_kotlin}}" \
    --proto_path=proto \
    proto/*.proto

  rm -rf "{{protodir_swift}}"
  mkdir -p "{{protodir_swift}}"
  protoc \
    --swift_out="{{protodir_swift}}" \
    --swift_opt=FileNaming=DropPath \
    --proto_path=proto \
    proto/*.proto

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