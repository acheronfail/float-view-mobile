_default:
    @just --list

core-lint:
    cd backends/core && cargo fmt --check
    cd backends/core && cargo clippy --locked --offline --workspace --all-targets -- -D warnings


apple-rust-runtime configuration="Debug":
    #!/usr/bin/env bash
    set -euo pipefail
    backends/core/build-apple-runtime.sh \
      macosx "$(uname -m)" "{{ configuration }}" \
      "$PWD/backends/core/target/apple/macos"

apple-rust-bridges configuration="Debug":
    just apple-rust-runtime "{{ configuration }}"


apple-packaging:
    just apple-rust-bridges Release
    cd backends/apple && swift build -c release \
      --scratch-path /tmp/float-view-swift-release --product FloatViewMac
    xcodebuild -quiet -project backends/apple/ios/float-view.xcodeproj \
      -scheme float-view -configuration Release -sdk iphonesimulator \
      -destination 'generic/platform=iOS Simulator' \
      -derivedDataPath /tmp/float-view-ios-release CODE_SIGNING_ALLOWED=NO clean build
    xcodebuild -quiet -project backends/apple/ios/float-view.xcodeproj \
      -scheme float-view -configuration Debug -sdk iphoneos \
      -destination 'generic/platform=iOS' \
      -derivedDataPath /tmp/float-view-ios-device CODE_SIGNING_ALLOWED=NO clean build
    just apple-rust-bridges Debug

core-test:
    cd backends/core && cargo test --locked --offline

core-application-runtime-test:
    cd backends/core && cargo test --locked --offline \
      --package float-view-application-runtime

core-application-runtime-audit:
    #!/usr/bin/env bash
    set -euo pipefail
    crate='backends/core/application-runtime'
    [[ -f "$crate/Cargo.toml" && -f "$crate/src/lib.rs" ]]
    [[ -x backends/core/audit-migration-convergence.sh ]]
    [[ -f backends/core/migration-baseline.txt ]]
    rg -q '^#!\[forbid\(unsafe_code\)\]$' "$crate/src/lib.rs"
    [[ "$(sed -n '/^\[dependencies\]$/,$p' "$crate/Cargo.toml")" == \
      $'[dependencies]\nfloat-view-connection-application-coordinator = { path = "../connection-application-coordinator" }\nfloat-view-refloat-telemetry-service = { path = "../refloat-telemetry-service" }\nfloat-view-session-history-kernel = { path = "../session-history-kernel" }\nfloat-view-session-sqlite = { path = "../session-sqlite" }\nfloat-view-vesc-frame = { path = "../vesc-frame" }' ]]
    [[ "$(cd backends/core && cargo tree --locked --offline \
      --package float-view-application-runtime --edges normal \
      --depth 1 --prefix none | wc -l | tr -d ' ')" == 6 ]]
    ! rg -n 'unsafe[[:space:]]*(\{|fn|impl|trait)|extern[[:space:]]+"(C|system)"|no_mangle|Java_|cbindgen|bindgen|uniffi|jni|host_callback|Box<dyn[[:space:]]+Fn|std::thread|tokio|async_std|TcpListener|UdpSocket|std::net' \
      "$crate/src"
    rg -q 'RustApplicationRuntime' backends/apple/Sources/FloatViewBackend/application
    [[ -f backends/core/application-runtime-apple-ffi/Cargo.toml ]]
    [[ ! -d backends/core/application-runtime-android-jni ]]
    [[ "$(git ls-files "$crate" | rg '\.(h|a|so|dylib|xcframework|jar)$' || true)" == "" ]]
    backends/core/audit-migration-convergence.sh
    just core-application-runtime-test

core-refloat-configuration-codec-test:
    cd backends/core && cargo test --locked --offline \
      --package float-view-refloat-configuration-codec

core-refloat-configuration-service-test:
    cd backends/core && cargo test --locked --offline \
      --package float-view-refloat-configuration-service

core-refloat-configuration-service-audit:
    #!/usr/bin/env bash
    set -euo pipefail
    crate='backends/core/refloat-configuration-service'
    [[ -f "$crate/Cargo.toml" && -f "$crate/src/lib.rs" ]]
    [[ -f "$crate/tests/service.rs" && -f "$crate/tests/adversarial.rs" ]]
    rg -q '^#!\[forbid\(unsafe_code\)\]$' "$crate/src/lib.rs"
    [[ "$(sed -n '/^\[dependencies\]$/,$p' "$crate/Cargo.toml")" == \
      $'[dependencies]\nfloat-view-refloat-configuration-codec = { path = "../refloat-configuration-codec" }' ]]
    [[ "$(cd backends/core && cargo tree --locked --offline \
      --package float-view-refloat-configuration-service --edges normal \
      --depth 1 --prefix none | wc -l | tr -d ' ')" == 2 ]]
    ! rg -n 'unsafe[[:space:]]*(\{|fn|impl|trait)|extern[[:space:]]+"(C|system)"|no_mangle|Java_|cbindgen|bindgen|uniffi|jni|host_callback' \
      "$crate/src" "$crate/tests"
    [[ "$(find "$crate" -mindepth 1 -maxdepth 1 -type d \
      ! -name src ! -name tests -print -quit)" == "" ]]
    ! rg -n 'refloat-configuration-service' \
      backends/apple backends/android backends/core/bridge-build-catalog.sh \
      backends/core/build-apple-bridge.sh backends/core/build-android-bridge.sh
    [[ "$(git ls-files "$crate" | rg '\.(h|a|so|dylib|xcframework|jar)$' || true)" == "" ]]
    just core-refloat-configuration-service-test

core-vesc-request-scheduler-test:
    cd backends/core && cargo test --locked --offline \
      --package float-view-vesc-request-scheduler

core-vesc-request-scheduler-audit:
    #!/usr/bin/env bash
    set -euo pipefail
    crate='backends/core/vesc-request-scheduler'
    [[ -f "$crate/Cargo.toml" && -f "$crate/src/lib.rs" ]]
    rg -q '^#!\[forbid\(unsafe_code\)\]$' "$crate/src/lib.rs"
    [[ "$(sed -n '/^\[dependencies\]$/,$p' "$crate/Cargo.toml")" == '[dependencies]' ]]
    [[ "$(cd backends/core && cargo tree --locked --offline \
      --package float-view-vesc-request-scheduler --edges normal \
      --prefix none | wc -l | tr -d ' ')" == 1 ]]
    ! rg -n 'unsafe[[:space:]]*(\{|fn|impl|trait)|extern[[:space:]]+"(C|system)"|no_mangle|Java_|cbindgen|bindgen|uniffi|jni|host_callback|Box<dyn Fn|std::thread|tokio|rusqlite' \
      "$crate/src"
    [[ "$(find "$crate" -mindepth 1 -maxdepth 1 -type d \
      ! -name src -print -quit)" == "" ]]
    ! rg -n 'vesc-request-scheduler' \
      backends/apple backends/android backends/core/bridge-build-catalog.sh \
      backends/core/build-apple-bridge.sh backends/core/build-android-bridge.sh
    [[ "$(git ls-files "$crate" | rg '\.(h|a|so|dylib|xcframework|jar)$' || true)" == "" ]]
    just core-vesc-request-scheduler-test

core-vesc-compatibility-test:
    cd backends/core && cargo test --locked --offline \
      --package float-view-vesc-compatibility

core-vesc-compatibility-audit:
    #!/usr/bin/env bash
    set -euo pipefail
    crate='backends/core/vesc-compatibility'
    [[ -f "$crate/Cargo.toml" && -f "$crate/src/lib.rs" ]]
    rg -q '^#!\[forbid\(unsafe_code\)\]$' "$crate/src/lib.rs"
    [[ "$(sed -n '/^\[dependencies\]$/,/^\[dev-dependencies\]$/p' "$crate/Cargo.toml" | sed '$d')" == \
      $'[dependencies]\nfloat-view-vesc-firmware-identity = { path = "../vesc-firmware-identity" }' ]]
    [[ "$(cd backends/core && cargo tree --locked --offline \
      --package float-view-vesc-compatibility --edges normal \
      --depth 1 --prefix none | wc -l | tr -d ' ')" == 2 ]]
    ! rg -n 'unsafe[[:space:]]*(\{|fn|impl|trait)|extern[[:space:]]+"(C|system)"|no_mangle|Java_|cbindgen|bindgen|uniffi|jni|host_callback|Box<dyn Fn|std::thread|tokio|async_std|rusqlite|sqlite|TcpListener|UdpSocket|std::fs|std::net' \
      "$crate/src"
    [[ "$(find "$crate" -mindepth 1 -maxdepth 1 -type d \
      ! -name src -print -quit)" == "" ]]
    ! rg -n 'vesc-compatibility' \
      backends/apple backends/android backends/core/bridge-build-catalog.sh \
      backends/core/build-apple-bridge.sh backends/core/build-android-bridge.sh
    actual_bridges="$(sed -n 's/^    \([a-z][a-z-]*\))$/\1/p' \
      backends/core/bridge-build-catalog.sh)"
    expected_bridges="$(printf '%s\n' \
      frame firmware-identity custom-configuration bms-values controller-data \
      refloat-control refloat-compatibility refloat-dynamic-telemetry \
      refloat-legacy-telemetry refloat-configuration-codec)"
    [[ "$actual_bridges" == "$expected_bridges" ]]
    [[ -f backends/apple/Sources/FloatViewBackend/protocol/vesc/VescCompatibility.swift ]]
    [[ -f backends/android/app/src/main/java/fail/acheron/floatview/protocol/vesc/VescCompatibility.kt ]]
    [[ "$(git ls-files "$crate" | rg '\.(h|a|so|dylib|xcframework|jar)$' || true)" == "" ]]
    just core-vesc-compatibility-test

core-controller-discovery-test:
    cd backends/core && cargo test --locked --offline \
      --package float-view-controller-discovery

core-controller-discovery-audit:
    #!/usr/bin/env bash
    set -euo pipefail
    crate='backends/core/controller-discovery'
    [[ -f "$crate/Cargo.toml" && -f "$crate/src/lib.rs" ]]
    [[ -f "$crate/tests/vectors.rs" && -f "$crate/tests/adversarial.rs" ]]
    rg -q '^#!\[forbid\(unsafe_code\)\]$' "$crate/src/lib.rs"
    [[ "$(sed -n '/^\[dependencies\]$/,$p' "$crate/Cargo.toml")" == \
      $'[dependencies]\nfloat-view-refloat-compatibility = { path = "../refloat-compatibility" }\nfloat-view-vesc-compatibility = { path = "../vesc-compatibility" }\nfloat-view-vesc-firmware-identity = { path = "../vesc-firmware-identity" }' ]]
    [[ "$(cd backends/core && cargo tree --locked --offline \
      --package float-view-controller-discovery --edges normal \
      --depth 1 --prefix none | wc -l | tr -d ' ')" == 4 ]]
    ! rg -n 'unsafe[[:space:]]*(\{|fn|impl|trait)|extern[[:space:]]+"(C|system)"|no_mangle|Java_|cbindgen|bindgen|uniffi|jni|host_callback|Box<dyn Fn|std::thread|tokio|async_std|rusqlite|sqlite|TcpListener|UdpSocket|std::fs|std::net' \
      "$crate/src" "$crate/tests"
    [[ "$(find "$crate" -mindepth 1 -maxdepth 1 -type d \
      ! -name src ! -name tests -print -quit)" == "" ]]
    ! rg -n 'controller-discovery' \
      backends/apple/Package.swift \
      backends/apple/ios/float-view.xcodeproj/project.pbxproj \
      backends/android/app/build.gradle.kts \
      backends/core/bridge-build-catalog.sh \
      backends/core/build-apple-bridge.sh backends/core/build-android-bridge.sh
    actual_bridges="$(sed -n 's/^    \([a-z][a-z-]*\))$/\1/p' \
      backends/core/bridge-build-catalog.sh)"
    expected_bridges="$(printf '%s\n' \
      frame firmware-identity custom-configuration bms-values controller-data \
      refloat-control refloat-compatibility refloat-dynamic-telemetry \
      refloat-legacy-telemetry refloat-configuration-codec)"
    [[ "$actual_bridges" == "$expected_bridges" ]]
    [[ -f backends/apple/Sources/FloatViewBackend/application/ControllerDiscovery.swift ]]
    [[ -f backends/android/app/src/main/java/fail/acheron/floatview/application/ControllerDiscovery.kt ]]
    [[ "$(git ls-files "$crate" | rg '\.(h|a|so|dylib|xcframework|jar)$' || true)" == "" ]]
    just core-controller-discovery-test

core-auxiliary-node-discovery-test:
    cd backends/core && cargo test --locked --offline \
      --package float-view-auxiliary-node-discovery

core-auxiliary-node-discovery-audit:
    #!/usr/bin/env bash
    set -euo pipefail
    crate='backends/core/auxiliary-node-discovery'
    [[ -f "$crate/Cargo.toml" && -f "$crate/src/lib.rs" ]]
    [[ -f "$crate/tests/vectors.rs" && -f "$crate/tests/adversarial.rs" ]]
    [[ -f backends/fixtures/connection/auxiliary-node-discovery.tsv ]]
    [[ "$(sed '/^#/d;/^$/d' backends/fixtures/connection/auxiliary-node-discovery.tsv | wc -l | tr -d ' ')" == 13 ]]
    rg -q '^#!\[forbid\(unsafe_code\)\]$' "$crate/src/lib.rs"
    [[ "$(sed -n '/^\[dependencies\]$/,/^\[dev-dependencies\]$/p' "$crate/Cargo.toml" | sed '$d')" == \
      $'[dependencies]\nfloat-view-controller-discovery = { path = "../controller-discovery" }' ]]
    [[ "$(cd backends/core && cargo tree --locked --offline \
      --package float-view-auxiliary-node-discovery --edges normal \
      --depth 1 --prefix none | wc -l | tr -d ' ')" == 2 ]]
    ! rg -n 'unsafe[[:space:]]*(\{|fn|impl|trait)|extern[[:space:]]+"(C|system)"|no_mangle|Java_|cbindgen|bindgen|uniffi|jni|host_callback|Box<dyn Fn|std::thread|tokio|async_std|rusqlite|sqlite|TcpListener|UdpSocket|std::fs|std::net' \
      "$crate/src" "$crate/tests"
    [[ "$(find "$crate" -mindepth 1 -maxdepth 1 -type d \
      ! -name src ! -name tests -print -quit)" == "" ]]
    ! rg -n 'auxiliary-node-discovery' \
      backends/apple/Package.swift \
      backends/apple/ios/float-view.xcodeproj/project.pbxproj \
      backends/android/app/build.gradle.kts \
      backends/core/bridge-build-catalog.sh \
      backends/core/build-apple-bridge.sh backends/core/build-android-bridge.sh
    actual_bridges="$(sed -n 's/^    \([a-z][a-z-]*\))$/\1/p' \
      backends/core/bridge-build-catalog.sh)"
    expected_bridges="$(printf '%s\n' \
      frame firmware-identity custom-configuration bms-values controller-data \
      refloat-control refloat-compatibility refloat-dynamic-telemetry \
      refloat-legacy-telemetry refloat-configuration-codec)"
    [[ "$actual_bridges" == "$expected_bridges" ]]
    [[ -f backends/apple/Sources/FloatViewBackend/application/AuxiliaryNodeDiscovery.swift ]]
    [[ -f backends/android/app/src/main/java/fail/acheron/floatview/application/AuxiliaryNodeDiscovery.kt ]]
    [[ "$(git ls-files "$crate" | rg '\.(h|a|so|dylib|xcframework|jar)$' || true)" == "" ]]
    just core-auxiliary-node-discovery-test

core-vesc-bms-telemetry-service-test:
    cd backends/core && cargo test --locked --offline \
      --package float-view-vesc-bms-telemetry-service

core-vesc-bms-telemetry-service-audit:
    #!/usr/bin/env bash
    set -euo pipefail
    crate='backends/core/vesc-bms-telemetry-service'
    [[ -f "$crate/Cargo.toml" && -f "$crate/src/lib.rs" ]]
    [[ -f "$crate/tests/vectors.rs" && -f "$crate/tests/adversarial.rs" ]]
    [[ -f backends/fixtures/connection/vesc-bms-telemetry-service.tsv ]]
    [[ "$(sed '/^#/d;/^$/d' backends/fixtures/connection/vesc-bms-telemetry-service.tsv | wc -l | tr -d ' ')" == 12 ]]
    rg -q '^#!\[forbid\(unsafe_code\)\]$' "$crate/src/lib.rs"
    [[ "$(sed -n '/^\[dependencies\]$/,/^\[dev-dependencies\]$/p' "$crate/Cargo.toml" | sed '$d')" == \
      $'[dependencies]\nfloat-view-auxiliary-node-discovery = { path = "../auxiliary-node-discovery" }\nfloat-view-vesc-bms-values = { path = "../vesc-bms-values" }\nfloat-view-vesc-request-scheduler = { path = "../vesc-request-scheduler" }' ]]
    [[ "$(cd backends/core && cargo tree --locked --offline \
      --package float-view-vesc-bms-telemetry-service --edges normal \
      --depth 1 --prefix none | wc -l | tr -d ' ')" == 4 ]]
    ! rg -n 'unsafe[[:space:]]*(\{|fn|impl|trait)|extern[[:space:]]+"(C|system)"|no_mangle|Java_|cbindgen|bindgen|uniffi|jni|host_callback|Box<dyn Fn|std::thread|tokio|async_std|rusqlite|sqlite|TcpListener|UdpSocket|std::fs|std::net' \
      "$crate/src" "$crate/tests"
    [[ "$(find "$crate" -mindepth 1 -maxdepth 1 -type d \
      ! -name src ! -name tests -print -quit)" == "" ]]
    ! rg -n 'vesc-bms-telemetry-service' \
      backends/apple backends/android backends/core/bridge-build-catalog.sh \
      backends/core/build-apple-bridge.sh backends/core/build-android-bridge.sh
    actual_bridges="$(sed -n 's/^    \([a-z][a-z-]*\))$/\1/p' \
      backends/core/bridge-build-catalog.sh)"
    expected_bridges="$(printf '%s\n' \
      frame firmware-identity custom-configuration bms-values controller-data \
      refloat-control refloat-compatibility refloat-dynamic-telemetry \
      refloat-legacy-telemetry refloat-configuration-codec)"
    [[ "$actual_bridges" == "$expected_bridges" ]]
    [[ -f backends/apple/Sources/FloatViewBackend/application/ApplicationService.swift ]]
    [[ -f backends/apple/Sources/FloatViewBackend/application/AuxiliaryNodeDiscovery.swift ]]
    [[ -f backends/android/app/src/main/java/fail/acheron/floatview/application/ApplicationService.kt ]]
    [[ -f backends/android/app/src/main/java/fail/acheron/floatview/application/AuxiliaryNodeDiscovery.kt ]]
    [[ "$(git ls-files "$crate" | rg '\.(h|a|so|dylib|xcframework|jar)$' || true)" == "" ]]
    just core-vesc-bms-telemetry-service-test

core-refloat-telemetry-service-test:
    cd backends/core && cargo test --locked --offline \
      --package float-view-refloat-telemetry-service

core-refloat-telemetry-service-audit:
    #!/usr/bin/env bash
    set -euo pipefail
    crate='backends/core/refloat-telemetry-service'
    [[ -f "$crate/Cargo.toml" && -f "$crate/src/lib.rs" ]]
    rg -q '^#!\[forbid\(unsafe_code\)\]$' "$crate/src/lib.rs"
    [[ "$(sed -n '/^\[dependencies\]$/,$p' "$crate/Cargo.toml")" == \
      $'[dependencies]\nfloat-view-refloat-dynamic-telemetry = { path = "../refloat-dynamic-telemetry" }\nfloat-view-refloat-legacy-telemetry = { path = "../refloat-legacy-telemetry" }\nfloat-view-vesc-request-scheduler = { path = "../vesc-request-scheduler" }' ]]
    [[ "$(cd backends/core && cargo tree --locked --offline \
      --package float-view-refloat-telemetry-service --edges normal \
      --depth 1 --prefix none | wc -l | tr -d ' ')" == 4 ]]
    ! rg -n 'unsafe[[:space:]]*(\{|fn|impl|trait)|extern[[:space:]]+"(C|system)"|no_mangle|Java_|cbindgen|bindgen|uniffi|jni|host_callback|Box<dyn Fn|std::thread|tokio|async_std|rusqlite|sqlite|TcpListener|UdpSocket|std::fs|std::net' \
      "$crate/src"
    [[ "$(find "$crate" -mindepth 1 -maxdepth 1 -type d \
      ! -name src -print -quit)" == "" ]]
    ! rg -n 'refloat-telemetry-service' \
      backends/apple backends/android backends/core/bridge-build-catalog.sh \
      backends/core/build-apple-bridge.sh backends/core/build-android-bridge.sh
    [[ "$(git ls-files "$crate" | rg '\.(h|a|so|dylib|xcframework|jar)$' || true)" == "" ]]
    just core-refloat-telemetry-service-test

core-connection-application-coordinator-test:
    cd backends/core && cargo test --locked --offline \
      --package float-view-connection-application-coordinator

core-connection-application-coordinator-audit:
    #!/usr/bin/env bash
    set -euo pipefail
    crate='backends/core/connection-application-coordinator'
    [[ -f "$crate/Cargo.toml" && -f "$crate/src/lib.rs" ]]
    [[ -f "$crate/tests/vectors.rs" && -f "$crate/tests/adversarial.rs" && -f "$crate/tests/composition.rs" && -f "$crate/tests/current_limits.rs" ]]
    [[ -f backends/fixtures/connection/connection-application-coordinator.tsv ]]
    [[ "$(sed '/^#/d;/^$/d' backends/fixtures/connection/connection-application-coordinator.tsv | wc -l | tr -d ' ')" == 8 ]]
    [[ -f backends/fixtures/connection/device-configuration-composition.tsv ]]
    [[ "$(sed '/^#/d;/^$/d' backends/fixtures/connection/device-configuration-composition.tsv | wc -l | tr -d ' ')" == 12 ]]
    [[ -f backends/fixtures/connection/controller-current-limits.tsv ]]
    [[ "$(sed '/^#/d;/^$/d' backends/fixtures/connection/controller-current-limits.tsv | wc -l | tr -d ' ')" == 4 ]]
    rg -q '^#!\[forbid\(unsafe_code\)\]$' "$crate/src/lib.rs"
    [[ "$(sed -n '/^\[dependencies\]$/,$p' "$crate/Cargo.toml")" == \
      $'[dependencies]\nfloat-view-controller-discovery = { path = "../controller-discovery" }\nfloat-view-device-mutation-coordinator = { path = "../device-mutation-coordinator" }\nfloat-view-refloat-compatibility = { path = "../refloat-compatibility" }\nfloat-view-refloat-configuration-codec = { path = "../refloat-configuration-codec" }\nfloat-view-refloat-configuration-service = { path = "../refloat-configuration-service" }\nfloat-view-refloat-dynamic-telemetry = { path = "../refloat-dynamic-telemetry" }\nfloat-view-refloat-legacy-telemetry = { path = "../refloat-legacy-telemetry" }\nfloat-view-refloat-telemetry-service = { path = "../refloat-telemetry-service" }\nfloat-view-vesc-bms-telemetry-service = { path = "../vesc-bms-telemetry-service" }\nfloat-view-vesc-custom-configuration = { path = "../vesc-custom-configuration" }\nfloat-view-vesc-controller-data = { path = "../vesc-controller-data" }\nfloat-view-vesc-firmware-identity = { path = "../vesc-firmware-identity" }\nfloat-view-vesc-request-scheduler = { path = "../vesc-request-scheduler" }' ]]
    [[ "$(cd backends/core && cargo tree --locked --offline \
      --package float-view-connection-application-coordinator --edges normal \
      --depth 1 --prefix none | wc -l | tr -d ' ')" == 14 ]]
    ! rg -n 'unsafe[[:space:]]*(\{|fn|impl|trait)|extern[[:space:]]+"(C|system)"|no_mangle|Java_|cbindgen|bindgen|uniffi|jni|host_callback|Box<dyn Fn|std::thread|tokio|async_std|rusqlite|sqlite|TcpListener|UdpSocket|std::fs|std::net' \
      "$crate/src" "$crate/tests"
    ! rg -n 'const[[:space:]]+[A-Z0-9_]+.*(92|93|94|95)|fn[[:space:]]+(schema_request|values_request|write_request|decode_response)' \
      "$crate/src"
    ! rg -n 'fn[[:space:]]+(decode_current_limits|float32_auto)|CURRENT_LIMITS_CONSUMED_BYTES|BATTERY_SERIES_CELL_OFFSETS' \
      "$crate/src"
    [[ "$(find "$crate" -mindepth 1 -maxdepth 1 -type d \
      ! -name src ! -name tests -print -quit)" == "" ]]
    ! rg -n 'connection-application-coordinator' \
      backends/apple backends/android backends/core/bridge-build-catalog.sh \
      backends/core/build-apple-bridge.sh backends/core/build-android-bridge.sh
    [[ -f backends/apple/Sources/FloatViewBackend/application/ApplicationService.swift ]]
    [[ -f backends/android/app/src/main/java/fail/acheron/floatview/application/ApplicationService.kt ]]
    [[ -f backends/apple/Sources/FloatViewBackend/application/RefloatConfigurationService.swift ]]
    [[ -f backends/android/app/src/main/java/fail/acheron/floatview/application/RefloatConfigurationService.kt ]]
    [[ -f backends/apple/Sources/FloatViewBackend/application/RefloatMutationCoordinator.swift ]]
    [[ -f backends/android/app/src/main/java/fail/acheron/floatview/application/RefloatMutationCoordinator.kt ]]
    [[ "$(git ls-files "$crate" | rg '\.(h|a|so|dylib|xcframework|jar)$' || true)" == "" ]]
    just core-connection-application-coordinator-test

core-session-history-kernel-test:
    cd backends/core && cargo test --locked --offline \
      --package float-view-session-history-kernel

core-session-history-kernel-audit:
    #!/usr/bin/env bash
    set -euo pipefail
    crate='backends/core/session-history-kernel'
    fixtures='backends/fixtures/session-history'
    [[ -f "$crate/Cargo.toml" && -f "$crate/src/lib.rs" ]]
    [[ -f "$crate/tests/vectors.rs" && -f "$crate/tests/adversarial.rs" && -f "$crate/tests/support/mod.rs" ]]
    for fixture in series.tsv series-expected.tsv prune.tsv prune-expected.tsv \
      prune-eight-cap.tsv prune-eight-cap-expected.tsv; do
      [[ -f "$fixtures/$fixture" ]]
    done
    [[ "$(sed '/^#/d;/^$/d' "$fixtures/series.tsv" | wc -l | tr -d ' ')" == 9 ]]
    [[ "$(sed '/^#/d;/^$/d' "$fixtures/series-expected.tsv" | wc -l | tr -d ' ')" == 17 ]]
    [[ "$(sed '/^#/d;/^$/d' "$fixtures/prune.tsv" | wc -l | tr -d ' ')" == 24 ]]
    [[ "$(sed '/^#/d;/^$/d' "$fixtures/prune-expected.tsv" | wc -l | tr -d ' ')" == 10 ]]
    [[ "$(sed '/^#/d;/^$/d' "$fixtures/prune-eight-cap.tsv" | wc -l | tr -d ' ')" == 11 ]]
    [[ "$(sed '/^#/d;/^$/d' "$fixtures/prune-eight-cap-expected.tsv" | wc -l | tr -d ' ')" == 10 ]]
    rg -q '^#!\[forbid\(unsafe_code\)\]$' "$crate/src/lib.rs"
    [[ "$(sed -n '/^\[dependencies\]$/,$p' "$crate/Cargo.toml")" == '[dependencies]' ]]
    [[ "$(cd backends/core && cargo tree --locked --offline \
      --package float-view-session-history-kernel --edges normal \
      --prefix none | wc -l | tr -d ' ')" == 1 ]]
    ! rg -n 'unsafe[[:space:]]*(\{|fn|impl|trait)|extern[[:space:]]+"(C|system)"|no_mangle|Java_|cbindgen|bindgen|uniffi|jni|host_callback|Box<dyn Fn|std::thread|tokio|async_std|rusqlite|libsqlite|sqlite3_|TcpListener|UdpSocket|std::fs|std::net|serde|base64' \
      "$crate/src" "$crate/tests"
    ! rg -n 'SessionHistoryReducer\.(swift|kt)|session-reference\.ts|session-vectors\.ts|prune-vectors\.ts|include_(str|bytes)!.*(frontend|apple|android)' \
      "$crate"
    [[ "$(find "$crate" -mindepth 1 -maxdepth 1 -type d \
      ! -name src ! -name tests -print -quit)" == "" ]]
    ! rg -n 'session-history-kernel' \
      backends/apple/Package.swift \
      backends/apple/ios/float-view.xcodeproj/project.pbxproj \
      backends/android/app/build.gradle.kts \
      backends/core/bridge-build-catalog.sh \
      backends/core/build-apple-bridge.sh backends/core/build-android-bridge.sh
    [[ ! -e backends/core/session-history-apple-ffi ]]
    [[ ! -e backends/core/session-history-android-jni ]]
    [[ -f backends/apple/Sources/FloatViewBackend/application/SessionHistoryReducer.swift ]]
    [[ -f backends/android/app/src/main/java/fail/acheron/floatview/application/SessionHistoryReducer.kt ]]
    git diff --quiet HEAD -- backends/apple backends/android frontend backends/sql \
      ':(exclude)backends/apple/ios/float-view.xcodeproj/project.pbxproj'
    [[ "$(git ls-files "$crate" | rg '\.(h|a|so|dylib|xcframework|jar)$' || true)" == "" ]]
    just core-session-history-kernel-test

core-session-sqlite-test:
    cd backends/core && cargo test --locked --offline \
      --package float-view-session-sqlite

core-session-sqlite-audit:
    #!/usr/bin/env bash
    set -euo pipefail
    crate='backends/core/session-sqlite'
    fixtures='backends/fixtures/session-sqlite'
    [[ -f "$crate/Cargo.toml" && -f "$crate/src/lib.rs" && -f "$crate/src/reader.rs" && -f "$crate/src/pruner.rs" ]]
    [[ -f "$crate/tests/vectors.rs" && -f "$crate/tests/adversarial.rs" && -f "$crate/tests/reader.rs" && -f "$crate/tests/pruner.rs" && -f "$crate/tests/support/mod.rs" ]]
    for fixture in samples.tsv samples-expected.tsv locations.tsv locations-expected.tsv reader-requests.tsv reader-expected.tsv prune-vectors.tsv prune-expected.tsv; do
      [[ -f "$fixtures/$fixture" ]]
    done
    rg -q '^#!\[forbid\(unsafe_code\)\]$' "$crate/src/lib.rs"
    rg -q 'include_str!\("\.\./\.\./\.\./sql/session\.sql"\)' "$crate/src/lib.rs"
    [[ "$(cd backends/core && cargo tree --locked --offline \
      --package float-view-session-sqlite --edges normal --depth 1 \
      --prefix none | wc -l | tr -d ' ')" == 5 ]]
    [[ "$(sed -n '/^\[dependencies\]$/,/^\[dev-dependencies\]$/p' "$crate/Cargo.toml")" == \
      $'[dependencies]\nfloat-view-refloat-telemetry-service = { path = "../refloat-telemetry-service" }\nfloat-view-session-history-kernel = { path = "../session-history-kernel" }\nrusqlite = { version = "0.37.0", features = ["bundled"] }\nserde_json = "1.0.149"\n\n[dev-dependencies]' ]]
    ! rg -n 'unsafe[[:space:]]*\(|extern[[:space:]]+"(C|system)"|no_mangle|Java_|cbindgen|bindgen|uniffi|jni|host_callback|Box<dyn Fn|TcpListener|UdpSocket|std::net|Swift|Kotlin|Catalog|Loopback|WebSocket|Generated' \
      "$crate/src" "$crate/tests"
    ! rg -n 'std::thread|tokio|async_std' "$crate/src"
    rg -q 'session_history_kernel::.*Prune|session_history_kernel::\{' "$crate/src/pruner.rs"
    ! rg -n 'pub fn [^(]*\([^)]*(sql|query)|pub (struct|enum) .*Sql|pub (fn|type|use).*rusqlite|pub [a-z_]+:.*rusqlite' "$crate/src"
    ! rg -n 'session-sqlite(-writer)?' \
      backends/apple/Package.swift \
      backends/apple/ios/float-view.xcodeproj/project.pbxproj \
      backends/android/app/build.gradle.kts \
      backends/core/bridge-build-catalog.sh \
      backends/core/build-apple-bridge.sh backends/core/build-android-bridge.sh
    [[ ! -e backends/core/session-sqlite-writer ]]
    [[ ! -e backends/fixtures/session-sqlite-writer ]]
    [[ ! -e backends/core/session-sqlite-apple-ffi ]]
    [[ ! -e backends/core/session-sqlite-android-jni ]]
    [[ ! -e backends/core/session-pruning-apple-ffi ]]
    [[ ! -e backends/core/session-pruning-android-jni ]]
    git diff --quiet HEAD -- backends/apple backends/android frontend backends/sql \
      ':(exclude)backends/apple/ios/float-view.xcodeproj/project.pbxproj'
    [[ "$(git ls-files "$crate" | rg '\.(h|a|so|dylib|xcframework|jar)$' || true)" == "" ]]
    just core-session-sqlite-test

core-device-mutation-coordinator-test:
    cd backends/core && cargo test --locked --offline \
      --package float-view-device-mutation-coordinator

core-device-mutation-coordinator-audit:
    #!/usr/bin/env bash
    set -euo pipefail
    crate='backends/core/device-mutation-coordinator'
    [[ -f "$crate/Cargo.toml" && -f "$crate/src/lib.rs" ]]
    [[ -f "$crate/tests/vectors.rs" && -f "$crate/tests/adversarial.rs" ]]
    [[ -f backends/fixtures/connection/device-mutation-coordinator.tsv ]]
    [[ "$(sed '/^#/d;/^$/d' backends/fixtures/connection/device-mutation-coordinator.tsv | wc -l | tr -d ' ')" == 16 ]]
    rg -q '^#!\[forbid\(unsafe_code\)\]$' "$crate/src/lib.rs"
    [[ "$(sed -n '/^\[dependencies\]$/,$p' "$crate/Cargo.toml")" == \
      $'[dependencies]\nfloat-view-refloat-compatibility = { path = "../refloat-compatibility" }\nfloat-view-refloat-control = { path = "../refloat-control" }\nfloat-view-refloat-telemetry-service = { path = "../refloat-telemetry-service" }\nfloat-view-vesc-compatibility = { path = "../vesc-compatibility" }\nfloat-view-vesc-firmware-identity = { path = "../vesc-firmware-identity" }\nfloat-view-vesc-request-scheduler = { path = "../vesc-request-scheduler" }' ]]
    [[ "$(cd backends/core && cargo tree --locked --offline \
      --package float-view-device-mutation-coordinator --edges normal \
      --depth 1 --prefix none | wc -l | tr -d ' ')" == 7 ]]
    ! rg -n 'unsafe[[:space:]]*(\{|fn|impl|trait)|extern[[:space:]]+"(C|system)"|no_mangle|Java_|cbindgen|bindgen|uniffi|jni|host_callback|Box<dyn Fn|std::thread|tokio|async_std|rusqlite|sqlite|TcpListener|UdpSocket|std::fs|std::net' \
      "$crate/src" "$crate/tests"
    [[ "$(find "$crate" -mindepth 1 -maxdepth 1 -type d \
      ! -name src ! -name tests -print -quit)" == "" ]]
    ! rg -n 'device-mutation-coordinator' \
      backends/apple backends/android backends/core/bridge-build-catalog.sh \
      backends/core/build-apple-bridge.sh backends/core/build-android-bridge.sh
    [[ -f backends/apple/Sources/FloatViewBackend/application/RefloatMutationCoordinator.swift ]]
    [[ -f backends/android/app/src/main/java/fail/acheron/floatview/application/RefloatMutationCoordinator.kt ]]
    [[ -f backends/apple/Sources/FloatViewBackend/application/ApplicationService.swift ]]
    [[ -f backends/android/app/src/main/java/fail/acheron/floatview/application/ApplicationService.kt ]]
    [[ "$(git ls-files "$crate" | rg '\.(h|a|so|dylib|xcframework|jar)$' || true)" == "" ]]
    just core-device-mutation-coordinator-test

core-refloat-configuration-codec-audit:
    #!/usr/bin/env bash
    set -euo pipefail
    crate='backends/core/refloat-configuration-codec'
    [[ -f "$crate/Cargo.toml" && -f "$crate/src/lib.rs" ]]
    [[ -f "$crate/tests/fixtures.rs" && -f "$crate/tests/adversarial.rs" ]]
    rg -q '^#!\[forbid\(unsafe_code\)\]$' "$crate/src/lib.rs"
    [[ "$(sed -n '/^\[dependencies\]$/,/^\[/p' "$crate/Cargo.toml" | sed '$d')" == \
      $'[dependencies]\nflate2 = { version = "=1.1.9", default-features = false, features = ["rust_backend"] }\nquick-xml = { version = "=0.39.4", default-features = false }\nsha2 = { version = "=0.11.0", default-features = false }' ]]
    dependency_names="$(cd backends/core && cargo tree --locked --offline \
      --package float-view-refloat-configuration-codec --edges normal \
      --prefix none --format '{p}' | sed -E 's/ v.*$//' | sort -u)"
    [[ "$dependency_names" == \
      $'adler2\nblock-buffer\ncfg-if\ncpufeatures\ncrc32fast\ncrypto-common\ndigest\nflate2\nfloat-view-refloat-configuration-codec\nhybrid-array\nlibc\nmemchr\nminiz_oxide\nquick-xml\nsha2\nsimd-adler32\ntypenum' ]]
    [[ "$(cd backends/core && cargo tree --locked --offline \
      --package float-view-refloat-configuration-codec --edges normal \
      --depth 1 --prefix none | wc -l | tr -d ' ')" == 4 ]]
    ! (cd backends/core && cargo tree --locked --offline \
      --package float-view-refloat-configuration-codec --edges normal) \
      | rg 'libz-sys|cloudflare-zlib|zlib-rs'
    ! rg -n 'unsafe[[:space:]]*(\{|fn|impl|trait)|extern[[:space:]]+"(C|system)"|no_mangle|Java_|cbindgen|bindgen|uniffi|jni|host_callback' \
      "$crate/src" "$crate/tests"
    [[ "$(find "$crate" -mindepth 1 -maxdepth 1 -type d \
      ! -name src ! -name tests -print -quit)" == "" ]]
    [[ -f backends/core/refloat-configuration-codec-apple-ffi/Cargo.toml ]]
    [[ -f backends/core/refloat-configuration-codec-apple-ffi/src/lib.rs ]]
    [[ -f backends/core/refloat-configuration-codec-apple-ffi/tests/abi.c ]]
    [[ -f backends/core/refloat-configuration-codec-android-jni/Cargo.toml ]]
    [[ -f backends/core/refloat-configuration-codec-android-jni/src/lib.rs ]]
    ! rg -n 'CryptoKit|import zlib|XMLParser|inflateInit_|crc32c|ConfigurationByteReader' \
      backends/apple/Sources/FloatViewBackend/protocol/refloat/RefloatConfigurationCodec.swift
    rg -q '^public enum RefloatConfigurationSchemaCodec' \
      backends/apple/Sources/FloatViewBackend/protocol/refloat/RefloatConfigurationCodec.swift
    rg -q '^public enum RefloatConfigurationValueCodec' \
      backends/apple/Sources/FloatViewBackend/protocol/refloat/RefloatConfigurationCodec.swift
    apple_configuration_source='backends/apple/Sources/FloatViewBackend/protocol/refloat/RefloatConfigurationCodec.swift'
    [[ "$(rg -c 'float_view_refloat_configuration_codec_(abi_version|decode_schema|decode_values|encode_values)\(' \
      "$apple_configuration_source")" == 4 ]]
    [[ "$(rg -l '^import CVescRefloatConfigurationCodec$' \
      backends/apple/Sources -g '*.swift')" == "$apple_configuration_source" ]]
    [[ "$(rg -l 'float_view_refloat_configuration_codec_' \
      backends/apple/Sources -g '*.swift')" == "$apple_configuration_source" ]]
    [[ "$(rg -c '\.systemLibrary\(name: "CVescRefloatConfigurationCodec"\)' \
      backends/apple/Package.swift)" == 1 ]]
    [[ "$(rg -c '\.linkedLibrary\("float_view_refloat_configuration_codec_apple_ffi"\)' \
      backends/apple/Package.swift)" == 1 ]]
    [[ "$(rg -c 'Build Rust Refloat configuration codec' \
      backends/apple/ios/float-view.xcodeproj/project.pbxproj)" == 3 ]]
    [[ "$(rg -c 'refloat-configuration-codec/src/records.rs' \
      backends/apple/ios/float-view.xcodeproj/project.pbxproj)" == 1 ]]
    [[ "$(rg -c 'libfloat_view_refloat_configuration_codec_apple_ffi.a' \
      backends/apple/ios/float-view.xcodeproj/project.pbxproj)" == 1 ]]
    [[ "$(rg -c -- '-lfloat_view_refloat_configuration_codec_apple_ffi' \
      backends/apple/ios/float-view.xcodeproj/project.pbxproj)" == 2 ]]
    [[ "$(rg -c 'Sources/CVescRefloatConfigurationCodec' \
      backends/apple/ios/float-view.xcodeproj/project.pbxproj)" == 2 ]]
    rg -q '^object RefloatConfigurationSchemaCodec' \
      backends/android/app/src/main/java/fail/acheron/floatview/protocol/refloat/RefloatConfigurationCodec.kt
    rg -q '^object RefloatConfigurationValueCodec' \
      backends/android/app/src/main/java/fail/acheron/floatview/protocol/refloat/RefloatConfigurationCodec.kt
    android_configuration_source='backends/android/app/src/main/java/fail/acheron/floatview/protocol/refloat/RefloatConfigurationCodec.kt'
    ! rg -n 'Inflater|SAXParserFactory|MessageDigest|ConfigParamsHandler|RawField|ConfigurationByteReader|crc32c|fun signature\(' \
      "$android_configuration_source"
    [[ "$(rg -c 'RustRefloatConfigurationCodecBridge\.(decodeSchema|decodeValues|encodeValues)\(' \
      "$android_configuration_source")" == 3 ]]
    [[ "$(rg -l 'RustRefloatConfigurationCodecBridge\.(decodeSchema|decodeValues|encodeValues)' \
      backends/android/app/src/main -g '*.kt')" == "$android_configuration_source" ]]
    ! rg -n 'refloat-configuration-codec-android-jni|RustRefloatConfigurationCodecBridge|float_view_refloat_configuration_codec_android_jni' \
      backends/apple
    just core-refloat-configuration-codec-test

android-rust-host:
    backends/core/build-android-bridge.sh frame host debug \
      "$PWD/backends/core/target/android/host/debug"
    backends/core/build-android-bridge.sh firmware-identity host debug \
      "$PWD/backends/core/target/android/identity-host/debug"
    backends/core/build-android-bridge.sh custom-configuration host debug \
      "$PWD/backends/core/target/android/custom-configuration-host/debug"
    backends/core/build-android-bridge.sh bms-values host debug \
      "$PWD/backends/core/target/android/bms-values-host/debug"
    backends/core/build-android-bridge.sh controller-data host debug \
      "$PWD/backends/core/target/android/controller-data-host/debug"
    backends/core/build-android-bridge.sh refloat-control host debug \
      "$PWD/backends/core/target/android/refloat-control-host/debug"
    backends/core/build-android-bridge.sh refloat-compatibility host debug \
      "$PWD/backends/core/target/android/refloat-compatibility-host/debug"
    backends/core/build-android-bridge.sh refloat-dynamic-telemetry host debug \
      "$PWD/backends/core/target/android/refloat-dynamic-telemetry-host/debug"
    backends/core/build-android-bridge.sh refloat-legacy-telemetry host debug \
      "$PWD/backends/core/target/android/refloat-legacy-telemetry-host/debug"
    backends/core/build-android-bridge.sh refloat-configuration-codec host debug \
      "$PWD/backends/core/target/android/refloat-configuration-codec-host/debug"

android-rust-codec-test:
    cd backends/android && ./gradlew testDebugUnitTest \
      --tests fail.acheron.floatview.protocol.RustVescFrameBridgeTest \
      --tests fail.acheron.floatview.protocol.RustVescFirmwareIdentityBridgeTest \
      --tests fail.acheron.floatview.protocol.RustVescCustomConfigurationBridgeTest \
      --tests fail.acheron.floatview.protocol.RustVescBmsValuesBridgeTest \
      --tests fail.acheron.floatview.protocol.RustVescControllerDataBridgeTest \
      --tests fail.acheron.floatview.protocol.RustRefloatControlBridgeTest \
      --tests fail.acheron.floatview.protocol.RustRefloatCompatibilityBridgeTest \
      --tests fail.acheron.floatview.protocol.RustRefloatDynamicTelemetryBridgeTest \
      --tests fail.acheron.floatview.protocol.RustRefloatLegacyTelemetryBridgeTest \
      --tests fail.acheron.floatview.protocol.refloat.RefloatConfigurationCodecTest \
      --tests fail.acheron.floatview.protocol.RefloatControlCodecTest \
      --tests fail.acheron.floatview.protocol.VescProtocolTest

android-rust-packaging:
    #!/usr/bin/env bash
    set -euo pipefail
    (cd backends/android && ./gradlew clean assembleDebug assembleRelease)
    expected="$(for library in \
      libfloat_view_vesc_frame_android_jni.so \
      libfloat_view_vesc_firmware_identity_android_jni.so \
      libfloat_view_vesc_custom_configuration_android_jni.so \
      libfloat_view_vesc_bms_values_android_jni.so \
      libfloat_view_vesc_controller_data_android_jni.so \
      libfloat_view_refloat_control_android_jni.so \
      libfloat_view_refloat_compatibility_android_jni.so \
      libfloat_view_refloat_dynamic_telemetry_android_jni.so \
      libfloat_view_refloat_legacy_telemetry_android_jni.so \
      libfloat_view_refloat_configuration_codec_android_jni.so; do
        printf '%s\n' \
          "lib/arm64-v8a/$library" \
          "lib/armeabi-v7a/$library" \
          "lib/x86/$library" \
          "lib/x86_64/$library"
      done | sort)"
    for apk in \
      backends/android/app/build/outputs/apk/debug/app-debug.apk \
      backends/android/app/build/outputs/apk/release/app-release-unsigned.apk; do
      [[ -f "$apk" ]] || { echo "Missing Android package: $apk" >&2; exit 1; }
      unzip -tq "$apk" >/dev/null
      actual="$(unzip -Z1 "$apk" \
        | rg '(^|/)libfloat_view_(vesc_(frame|firmware_identity|custom_configuration|bms_values|controller_data)|refloat_(control|compatibility|dynamic_telemetry|legacy_telemetry|configuration_codec))_android_jni\.so$' \
        | sort)"
      [[ "$actual" == "$expected" ]] || {
        echo "Unexpected Rust JNI library set in $apk:" >&2
        printf '%s\n' "$actual" >&2
        exit 1
      }
    done

android-rust-audit: android-rust-host
    #!/usr/bin/env bash
    set -euo pipefail
    [[ "$(cd backends/core && cargo tree --locked --offline \
      --package float-view-vesc-frame --edges normal --prefix none | wc -l | tr -d ' ')" == 1 ]]
    [[ "$(cd backends/core && cargo tree --locked --offline \
      --package float-view-vesc-firmware-identity --edges normal --prefix none | wc -l | tr -d ' ')" == 1 ]]
    [[ "$(cd backends/core && cargo tree --locked --offline \
      --package float-view-vesc-frame-apple-ffi --edges normal --depth 1 --prefix none | wc -l | tr -d ' ')" == 3 ]]
    [[ "$(cd backends/core && cargo tree --locked --offline \
      --package float-view-vesc-firmware-identity-android-jni --edges normal --depth 1 --prefix none | wc -l | tr -d ' ')" == 3 ]]
    [[ "$(cd backends/core && cargo tree --locked --offline \
      --package float-view-vesc-custom-configuration-android-jni --edges normal --depth 1 --prefix none | wc -l | tr -d ' ')" == 3 ]]
    [[ "$(cd backends/core && cargo tree --locked --offline \
      --package float-view-vesc-bms-values-android-jni --edges normal --depth 1 --prefix none | wc -l | tr -d ' ')" == 3 ]]
    [[ "$(cd backends/core && cargo tree --locked --offline \
      --package float-view-vesc-controller-data-android-jni --edges normal --depth 1 --prefix none | wc -l | tr -d ' ')" == 3 ]]
    [[ "$(cd backends/core && cargo tree --locked --offline \
      --package float-view-refloat-control-android-jni --edges normal --depth 1 --prefix none | wc -l | tr -d ' ')" == 3 ]]
    [[ "$(cd backends/core && cargo tree --locked --offline \
      --package float-view-refloat-compatibility-android-jni --edges normal --depth 1 --prefix none | wc -l | tr -d ' ')" == 3 ]]
    [[ "$(cd backends/core && cargo tree --locked --offline \
      --package float-view-refloat-dynamic-telemetry-android-jni --edges normal --depth 1 --prefix none | wc -l | tr -d ' ')" == 3 ]]
    [[ "$(cd backends/core && cargo tree --locked --offline \
      --package float-view-refloat-legacy-telemetry-android-jni --edges normal --depth 1 --prefix none | wc -l | tr -d ' ')" == 3 ]]
    [[ "$(cd backends/core && cargo tree --locked --offline \
      --package float-view-refloat-configuration-codec-android-jni --edges normal --depth 1 --prefix none | wc -l | tr -d ' ')" == 3 ]]
    [[ "$(cd backends/core && cargo tree --locked --offline \
      --package float-view-android-jni-support --edges normal --depth 1 --prefix none | wc -l | tr -d ' ')" == 2 ]]
    for manifest in \
      backends/core/vesc-frame-android-jni/Cargo.toml \
      backends/core/vesc-firmware-identity-android-jni/Cargo.toml \
      backends/core/vesc-custom-configuration-android-jni/Cargo.toml \
      backends/core/vesc-bms-values-android-jni/Cargo.toml \
      backends/core/vesc-controller-data-android-jni/Cargo.toml \
      backends/core/refloat-control-android-jni/Cargo.toml \
      backends/core/refloat-compatibility-android-jni/Cargo.toml \
      backends/core/refloat-dynamic-telemetry-android-jni/Cargo.toml \
      backends/core/refloat-legacy-telemetry-android-jni/Cargo.toml \
      backends/core/refloat-configuration-codec-android-jni/Cargo.toml; do
      rg -q '^float-view-android-jni-support = \{ path = "\.\./android-jni-support" \}$' "$manifest"
      ! rg -n '^jni-sys[[:space:]]*=' "$manifest"
    done
    [[ "$(rg -l '^jni-sys = "=0\.4\.1"$' backends/core -g Cargo.toml)" == \
      'backends/core/android-jni-support/Cargo.toml' ]]
    ! rg -n '^jni[[:space:]]*=' backends/core -g Cargo.toml
    [[ "$(rg -c 'rustAndroidJniSupportInputs' backends/android/app/build.gradle.kts)" == 2 ]]
    ! rg -n 'jni_sys|JNINativeInterface|functions\.(ExceptionCheck|GetArrayLength|IsSameObject|GetByteArrayRegion|SetByteArrayRegion|SetIntArrayRegion)|AssertUnwindSafe|catch_unwind|unsafe fn environment|fn (references_alias|any_alias|copy_bytes_from_java|copy_bytes_to_java|write_ints|write_metadata|write_length|write_scan_fields)' \
      backends/core/*-android-jni/src/lib.rs
    rg -q 'functions\.ExceptionCheck' backends/core/android-jni-support/src/lib.rs
    rg -q 'functions\.GetArrayLength' backends/core/android-jni-support/src/lib.rs
    rg -q 'functions\.IsSameObject' backends/core/android-jni-support/src/lib.rs
    rg -q 'functions\.GetByteArrayRegion' backends/core/android-jni-support/src/lib.rs
    rg -q 'functions\.SetByteArrayRegion' backends/core/android-jni-support/src/lib.rs
    rg -q 'functions\.SetIntArrayRegion' backends/core/android-jni-support/src/lib.rs
    rg -q 'catch_unwind' backends/core/android-jni-support/src/lib.rs
    ! rg -n 'unsafe\(no_mangle\)|extern[[:space:]]+"system"|Java_' \
      backends/core/android-jni-support
    rust_abi="$(sed -n 's/^pub const ABI_VERSION: jint = \([0-9][0-9]*\);$/\1/p' \
      backends/core/vesc-frame-android-jni/src/lib.rs)"
    kotlin_abi="$(sed -n 's/^[[:space:]]*private const val ABI_VERSION = \([0-9][0-9]*\)$/\1/p' \
      backends/android/app/src/main/java/fail/acheron/floatview/protocol/vesc/VescFrame.kt)"
    [[ -n "$rust_abi" && "$rust_abi" == "$kotlin_abi" ]] || {
      echo "Android VESC JNI ABI mismatch: Rust=$rust_abi Kotlin=$kotlin_abi" >&2
      exit 1
    }
    identity_rust_abi="$(sed -n 's/^pub const RECORD_VERSION: u32 = \([0-9][0-9]*\);$/\1/p' \
      backends/core/vesc-firmware-identity/src/lib.rs)"
    rg -q '^pub const ABI_VERSION: jint = RECORD_VERSION as jint;$' \
      backends/core/vesc-firmware-identity-android-jni/src/lib.rs
    identity_kotlin_abi="$(sed -n 's/^internal const val VESC_FIRMWARE_IDENTITY_ABI_VERSION = \([0-9][0-9]*\)$/\1/p' \
      backends/android/app/src/main/java/fail/acheron/floatview/protocol/vesc/RustVescFirmwareIdentityBridge.kt)"
    [[ -n "$identity_rust_abi" && "$identity_rust_abi" == "$identity_kotlin_abi" ]] || {
      echo "Android firmware-identity JNI ABI mismatch: Rust=$identity_rust_abi Kotlin=$identity_kotlin_abi" >&2
      exit 1
    }
    configuration_rust_abi="$(sed -n 's/^pub const ABI_VERSION: jint = \([0-9][0-9]*\);$/\1/p' \
      backends/core/vesc-custom-configuration-android-jni/src/lib.rs)"
    configuration_kotlin_abi="$(sed -n 's/^internal const val VESC_CUSTOM_CONFIGURATION_ABI_VERSION = \([0-9][0-9]*\)$/\1/p' \
      backends/android/app/src/main/java/fail/acheron/floatview/protocol/vesc/RustVescCustomConfigurationBridge.kt)"
    [[ -n "$configuration_rust_abi" && "$configuration_rust_abi" == "$configuration_kotlin_abi" ]] || {
      echo "Android custom-configuration JNI ABI mismatch: Rust=$configuration_rust_abi Kotlin=$configuration_kotlin_abi" >&2
      exit 1
    }
    bms_rust_abi="$(sed -n 's/^pub const ABI_VERSION: jint = \([0-9][0-9]*\);$/\1/p' \
      backends/core/vesc-bms-values-android-jni/src/lib.rs)"
    bms_kotlin_abi="$(sed -n 's/^internal const val VESC_BMS_VALUES_ABI_VERSION = \([0-9][0-9]*\)$/\1/p' \
      backends/android/app/src/main/java/fail/acheron/floatview/protocol/vesc/RustVescBmsValuesBridge.kt)"
    [[ -n "$bms_rust_abi" && "$bms_rust_abi" == "$bms_kotlin_abi" ]] || {
      echo "Android BMS-values JNI ABI mismatch: Rust=$bms_rust_abi Kotlin=$bms_kotlin_abi" >&2
      exit 1
    }
    controller_data_rust_abi="$(sed -n 's/^pub const ABI_VERSION: jint = \([0-9][0-9]*\);$/\1/p' \
      backends/core/vesc-controller-data-android-jni/src/lib.rs)"
    controller_data_kotlin_abi="$(sed -n 's/^internal const val VESC_CONTROLLER_DATA_ABI_VERSION = \([0-9][0-9]*\)$/\1/p' \
      backends/android/app/src/main/java/fail/acheron/floatview/protocol/vesc/RustVescControllerDataBridge.kt)"
    [[ -n "$controller_data_rust_abi" && "$controller_data_rust_abi" == "$controller_data_kotlin_abi" ]] || {
      echo "Android controller-data JNI ABI mismatch: Rust=$controller_data_rust_abi Kotlin=$controller_data_kotlin_abi" >&2
      exit 1
    }
    refloat_rust_abi="$(sed -n 's/^pub const ABI_VERSION: jint = \([0-9][0-9]*\);$/\1/p' \
      backends/core/refloat-control-android-jni/src/lib.rs)"
    refloat_kotlin_abi="$(sed -n 's/^internal const val REFLOAT_CONTROL_ABI_VERSION = \([0-9][0-9]*\)$/\1/p' \
      backends/android/app/src/main/java/fail/acheron/floatview/protocol/vesc/RustRefloatControlBridge.kt)"
    [[ -n "$refloat_rust_abi" && "$refloat_rust_abi" == "$refloat_kotlin_abi" ]] || {
      echo "Android Refloat-control JNI ABI mismatch: Rust=$refloat_rust_abi Kotlin=$refloat_kotlin_abi" >&2
      exit 1
    }
    compatibility_rust_abi="$(sed -n 's/^pub const RECORD_VERSION: u8 = \([0-9][0-9]*\);$/\1/p' \
      backends/core/refloat-compatibility/src/lib.rs)"
    rg -q '^pub const ABI_VERSION: jint = RECORD_VERSION as jint;$' \
      backends/core/refloat-compatibility-android-jni/src/lib.rs
    compatibility_kotlin_abi="$(sed -n 's/^internal const val REFLOAT_COMPATIBILITY_ABI_VERSION = \([0-9][0-9]*\)$/\1/p' \
      backends/android/app/src/main/java/fail/acheron/floatview/protocol/RustRefloatCompatibilityBridge.kt)"
    [[ -n "$compatibility_rust_abi" && "$compatibility_rust_abi" == "$compatibility_kotlin_abi" ]] || {
      echo "Android Refloat compatibility JNI ABI mismatch: Rust=$compatibility_rust_abi Kotlin=$compatibility_kotlin_abi" >&2
      exit 1
    }
    dynamic_telemetry_rust_abi="$(sed -n 's/^pub const RECORD_VERSION: u8 = \([0-9][0-9]*\);$/\1/p' \
      backends/core/refloat-dynamic-telemetry/src/lib.rs)"
    rg -q '^pub const ABI_VERSION: jint = RECORD_VERSION as jint;$' \
      backends/core/refloat-dynamic-telemetry-android-jni/src/lib.rs
    dynamic_telemetry_kotlin_abi="$(sed -n 's/^internal const val REFLOAT_DYNAMIC_TELEMETRY_ABI_VERSION = \([0-9][0-9]*\)$/\1/p' \
      backends/android/app/src/main/java/fail/acheron/floatview/protocol/RustRefloatDynamicTelemetryBridge.kt)"
    [[ -n "$dynamic_telemetry_rust_abi" && "$dynamic_telemetry_rust_abi" == "$dynamic_telemetry_kotlin_abi" ]] || {
      echo "Android Refloat dynamic-telemetry JNI ABI mismatch: Rust=$dynamic_telemetry_rust_abi Kotlin=$dynamic_telemetry_kotlin_abi" >&2
      exit 1
    }
    legacy_rust_abi="$(sed -n 's/^pub const RECORD_VERSION: u32 = \([0-9][0-9]*\);$/\1/p' \
      backends/core/refloat-legacy-telemetry/src/lib.rs)"
    rg -q '^pub const ABI_VERSION: jint = RECORD_VERSION as jint;$' \
      backends/core/refloat-legacy-telemetry-android-jni/src/lib.rs
    legacy_kotlin_abi="$(sed -n 's/^internal const val REFLOAT_LEGACY_TELEMETRY_ABI_VERSION = \([0-9][0-9]*\)$/\1/p' \
      backends/android/app/src/main/java/fail/acheron/floatview/protocol/RustRefloatLegacyTelemetryBridge.kt)"
    [[ -n "$legacy_rust_abi" && "$legacy_rust_abi" == "$legacy_kotlin_abi" ]] || {
      echo "Android Refloat legacy-telemetry JNI ABI mismatch: Rust=$legacy_rust_abi Kotlin=$legacy_kotlin_abi" >&2
      exit 1
    }
    refloat_configuration_rust_abi="$(sed -n 's/^pub const ABI_VERSION: jint = \([0-9][0-9]*\);$/\1/p' \
      backends/core/refloat-configuration-codec-android-jni/src/lib.rs)"
    refloat_configuration_kotlin_abi="$(sed -n 's/^internal const val REFLOAT_CONFIGURATION_CODEC_ABI_VERSION = \([0-9][0-9]*\)$/\1/p' \
      backends/android/app/src/main/java/fail/acheron/floatview/protocol/refloat/RefloatConfigurationCodec.kt)"
    [[ -n "$refloat_configuration_rust_abi" && "$refloat_configuration_rust_abi" == "$refloat_configuration_kotlin_abi" ]] || {
      echo "Android Refloat configuration-codec JNI ABI mismatch: Rust=$refloat_configuration_rust_abi Kotlin=$refloat_configuration_kotlin_abi" >&2
      exit 1
    }
    ! git ls-files | rg '\.(a|so|dylib|xcframework)$'
    android_main='backends/android/app/src/main/java/fail/acheron/floatview'
    codec_source="$android_main/protocol/vesc/VescFrame.kt"
    bridge_source="$android_main/protocol/vesc/RustVescFrameBridge.kt"
    ! rg -n -e 'object VescCrc' -e 'VescCrc\.checksum' -e '0x1021' \
      -e 'fun checksum\(' -e 'VescFrameReference' -e 'CheckpointKotlinVesc' \
      -e 'PreCutoverKotlinStreamReference' -e 'buffer\.first\(\)' \
      -e 'headerSize' -e 'frameSize[[:space:]]*=' -e 'setOf\(2, 3\)' \
      -e 'buffer\[[12]\]\.toInt\(\)' \
      -e 'payload\.size <= 255' -e 'header \+ payload' \
      -e 'frame\.last\(\)\.toInt\(\) != 3' \
      -e 'frame\.copyOfRange\(headerSize' \
      backends/android/app/src/main backends/android/app/src/test -g '*.kt'
    [[ "$(rg -l 'object VescFrameCodec' "$android_main" -g '*.kt')" == "$codec_source" ]]
    [[ "$(rg -l 'class VescFrameStreamDecoder' "$android_main" -g '*.kt')" == "$codec_source" ]]
    [[ "$(rg -c 'RustVescFrameBridge\.encode\(' "$codec_source")" == 1 ]]
    [[ "$(rg -c 'RustVescFrameBridge\.decodeExact\(' "$codec_source")" == 1 ]]
    [[ "$(rg -c 'RustVescFrameBridge\.scanNext\(' "$codec_source")" == 1 ]]
    [[ "$(rg -l 'RustVescFrameBridge\.(encode|decodeExact|scanNext)' "$android_main" -g '*.kt')" == "$codec_source" ]]
    [[ "$(rg -l 'external fun (encode|decodeExact|scanNext)' \
      "$bridge_source")" == \
      "$bridge_source" ]]
    identity_source="$android_main/protocol/vesc/VescFirmwareIdentityDecoder.kt"
    identity_bridge_source="$android_main/protocol/vesc/RustVescFirmwareIdentityBridge.kt"
    ! rg -n -e 'terminatedBytes' -e 'extendedFirmwareName' -e 'uuidStart' \
      -e 'commonEnd' -e 'firstOrNull \{ payload\[it\]' -e 'tailLength' \
      "$identity_source"
    [[ "$(rg -c 'RustVescFirmwareIdentityBridge\.decode\(' "$identity_source")" == 1 ]]
    [[ "$(rg -l 'RustVescFirmwareIdentityBridge\.decode\(' "$android_main" -g '*.kt')" == \
      "$identity_source" ]]
    [[ "$(rg -l 'external fun (abiVersion|decode)' "$identity_bridge_source")" == \
      "$identity_bridge_source" ]]
    configuration_source="$android_main/protocol/vesc/VescCustomConfigurationCodec.kt"
    configuration_bridge_source="$android_main/protocol/vesc/RustVescCustomConfigurationBridge.kt"
    ! rg -n -e 'positiveI32' -e 'nonnegativeI32' -e 'private fun i32' \
      -e 'payload\.firstOrNull' -e 'payload\.copyOfRange\(10' \
      -e 'bytes\.size > MAXIMUM_CHUNK_BYTES' \
      "$configuration_source"
    [[ "$(rg -c 'RustVescCustomConfigurationBridge\.encode\(' "$configuration_source")" == 1 ]]
    [[ "$(rg -c 'RustVescCustomConfigurationBridge\.decode\(' "$configuration_source")" == 1 ]]
    [[ "$(rg -l 'RustVescCustomConfigurationBridge\.(encode|decode)' "$android_main" -g '*.kt')" == \
      "$configuration_source" ]]
    [[ "$(rg -l 'external fun (abiVersion|encode|decode)' "$configuration_bridge_source")" == \
      "$configuration_bridge_source" ]]
    bms_source="$android_main/protocol/vesc/VescBmsValuesDecoder.kt"
    bms_protocol_source="$android_main/protocol/vesc/VescProtocolService.kt"
    bms_bridge_source="$android_main/protocol/vesc/RustVescBmsValuesBridge.kt"
    ! rg -n -e 'cellCount[[:space:]]*= reader' -e 'temperatureCount[[:space:]]*= reader' \
      -e 'terminatedUtf8' -e 'maxContentBytes' -e 'lifetimeChargeAmpHours[[:space:]]*= reader' \
      -e 'reportedCanId[[:space:]]*= reader' \
      "$bms_protocol_source"
    [[ "$(rg -c 'VescBmsValuesDecoder\.decode\(' "$bms_protocol_source")" == 1 ]]
    [[ "$(rg -c 'RustVescBmsValuesBridge\.decode\(' "$bms_source")" == 1 ]]
    [[ "$(rg -l 'RustVescBmsValuesBridge\.decode\(' "$android_main" -g '*.kt')" == \
      "$bms_source" ]]
    [[ "$(rg -l 'external fun (abiVersion|decode)' "$bms_bridge_source")" == \
      "$bms_bridge_source" ]]
    controller_data_source="$android_main/protocol/vesc/VescControllerDataDecoder.kt"
    controller_data_protocol_source="$android_main/protocol/vesc/VescProtocolService.kt"
    controller_data_bridge_source="$android_main/protocol/vesc/RustVescControllerDataBridge.kt"
    ! rg -n -e 'private class ByteReader' -e 'float32Auto' -e 'batterySeriesCellOffsets' \
      -e 'Math\.scalb' -e 'VescFault\.fromCode' \
      "$controller_data_protocol_source"
    [[ "$(rg -c 'VescControllerDataDecoder\.realtime\(' "$controller_data_protocol_source")" == 1 ]]
    [[ "$(rg -c 'VescControllerDataDecoder\.currentLimits\(' "$controller_data_protocol_source")" == 1 ]]
    [[ "$(rg -c 'RustVescControllerDataBridge\.decode\(' "$controller_data_source")" == 1 ]]
    [[ "$(rg -l 'RustVescControllerDataBridge\.decode\(' "$android_main" -g '*.kt')" == \
      "$controller_data_source" ]]
    [[ "$(rg -l 'external fun (abiVersion|decode)' "$controller_data_bridge_source")" == \
      "$controller_data_bridge_source" ]]
    refloat_source="$android_main/protocol/refloat/RefloatControlCodec.kt"
    refloat_bridge_source="$android_main/protocol/vesc/RustRefloatControlBridge.kt"
    ! rg -n -e 'RefloatWire\.request\(' -e 'val mask = when' \
      -e 'payload\[2\].*RefloatCommand' -e 'payload\[3\].*0xfc' \
      "$refloat_source"
    [[ "$(rg -c 'RustRefloatControlBridge\.encode\(' "$refloat_source")" == 1 ]]
    [[ "$(rg -c 'RustRefloatControlBridge\.decodeLighting\(' "$refloat_source")" == 1 ]]
    [[ "$(rg -l 'RustRefloatControlBridge\.(encode|decodeLighting)' "$android_main" -g '*.kt')" == \
      "$refloat_source" ]]
    [[ "$(rg -l 'external fun (abiVersion|encode|decodeLighting)' "$refloat_bridge_source")" == \
      "$refloat_bridge_source" ]]
    compatibility_source="$android_main/protocol/refloat/RefloatCompatibility.kt"
    compatibility_bridge_source="$android_main/protocol/RustRefloatCompatibilityBridge.kt"
    [[ ! -f "$android_main/protocol/refloat/RefloatInfo.kt" ]]
    [[ "$(rg -c 'RustRefloatCompatibilityBridge\.infoRequest\(' "$compatibility_source")" == 1 ]]
    [[ "$(rg -c 'RustRefloatCompatibilityBridge\.decodeInfo\(' "$compatibility_source")" == 1 ]]
    [[ "$(rg -c 'RustRefloatCompatibilityBridge\.admitLayout\(' "$compatibility_source")" == 1 ]]
    [[ "$(rg -l 'RustRefloatCompatibilityBridge\.(infoRequest|decodeInfo|admitLayout)' \
      "$android_main" -g '*.kt')" == "$compatibility_source" ]]
    [[ "$(rg -l 'external fun (abiVersion|infoRequest|decodeInfo|admitLayout)' \
      "$compatibility_bridge_source")" == "$compatibility_bridge_source" ]]
    ! rg -n -e 'INFO_V2_(MAJOR|MINOR|PATCH)_OFFSET' -e 'requiredFields' \
      -e 'requiredRuntimeFields' -e 'supportsDynamicLayout' -e 'Set\(required' \
      -e 'sourceVersion' -e 'static (val|let) entries' \
      backends/android/app/src/main -g '*.kt'
    ! rg -n -e 'INFO_PROTOCOL_VERSION' -e 'INFO_FLAGS' \
      backends/android/app/src/main -g '*.kt'
    ! rg -n -e 'COMPATIBILITY_ENTRIES' -e 'fn (serialize|encode).*compatibility' \
      backends/core/refloat-compatibility-android-jni/src/lib.rs
    ! rg -n 'RustRefloatCompatibilityBridge|refloat-compatibility-android-jni|float_view_refloat_compatibility_android_jni' \
      backends/apple
    legacy_source="$android_main/protocol/refloat/RefloatLegacy.kt"
    legacy_bridge_source="$android_main/protocol/RustRefloatLegacyTelemetryBridge.kt"
    ! rg -n -e 'signed16' -e 'unsigned32' -e 'decodeFloat32' \
      -e 'legacySetpointAdjustment' -e 'legacySemanticState' \
      "$legacy_source"
    [[ "$(rg -c 'RustRefloatLegacyTelemetryBridge\.decode\(' "$legacy_source")" == 1 ]]
    [[ "$(rg -l 'RustRefloatLegacyTelemetryBridge\.decode\(' "$android_main" -g '*.kt')" == \
      "$legacy_source" ]]
    [[ "$(rg -l 'external fun (abiVersion|decode)' "$legacy_bridge_source")" == \
      "$legacy_bridge_source" ]]
    refloat_configuration_source="$android_main/protocol/refloat/RefloatConfigurationCodec.kt"
    ! rg -n -e 'Inflater' -e 'SAXParserFactory' -e 'MessageDigest' \
      -e 'ConfigParamsHandler' -e 'ConfigurationByteReader' -e 'crc32c' \
      -e 'fun signature\(' "$refloat_configuration_source"
    [[ "$(rg -c 'RustRefloatConfigurationCodecBridge\.decodeSchema\(' "$refloat_configuration_source")" == 1 ]]
    [[ "$(rg -c 'RustRefloatConfigurationCodecBridge\.decodeValues\(' "$refloat_configuration_source")" == 1 ]]
    [[ "$(rg -c 'RustRefloatConfigurationCodecBridge\.encodeValues\(' "$refloat_configuration_source")" == 1 ]]
    [[ "$(rg -l 'external fun (abiVersion|decodeSchema|decodeValues|encodeValues)' \
      "$refloat_configuration_source")" == "$refloat_configuration_source" ]]
    if ANDROID_NDK_HOME=/private/tmp/float-view-missing-ndk \
      backends/core/build-android-bridge.sh frame \
      android arm64-v8a debug /private/tmp/float-view-unreachable-output >/dev/null 2>&1; then
      echo 'Android Rust helper accepted a missing pinned NDK.' >&2
      exit 1
    fi
    if ANDROID_NDK_HOME=/private/tmp/float-view-missing-ndk \
      backends/core/build-android-bridge.sh custom-configuration \
      android arm64-v8a debug /private/tmp/float-view-unreachable-custom-configuration-output >/dev/null 2>&1; then
      echo 'Android Rust custom-configuration helper accepted a missing pinned NDK.' >&2
      exit 1
    fi
    if ANDROID_NDK_HOME=/private/tmp/float-view-missing-ndk \
      backends/core/build-android-bridge.sh firmware-identity \
      android arm64-v8a debug /private/tmp/float-view-unreachable-identity-output >/dev/null 2>&1; then
      echo 'Android Rust identity helper accepted a missing pinned NDK.' >&2
      exit 1
    fi
    if ANDROID_NDK_HOME=/private/tmp/float-view-missing-ndk \
      backends/core/build-android-bridge.sh bms-values \
      android arm64-v8a debug /private/tmp/float-view-unreachable-bms-values-output >/dev/null 2>&1; then
      echo 'Android Rust BMS-values helper accepted a missing pinned NDK.' >&2
      exit 1
    fi
    if ANDROID_NDK_HOME=/private/tmp/float-view-missing-ndk \
      backends/core/build-android-bridge.sh controller-data \
      android arm64-v8a debug /private/tmp/float-view-unreachable-controller-data-output >/dev/null 2>&1; then
      echo 'Android Rust controller-data helper accepted a missing pinned NDK.' >&2
      exit 1
    fi
    if ANDROID_NDK_HOME=/private/tmp/float-view-missing-ndk \
      backends/core/build-android-bridge.sh refloat-control \
      android arm64-v8a debug /private/tmp/float-view-unreachable-refloat-control-output >/dev/null 2>&1; then
      echo 'Android Rust Refloat-control helper accepted a missing pinned NDK.' >&2
      exit 1
    fi

setup:
    cd frontend && npm ci
    cd conformance && npm ci
    pio pkg install -d tools/vesc-test-device
    mkdir -p vendor
    [ -e "vendor/bldc" ] || git clone https://github.com/vedderb/bldc "vendor/bldc"
    [ -e "vendor/refloat" ] || git clone https://github.com/lukash/refloat "vendor/refloat"
    [ -e "vendor/vesc_tool" ] || git clone https://github.com/vedderb/vesc_tool "vendor/vesc_tool"
    [ -e "vendor/vesc_bms_fw" ] || git clone https://github.com/vedderb/vesc_bms_fw "vendor/vesc_bms_fw"
    [ -e "vendor/vesc_express" ] || git clone https://github.com/vedderb/vesc_express "vendor/vesc_express"
    [ -e "vendor/float-view" ] || git clone https://github.com/acheronfail/float-view "vendor/float-view"

lint:
    just core-lint
    cd frontend && npm run format:check
    cd conformance && npm run format:check
    swiftformat --config .swiftformat --lint backends/apple

fmt:
    cd frontend && npm run format:write
    cd conformance && npm run format:write
    swiftformat --config .swiftformat backends/apple
    cd backends/core && cargo fmt

dev platform="mock" target="":
    cd conformance && node --strip-types src/dev.ts "{{ platform }}" "{{ target }}"

list platform:
    #!/usr/bin/env bash
    set -euo pipefail
    case "{{ platform }}" in
      ios)
        xcrun xctrace list devices |
          awk '
            /^== Devices ==$/ { physical=1; next }
            /^== Simulators ==$/ { physical=0 }
            physical && /iPhone|iPad|iPod/ { print }
          '
        ;;
      android) adb devices -l;;
      *) echo 'Usage: just list android|ios' >&2; exit 2 ;;
    esac

emulate platform: build
    #!/usr/bin/env bash
    set -euo pipefail
    case "{{ platform }}" in
      android)
        emulator -avd Float_View_Conformance -no-window -no-audio
        ;;
      ios)
        simulator="${FLOAT_VIEW_IOS_SIMULATOR:-iPhone 17 Pro}"
        xcrun simctl boot "$simulator" >/dev/null 2>&1 || true
        xcrun simctl bootstatus "$simulator" -b
        ;;
      *) echo 'Usage: just emulate android|ios' >&2; exit 2 ;;
    esac

build:
    cd frontend && npm run build
    cp frontend/build/index.html backends/apple/macos/Resources/index.html
    mkdir -p backends/apple/macos/Resources/sql
    cp backends/sql/catalog.sql backends/apple/macos/Resources/sql/catalog.sql
    cp backends/sql/session.sql backends/apple/macos/Resources/sql/session.sql

run platform target="": build
    #!/usr/bin/env bash
    set -euo pipefail
    case "{{ platform }}" in
      android)
        (cd backends/android && ./gradlew assembleDebug)
        serial="{{ target }}"
        adb_args=()
        [[ -z "$serial" ]] || adb_args=(-s "$serial")
        adb "${adb_args[@]}" install -r backends/android/app/build/outputs/apk/debug/app-debug.apk
        adb "${adb_args[@]}" logcat -c || true
        adb "${adb_args[@]}" shell am start -W \
          -n fail.acheron.floatview/fail.acheron.floatview.ui.MainActivity
        process_id="$(adb "${adb_args[@]}" shell pidof -s fail.acheron.floatview | tr -d '\r')"
        [[ "$process_id" =~ ^[0-9]+$ ]] || { echo 'Float View did not start' >&2; exit 1; }
        echo "Streaming Float View logs from Android process $process_id; press Ctrl-C to stop."
        exec adb "${adb_args[@]}" logcat --pid="$process_id" -v color
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
            --terminate-existing --console fail.acheron.float-view
        fi
        ;;
      macos)
        frontend_html="$PWD/frontend/build/index.html"
        just apple-rust-bridges
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
      apple)
        just apple-rust-bridges
        (cd backends/apple && swift test --scratch-path /tmp/float-view-swift-build)
        ;;
      android) (cd backends/android && ./gradlew testDebugUnitTest) ;;
      core) just core-test ;;
      fixture) sh tools/vesc-test-device/test/run.sh ;;
      all)
        just test frontend
        just test conformance
        just test apple
        just test android
        just test core
        just test fixture
        ;;
      *) echo 'Usage: just test frontend|conformance|apple|android|core|fixture|all' >&2; exit 2 ;;
    esac

conformance target:
    cd conformance && npm run {{ target }}

conformance-physical platform fixture_port target="": build
    #!/usr/bin/env bash
    set -euo pipefail
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
    [[ -f backends/core/application-runtime-apple-ffi/Cargo.toml ]]
    [[ -f backends/apple/Sources/CFloatViewApplicationRuntime/include/CFloatViewApplicationRuntime.h ]]
    [[ -f backends/apple/Sources/CFloatViewApplicationRuntime/module.modulemap ]]
    [[ "$(find backends/core -mindepth 2 -maxdepth 2 -path '*-apple-ffi/Cargo.toml' -print | sort)" ==       'backends/core/application-runtime-apple-ffi/Cargo.toml' ]]
    [[ -z "$(find backends/apple/Sources -mindepth 1 -maxdepth 1 -type d -name 'CVesc*' -print -quit)" ]]
    [[ "$(rg -c '\.systemLibrary\(name: "CFloatViewApplicationRuntime"\)' backends/apple/Package.swift)" == 1 ]]
    [[ "$(rg -c '\.linkedLibrary\("float_view_application_runtime_apple_ffi"\)' backends/apple/Package.swift)" == 1 ]]
    ! rg -n 'CVesc|float_view_(vesc|refloat)_.*apple_ffi|apple-ffi-support'       backends/apple backends/core/Cargo.toml backends/core/Cargo.lock
    [[ "$(rg -c 'Build unified Rust application runtime'       backends/apple/ios/float-view.xcodeproj/project.pbxproj)" == 3 ]]
    [[ "$(rg -c -- '-lfloat_view_application_runtime_apple_ffi'       backends/apple/ios/float-view.xcodeproj/project.pbxproj)" == 2 ]]
    [[ "$(rg -c 'Sources/CFloatViewApplicationRuntime'       backends/apple/ios/float-view.xcodeproj/project.pbxproj)" == 2 ]]
    ! rg -n 'VescProtocolPort|VescProtocolService|import CVesc' backends/apple -g '*.swift'
    for file in AdaptiveLogging AuxiliaryNodeDiscovery ControllerDiscovery       RefloatConfigurationService RefloatMutationCoordinator SessionHistoryReducer       TelemetryPolling VescRequestScheduler; do
      [[ ! -e "backends/apple/Sources/FloatViewBackend/application/$file.swift" ]]
    done
    ! git ls-files | rg '\\.(a|so|dylib|xcframework)$'
    rust_lines="$(find backends/core -path '*/target' -prune -o -path '*/tests' -prune       -o -type f -name '*.rs' -print0 | xargs -0 wc -l | tail -1 | awk '{print $1}')"
    apple_lines="$(find backends/apple -path '*/.build' -prune -o -path '*/Tests' -prune       -o -type f -name '*.swift' -print0 | xargs -0 wc -l | tail -1 | awk '{print $1}')"
    (( rust_lines + apple_lines < 51994 ))
    [[ -z "$(git diff --name-only -- backends/android/app/src/main)" ]]
    git diff --check

verify: build
    just core-lint
    just apple-packaging
    just test all
    just android-rust-packaging
    cd backends/apple && swift build --scratch-path /tmp/float-view-swift-build --product FloatViewMac
    xcodebuild -project backends/apple/ios/float-view.xcodeproj -scheme float-view \
      -sdk iphonesimulator -destination 'generic/platform=iOS Simulator' \
      -derivedDataPath /tmp/float-view-ios CODE_SIGNING_ALLOWED=NO build
    cd conformance && npm run macos && npm run ios && npm run android
    just audit
