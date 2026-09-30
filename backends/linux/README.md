# Linux desktop host

The Linux executable owns one shared Rust service and loopback server at
`127.0.0.1:11013`. GTK4/WebKitGTK displays the same embedded frontend as the
other hosts; BlueR executes BlueZ operations. Product policy, SQLite, protocol,
write segmentation, reconnect, archives and public projection stay in core.

## Build and run

The initial source-build baseline is x86_64 TUXEDO OS 24.04 (Ubuntu Noble),
GTK 4.14.5 and WebKitGTK 2.52.6. Use the repository's Rust 1.93.0 toolchain
with rustfmt/clippy, Node 24 and `just`. Other distributions need equivalent
system development libraries; no distribution-independent package is provided.

```sh
sudo apt-get install libgtk-4-dev libwebkitgtk-6.0-dev libdbus-1-dev pkg-config
rustup toolchain install 1.93.0 --profile minimal --component clippy --component rustfmt
npm ci --prefix frontend
npm ci --prefix conformance
cargo fetch --manifest-path backends/linux/Cargo.toml --locked
just build-linux
just run linux
just dev linux
just test linux
cargo clippy --manifest-path backends/linux/Cargo.toml --locked --offline --all-targets -- -D warnings
```

Builds never install prerequisites. The frontend must be built before Cargo:
`build.rs` embeds its HTML into ignored Cargo output and fails if absent. The
Linux workspace has a separate lockfile so native GUI dependencies do not enter
Apple/Android builds. When shared dependencies change, verify both consumer
graphs. There is no additional application FFI family.

A desktop session with its session bus and an accessible system BlueZ daemon is
required. Run as your ordinary user. Float View does not install policy rules,
power the adapter, change discoverability or unpair other devices. If Bluetooth
is unavailable, configure it using your desktop's Bluetooth settings. Adapter
selection prefers the first powered adapter in sorted name order, retaining
that adapter during a connection. Linux cannot provide a universal OS permission
prompt or settings link; unavailable actions are omitted. Authorization failures
remain denied until the adapter is power-cycled or the daemon/app restarts;
successful adapter property reads do not prove discovery authorization.

Live location acquisition is unavailable in this version. Imported location
history remains readable. Minimize keeps recording active; close or quit stops
the process. System suspend interrupts capture and uses the shared reconnect
policy on resume. There is no tray service, autostart or unattended deployment.
`just dev linux` retains the native event loop and file dialogs and shows a small
host window while Vite serves the frontend in the browser.

## Storage and bounds

Owned data lives in `$XDG_DATA_HOME/float-view`, or
`$HOME/.local/share/float-view` when XDG is absent or not absolute. The root must
be a private directory (0700). A second instance fails without resetting data,
changing ports or stopping an existing process. Corruption and I/O failures do
not cause format resets. Native export presentation retains the shared lease
until dismissal or copy completion; copy failure is reported in the native UI.

The OS executor admits 256 ordinary facts, 16 cleanup facts, 16 BLE effects,
256 timers and eight UI effects, with latest-state conflation on UI saturation.
BLE notifications use an acquired bounded socket with a 512-byte read buffer;
ingress saturation fails the connection. Discovery examines at most 256 devices
per enumeration and 16 adapters. GATT discovery examines at most 64 services
and 64 characteristics per service. Setup and writes have eight-second native
operation deadlines inside the shared ten-second transport deadline. Payloads
use a conservative 20-byte ceiling. Writes are submitted serially; a completed
no-response OS write is local acceptance, never a remote ACK. Shared core owns
the 128-chunk/65,536-byte logical queue. Timers use Linux suspend-aware boottime.
OS shutdown allows ten seconds, followed by bounded shared listener/service
shutdown. Logs contain only operation-level diagnostics.

## Verification

```sh
cd conformance
npm run linux
npm run physical -- linux /dev/ttyACM0
```

Software conformance builds the explicit `software-bluetooth` feature and
passes `--software-conformance`, using the existing authenticated fixture relay
and isolated temporary storage. The ordinary build ignores fixture environment
variables. The test-only HTTP client replaces a handwritten relay transport;
all VESC simulation and assertions remain in `conformance/`.

Physical conformance builds the ordinary host and uses real BlueZ and the
serial-controlled ESP32. Follow the fixture safety/restore procedures in the
conformance README. Software results do not establish radio behavior, desktop
suspend, native file dialogs or WebView visual correctness. Record those
checkpoints separately and identify missing coverage.
