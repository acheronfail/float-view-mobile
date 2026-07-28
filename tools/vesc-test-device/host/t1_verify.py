#!/usr/bin/env python3

"""Run the T1 USB/CoreBluetooth transport acceptance on macOS."""

from __future__ import annotations

import argparse
import json
import os
import select
import subprocess
import sys
import tempfile
import time
from pathlib import Path

import serial

from probe import find_port

PROTOCOL_VERSION = 1
WRITE_HEX = "102030405060"
NOTIFICATION_HEX = "deadbeef0102"


class Harness:
    def __init__(self, connection: serial.Serial):
        self.connection = connection
        self.sequence = 0
        self.buffer = bytearray()
        self.events: list[dict[str, object]] = []

    def command(self, kind: str, **payload: object) -> int:
        self.sequence += 1
        command = {
            "protocolVersion": PROTOCOL_VERSION,
            "sequence": self.sequence,
            "kind": kind,
            **payload,
        }
        encoded = json.dumps(command, separators=(",", ":")).encode() + b"\n"
        if len(encoded) > 768:
            raise RuntimeError("host command exceeded firmware line limit")
        written = self.connection.write(encoded)
        if written != len(encoded):
            raise RuntimeError(
                f"short USB write for {kind}: {written}/{len(encoded)}"
            )
        return self.sequence

    def pump(self) -> list[dict[str, object]]:
        chunk = self.connection.read(self.connection.in_waiting or 1)
        if chunk:
            self.buffer.extend(chunk)
        received: list[dict[str, object]] = []
        while b"\n" in self.buffer:
            raw, _, remainder = self.buffer.partition(b"\n")
            self.buffer = bytearray(remainder)
            line = raw.strip()
            if not line.startswith(b"{"):
                continue
            event = json.loads(line)
            if event.get("protocolVersion") != PROTOCOL_VERSION:
                continue
            received.append(event)
            self.events.append(event)
        return received

    def wait_for(self, predicate, timeout: float, description: str):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            for event in self.pump():
                if predicate(event):
                    return event
            time.sleep(0.01)
        raise RuntimeError(f"timed out waiting for {description}")

    def wait_command(self, sequence: int, timeout: float = 3):
        event = next(
            (
                item
                for item in reversed(self.events)
                if item.get("commandSequence") == sequence
                and item.get("kind") in {"commandCompleted", "commandFailed"}
            ),
            None,
        )
        if event is None:
            event = self.wait_for(
                lambda item: item.get("commandSequence") == sequence
                and item.get("kind") in {"commandCompleted", "commandFailed"},
                timeout,
                f"completion of command {sequence}",
            )
        if event["kind"] == "commandFailed":
            raise RuntimeError(
                f"command {sequence} failed: {event.get('code')} "
                f"{event.get('message')}"
            )
        return event


def arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--usb-serial",
        default=os.environ.get("FLOAT_VIEW_ESP_USB_SERIAL"),
        required=os.environ.get("FLOAT_VIEW_ESP_USB_SERIAL") is None,
    )
    return parser.parse_args()


def main() -> int:
    args = arguments()
    port = find_port(args.usb_serial)
    host_dir = Path(__file__).resolve().parent
    repository_root = host_dir.parents[2]
    client_events: list[dict[str, object]] = []

    with tempfile.TemporaryDirectory(prefix="float-view-t1-") as temporary:
        client_path = Path(temporary) / "t1-production-client"
        subprocess.run(
            [
                "swiftc",
                str(repository_root / "apple/Sources/FloatViewCore/Domain.swift"),
                str(
                    repository_root
                    / "apple/Sources/FloatViewBluetooth/CoreBluetoothAdapter.swift"
                ),
                str(host_dir / "t1_production_client.swift"),
                "-o",
                str(client_path),
            ],
            check=True,
        )

        with serial.Serial(
            port=port,
            baudrate=115200,
            timeout=0.05,
            write_timeout=1,
            exclusive=True,
        ) as connection:
            connection.reset_input_buffer()
            harness = Harness(connection)
            harness.wait_command(harness.command("hello"))
            harness.wait_command(
                harness.command(
                    "configureGattProfile", profile="canonicalVescUart"
                )
            )
            harness.wait_command(harness.command("startAdvertising"))

            client = subprocess.Popen(
                [str(client_path)],
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                bufsize=0,
            )
            assert client.stdout is not None
            client_buffer = bytearray()
            next_keepalive = time.monotonic() + 1
            deadline = time.monotonic() + 30
            notification_sequence: int | None = None
            disconnect_sequence: int | None = None
            saw_write = False
            saw_notification = False
            client_passed = False

            while time.monotonic() < deadline and not client_passed:
                for event in harness.pump():
                    if (
                        event.get("kind") == "gattWrite"
                        and event.get("hex") == WRITE_HEX
                        and event.get("accepted") is True
                    ):
                        saw_write = True
                        notification_sequence = harness.command(
                            "notify", hex=NOTIFICATION_HEX
                        )

                readable, _, _ = select.select([client.stdout], [], [], 0)
                if readable:
                    chunk = os.read(client.stdout.fileno(), 4096)
                    client_buffer.extend(chunk)
                    while b"\n" in client_buffer:
                        raw, _, remainder = client_buffer.partition(b"\n")
                        client_buffer = bytearray(remainder)
                        if not raw.strip():
                            continue
                        event = json.loads(raw)
                        client_events.append(event)
                        if event.get("kind") == "notificationReceived":
                            if event.get("hex") != NOTIFICATION_HEX:
                                raise RuntimeError(
                                    "client notification bytes differ"
                                )
                            saw_notification = True
                            disconnect_sequence = harness.command("disconnect")
                            harness.wait_command(disconnect_sequence)
                        elif event.get("kind") == "passed":
                            client_passed = True
                        elif event.get("kind") == "failed":
                            raise RuntimeError(
                                f"CoreBluetooth client failed: {event}"
                            )

                now = time.monotonic()
                if now >= next_keepalive:
                    harness.command("getState")
                    next_keepalive = now + 1
                if client.poll() not in (None, 0):
                    remaining = client.stdout.read()
                    stderr = client.stderr.read() if client.stderr else b""
                    terminal_usb = [
                        event
                        for event in harness.events
                        if event.get("commandKind") == "disconnect"
                        or event.get("kind") in {"disconnected", "fault"}
                    ]
                    raise RuntimeError(
                        "CoreBluetooth client exited; "
                        f"events={client_events}, remaining={remaining!r}, "
                        f"stderr={stderr!r}, terminalUsb={terminal_usb}"
                    )
                time.sleep(0.01)

            if not client_passed:
                client.terminate()
                stderr = client.stderr.read() if client.stderr else b""
                raise RuntimeError(f"CoreBluetooth client timed out: {stderr}")
            client.wait(timeout=3)
            if not saw_write or not saw_notification:
                raise RuntimeError("byte-for-byte BLE round trip was incomplete")
            if notification_sequence is None or disconnect_sequence is None:
                raise RuntimeError("terminal BLE commands were not issued")
            harness.wait_command(notification_sequence)
            harness.wait_command(disconnect_sequence)

            state_sequence = harness.command("getState")
            state_event = harness.wait_command(state_sequence)
            if state_event.get("connected") is not False:
                raise RuntimeError("fixture remained connected after disconnect")

            harness.wait_command(harness.command("stopAdvertising"))
            harness.wait_command(harness.command("reset"))
            harness.wait_command(
                harness.command(
                    "configureGattProfile", profile="canonicalVescUart"
                )
            )
            harness.wait_command(harness.command("startAdvertising"))

            prior_sequence = harness.sequence

    time.sleep(6)

    with serial.Serial(
        port=port,
        baudrate=115200,
        timeout=0.05,
        write_timeout=1,
        exclusive=True,
    ) as connection:
        connection.reset_input_buffer()
        harness = Harness(connection)
        harness.sequence = prior_sequence
        harness.wait_command(harness.command("hello"))
        state_event = harness.wait_command(harness.command("getState"))
        safe_after_host_loss = (
            state_event.get("configured") is False
            and state_event.get("armed") is False
            and state_event.get("advertising") is False
            and state_event.get("connected") is False
            and state_event.get("notifyPending") is False
        )
        if not safe_after_host_loss:
            raise RuntimeError(f"unsafe state after host timeout: {state_event}")

    result = {
        "schemaVersion": 1,
        "kind": "vesc-test-device-t1-macos",
        "passed": True,
        "gattWriteHex": WRITE_HEX,
        "notificationHex": NOTIFICATION_HEX,
        "clientEventKinds": [event["kind"] for event in client_events],
        "safeAfterHostLoss": True,
    }
    print(json.dumps(result, indent=2))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"T1 verification failed: {error}", file=sys.stderr)
        raise SystemExit(1)
