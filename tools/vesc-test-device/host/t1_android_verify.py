#!/usr/bin/env python3

"""Orchestrate the T1 fixture and production Android BLE adapter acceptance."""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import time

import serial

from probe import find_port
from t1_verify import Harness, NOTIFICATION_HEX, WRITE_HEX


def arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--usb-serial",
        default=os.environ.get("FLOAT_VIEW_ESP_USB_SERIAL"),
        required=os.environ.get("FLOAT_VIEW_ESP_USB_SERIAL") is None,
    )
    parser.add_argument(
        "--android-serial",
        default=os.environ.get("ANDROID_SERIAL"),
        required=os.environ.get("ANDROID_SERIAL") is None,
    )
    return parser.parse_args()


def adb(serial_number: str, *arguments: str, check: bool = True):
    return subprocess.run(
        ["adb", "-s", serial_number, *arguments],
        check=check,
        capture_output=True,
        text=True,
    )


def main() -> int:
    args = arguments()
    port = find_port(args.usb_serial)
    package = "fail.acheron.floatview.bluetooth.test"
    test = (
        "fail.acheron.floatview.bluetooth.T1VescTestDeviceTest"
        "#productionAdapterRoundTripsExactBytes"
    )

    api_level = int(adb(args.android_serial, "shell", "getprop", "ro.build.version.sdk").stdout)
    if api_level >= 31:
        adb(
            args.android_serial,
            "shell",
            "pm",
            "grant",
            package,
            "android.permission.BLUETOOTH_SCAN",
        )
        adb(
            args.android_serial,
            "shell",
            "pm",
            "grant",
            package,
            "android.permission.BLUETOOTH_CONNECT",
        )
    else:
        # Some Android 10 vendor stacks gate scan delivery on both location
        # grants even though Float View's policy exposes fine location only.
        for permission in (
            "android.permission.ACCESS_COARSE_LOCATION",
            "android.permission.ACCESS_FINE_LOCATION",
        ):
            adb(
                args.android_serial,
                "shell",
                "pm",
                "grant",
                package,
                permission,
            )
        # An instrumentation-only APK has no foreground Activity. Samsung's
        # Android 10 stack otherwise leaves coarse location in foreground-only
        # app-op mode and suppresses all scan results.
        for operation in ("COARSE_LOCATION", "FINE_LOCATION"):
            adb(
                args.android_serial,
                "shell",
                "appops",
                "set",
                "--uid",
                package,
                operation,
                "allow",
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
            harness.command("configureGattProfile", profile="canonicalVescUart")
        )
        harness.wait_command(harness.command("startAdvertising"))

        instrumentation = subprocess.Popen(
            [
                "adb",
                "-s",
                args.android_serial,
                "shell",
                "am",
                "instrument",
                "-w",
                "-r",
                "-e",
                "class",
                test,
                f"{package}/androidx.test.runner.AndroidJUnitRunner",
            ],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        deadline = time.monotonic() + 35
        next_keepalive = time.monotonic() + 1
        saw_write = False
        notification_sequence: int | None = None

        while time.monotonic() < deadline and instrumentation.poll() is None:
            for event in harness.pump():
                if (
                    event.get("kind") == "gattWrite"
                    and event.get("hex") == WRITE_HEX
                    and event.get("accepted") is True
                    and notification_sequence is None
                ):
                    saw_write = True
                    notification_sequence = harness.command(
                        "notify", hex=NOTIFICATION_HEX
                    )
            if time.monotonic() >= next_keepalive:
                harness.command("getState")
                next_keepalive = time.monotonic() + 1
            time.sleep(0.01)

        if instrumentation.poll() is None:
            instrumentation.terminate()
            raise RuntimeError("Android instrumentation timed out")
        stdout, stderr = instrumentation.communicate(timeout=3)
        if instrumentation.returncode != 0 or "OK (1 test)" not in stdout:
            raise RuntimeError(
                f"Android instrumentation failed ({instrumentation.returncode}): "
                f"{stdout}\n{stderr}"
            )
        if not saw_write or notification_sequence is None:
            raise RuntimeError("Android client did not write the expected bytes")
        harness.wait_command(notification_sequence)
        disconnected = next(
            (
                event
                for event in reversed(harness.events)
                if event.get("kind") == "disconnected"
            ),
            None,
        )
        if disconnected is None:
            disconnected = harness.wait_for(
                lambda event: event.get("kind") == "disconnected",
                5,
                "Android client disconnect",
            )
        if disconnected.get("kind") != "disconnected":
            raise RuntimeError(f"unexpected disconnect event: {disconnected}")

    result = {
        "schemaVersion": 1,
        "kind": "vesc-test-device-t1-android",
        "passed": True,
        "apiLevel": api_level,
        "gattWriteHex": WRITE_HEX,
        "notificationHex": NOTIFICATION_HEX,
        "instrumentation": "OK (1 test)",
    }
    print(json.dumps(result, indent=2))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"T1 Android verification failed: {error}", file=sys.stderr)
        raise SystemExit(1)
