#!/usr/bin/env python3

"""Probe the T0 firmware over its native USB Serial/JTAG interface."""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import time

import serial


def find_port(usb_serial: str) -> str:
    result = subprocess.run(
        ["ioreg", "-r", "-n", "USB JTAG/serial debug unit", "-l", "-w0"],
        check=True,
        capture_output=True,
        text=True,
    )
    if f'"USB Serial Number" = "{usb_serial}"' not in result.stdout:
        raise RuntimeError(f"ESP USB serial {usb_serial} was not found")
    match = re.search(r'"IOTTYSuffix" = "([^"]+)"', result.stdout)
    if match is None:
        raise RuntimeError(f"ESP USB serial {usb_serial} has no tty suffix")
    port = f"/dev/cu.usbmodem{match.group(1)}"
    if not os.path.exists(port):
        raise RuntimeError(f"resolved ESP serial port does not exist: {port}")
    return port


def arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--usb-serial",
        default=os.environ.get("FLOAT_VIEW_ESP_USB_SERIAL"),
        required=os.environ.get("FLOAT_VIEW_ESP_USB_SERIAL") is None,
    )
    parser.add_argument("--duration-seconds", type=int, default=10)
    parsed = parser.parse_args()
    if not 2 <= parsed.duration_seconds <= 660:
        parser.error("--duration-seconds must be between 2 and 660")
    return parsed


def main() -> int:
    args = arguments()
    port = find_port(args.usb_serial)
    observed: set[str] = set()
    events: list[dict[str, object]] = []
    parse_errors = 0
    buffered = bytearray()
    deadline = time.monotonic() + args.duration_seconds
    next_command = time.monotonic()

    with serial.Serial(
        port=port,
        baudrate=115200,
        timeout=0.1,
        write_timeout=1,
        exclusive=True,
    ) as connection:
        connection.reset_input_buffer()
        while time.monotonic() < deadline:
            now = time.monotonic()
            if now >= next_command:
                connection.write(b'{"protocolVersion":0,"kind":"ping"}\n')
                connection.write(b'{"protocolVersion":0,"kind":"getState"}\n')
                next_command = now + 0.5

            chunk = connection.read(connection.in_waiting or 1)
            if not chunk:
                continue
            buffered.extend(chunk)
            while b"\n" in buffered:
                raw_line, _, remainder = buffered.partition(b"\n")
                buffered = bytearray(remainder)
                line = raw_line.strip()
                if not line.startswith(b"{"):
                    continue
                try:
                    event = json.loads(line)
                    if (
                        event.get("protocolVersion") != 0
                        or not isinstance(event.get("kind"), str)
                    ):
                        parse_errors += 1
                        continue
                    observed.add(event["kind"])
                    events.append(event)
                except (UnicodeDecodeError, json.JSONDecodeError):
                    parse_errors += 1

    required = {"heartbeat", "pong", "state"}
    missing = sorted(required - observed)
    advertising_observed = any(event.get("advertising") is True for event in events)
    result = {
        "schemaVersion": 1,
        "kind": "vesc-test-device-t0-usb",
        "port": port,
        "durationSeconds": args.duration_seconds,
        "eventCount": len(events),
        "eventKinds": sorted(observed),
        "parseErrors": parse_errors,
        "advertisingObserved": advertising_observed,
        "passed": not missing and parse_errors == 0 and advertising_observed,
        "missing": missing,
    }
    print(json.dumps(result, indent=2))
    return 0 if result["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
