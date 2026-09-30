# Proposal

## Why

Linux cannot currently run Float View as a native desktop application with real Bluetooth. The existing process-owned Rust service makes a Linux host possible without another implementation of product behavior, persistence, protocol policy, or the frontend API.

## What Changes

- Add a Linux desktop host under `backends/linux/`, using the existing SvelteKit artifact, shared Rust service, and authenticated loopback server on `127.0.0.1:11013`.
- Execute Linux Bluetooth discovery, GATT operations, timers, permission observations, desktop lifecycle, file presentation, and appearance through bounded native adapters.
- Use a Rust GTK4/WebKitGTK shell and BlueZ adapter, with direct safe Rust composition rather than a third application binding family.
- Preserve recording when minimized or when the WebView is replaced. Close/quit terminates the desktop process cleanly; suspend/resume is an interruption, not a guarantee of continuous capture.
- Support existing product workflows and archive formats. Report live location capture as unavailable in this first version, while retaining imported location history and review.
- Add Linux build/run/browser-development entry points and canonical software plus scoped physical conformance integration.
- Update repository ownership documentation and audits to include Linux. Replace exhaustive three-platform assumptions in current tooling; remove any temporary integration probes before completion. No existing backend is superseded and no compatibility layer or migration is added.

Non-goals: a headless product/service, remote or LAN access, systemd/autostart/tray behavior, live Linux location acquisition, new firmware support, distribution packages, sandboxed Flatpak/Snap delivery, and new product UI or frontend platform branches.

## Capabilities

### New Capabilities

- `linux-desktop-backend`: Desktop host behavior, OS capabilities, interruptions, storage, builds, and acceptance for Linux.

### Modified Capabilities

- `platform-hosts`: Include the Linux ownership root and allocate its stable loopback port.
- `native-development-loop`: Add Linux to the existing platform-selected browser development command.

## Impact

Affected areas are `backends/linux/`, private platform/permission integration in `backends/core/`, `conformance/`, the root `justfile`, ownership audits, and current repository documentation/configuration. Linux-specific dependencies are GTK4/WebKitGTK Rust bindings and BlueZ bindings; exact compatible versions and system prerequisites are pinned during implementation.

The public frontend contract and persistence formats remain unchanged. Private Rust platform enumerations gain Linux; existing Apple/Android values and behavior remain stable. Linux consumes the same shared service and BLE transport policy, with OS capability differences expressed through existing canonical states. If implementation discovers a necessary public contract change, revise this plan and update the frontend client/mock, all hosts, and canonical conformance together before adopting it.
