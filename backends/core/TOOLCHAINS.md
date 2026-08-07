# Portable backend toolchains

This file is the toolchain and packaging authority for the portable backend.
`toolchains.toml` is its machine-readable counterpart. Versions change only in
the same review as the lockfile, generated-header provenance, native smoke
matrix, and dependency notices.

## Pinned inputs

- Rust: `1.93.0`, minimal rustup profile, with `rustfmt` and `clippy`. The root
  `rust-toolchain.toml` selects it. `Cargo.lock` is committed using lockfile
  format 4. Setup and CI fetch with `--locked`; builds and tests use
  `--locked`, and verification rejects a lockfile diff.
- C header generation: `cbindgen 0.29.2`, installed with
  `cargo install --locked cbindgen --version 0.29.2`. It is generation-only;
  normal Apple and Android builds consume the checked-in header.
- Android: SDK 36, NDK `28.2.13676358`, JDK 17, and `cargo-ndk 4.1.2`.
  Gradle owns one Cargo invocation for each of `arm64-v8a`, `armeabi-v7a`,
  `x86`, and `x86_64`; these map respectively to
  `aarch64-linux-android`, `armv7-linux-androideabi`,
  `i686-linux-android`, and `x86_64-linux-android`. Minimum API is 29.
- Apple: Xcode 26.3, Swift 6.2.4, macOS 14.0 and iOS 17.0 deployment targets.
  Cargo produces target-specific `staticlib` products. Xcode links those
  products directly with the checked-in header and module map. Build products
  stay in derived data; no XCFramework or other native binary is committed.
- Required targets: native macOS `aarch64-apple-darwin`; iOS device
  `aarch64-apple-ios`; iOS simulator `aarch64-apple-ios-sim` and
  `x86_64-apple-ios`; and the four Android targets listed above. Linux and
  Windows compilation/runtime verification is intentionally deferred to a
  follow-up change.

## Integrity policy

`checksums.sha256` records repository-owned executable/bootstrap artifacts.
Rustup verifies signed channel manifests and component hashes; Android
`sdkmanager` verifies the repository metadata checksums for the exact NDK
revision; Cargo verifies registry package checksums captured in `Cargo.lock`.
Downloaded artifacts that bypass one of those package managers must be added to
`checksums.sha256` before use. Generated-header and packaged-library hashes are
added by their owning implementation tasks and must be checked before host
packaging. Mutable path or git dependency overrides are forbidden.

## Reproducible setup

From the repository root:

```sh
rustup toolchain install 1.93.0 --profile minimal --component clippy,rustfmt
rustup target add --toolchain 1.93.0 \
  aarch64-apple-darwin aarch64-apple-ios aarch64-apple-ios-sim \
  x86_64-apple-ios

sdkmanager "platforms;android-36" "ndk;28.2.13676358"
cargo install --locked cargo-ndk --version 4.1.2
cargo install --locked cbindgen --version 0.29.2
```

Android CI additionally installs the four Android Rust targets.

After the workspace and header exist, setup fetches and validates immutable
inputs without building or generating:

```sh
cargo fetch --manifest-path backends/core/Cargo.toml --locked
shasum -a 256 -c backends/core/checksums.sha256
```

## Validation record

Run these before compilation and retain their output with matrix results:

```sh
rustc -vV
cargo -vV
rustup show active-toolchain
rustup target list --installed
java -version
sdkmanager --list_installed
cargo ndk --version
cbindgen --version
xcodebuild -version
swift --version
shasum -a 256 -c backends/core/checksums.sha256
```

Expected mismatches fail before native packaging and report the expected value,
the observed value, and the corresponding install command above. Normal builds
do not install tools, update dependencies, access the network, or regenerate
the C header.
