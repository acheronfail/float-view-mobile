# Build and run Float View

Run commands from the repository root. [just](https://just.systems/) is the command
runner: `just` (or `just --list`) lists the recipes in [justfile](justfile).

## Setup

Install Node.js 24 (with npm), `just`, and Rust via `rustup`. Native builds use
the version pinned in [rust-toolchain.toml](rust-toolchain.toml).

```sh
npm --prefix frontend ci
npm --prefix conformance ci
cargo fetch --locked --manifest-path backends/core/Cargo.toml
```

Native Rust builds run offline, so fetch dependencies before your first build
and after Rust dependency changes. Install the tools for your platform:

- **macOS / iOS:** a Mac with Xcode 26+ and its command-line tools selected.
  For iOS, install the simulator runtime in Xcode and add the Rust target:
  `rustup target add aarch64-apple-ios` for a phone,
  `aarch64-apple-ios-sim` for an Apple Silicon simulator, or `x86_64-apple-ios`
  for an Intel simulator. See the [Apple guide](backends/apple/README.md) for signing.
- **Android:** JDK 17 or 21, the Android SDK/NDK and Rust Android targets.
  Follow [Android SDK setup](backends/android/README.md#local-sdk-and-verification)
  and put SDK `platform-tools` on `PATH` for `adb`.

`just setup` is the full contributor setup: it also installs Playwright browsers
and ESP32 firmware dependencies and clones upstream sources into `vendor/`.
It requires PlatformIO (`pio`); the smaller setup above is enough to get started.

## Build and run

Each `just run` command builds the frontend and native app before launching it.
Replace the example IDs below with those shown by `just list`.

```sh
just run macos

just list ios
just run ios SIMULATOR_ID
APPLE_TEAM_ID=YOUR_TEAM_ID just run ios DEVICE_ID

just list android
just run android DEVICE_SERIAL
```

For an iPhone, sign into Xcode Settings → Accounts, enable Developer Mode on the
phone and trust the Mac. `APPLE_TEAM_ID` selects your signing team without editing
the project; you can also export it in your shell. See the
[Apple signing notes](backends/apple/README.md) for details.

`just run ios` without an ID only builds for the simulator; it does not launch.
`just run android` can omit the serial when only one device or emulator is connected.
Start simulators/emulators through Xcode or Android Studio before selecting them.

For an AltStore IPA, run `just ipa` on a Mac; it writes the IPA to the current
directory. `just build` builds only the shared frontend and copies it into the
Apple resources.

## Develop with live reload

```sh
just dev                         # Browser UI with mock data
just dev macos                   # Browser UI with the native macOS backend
```

Open `http://127.0.0.1:11002` if the browser does not open automatically.
`just dev ios SIMULATOR_ID` and `just dev android DEVICE_SERIAL` use mobile backends.
Use `just run` to try the app in its native window or WebView.

## Verification

Run checks for what you changed, such as `just test frontend`, `just test core`,
`just test apple` or `just test android`. `just verify` runs the comprehensive
software checks and requires both Apple and Android tooling.
See the [development reference](docs/development.md#verification) for focused tests,
browser build prerequisites and physical checkpoints.

For more detail, see [repository layout and development tools](docs/development.md),
[release tooling](docs/releases.md) and [licensing](docs/licensing.md).
Project-owned contributions use AGPL-3.0-or-later.
