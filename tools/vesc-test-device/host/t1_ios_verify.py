#!/usr/bin/env python3

"""Build and orchestrate the T1 fixture with the production iOS BLE adapter."""

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
from t1_verify import Harness, NOTIFICATION_HEX, WRITE_HEX


def arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--usb-serial",
        default=os.environ.get("FLOAT_VIEW_ESP_USB_SERIAL"),
        required=os.environ.get("FLOAT_VIEW_ESP_USB_SERIAL") is None,
    )
    parser.add_argument(
        "--ios-device-id",
        default=os.environ.get("FLOAT_VIEW_IOS_DEVICE_ID"),
        required=os.environ.get("FLOAT_VIEW_IOS_DEVICE_ID") is None,
        help="physical iOS device identifier accepted by xcodebuild",
    )
    return parser.parse_args()


def xcode_command(
    repository_root: Path,
    device_id: str,
    derived_data: Path,
    action: str,
) -> list[str]:
    return [
        "xcodebuild",
        "-project",
        str(repository_root / "ios/float-view.xcodeproj"),
        "-scheme",
        "float-view-hardware",
        "-configuration",
        "Debug",
        "-destination",
        f"platform=iOS,id={device_id}",
        "-derivedDataPath",
        str(derived_data),
        "-allowProvisioningUpdates",
        "-only-testing:"
        "float-view-hardware-tests/T1VescTestDeviceTests/"
        "testProductionAdapterRoundTripsExactBytes",
        action,
    ]


def main() -> int:
    args = arguments()
    port = find_port(args.usb_serial)
    repository_root = Path(__file__).resolve().parents[3]

    with tempfile.TemporaryDirectory(prefix="float-view-t1-ios-") as temporary:
        derived_data = Path(temporary) / "DerivedData"
        build = subprocess.run(
            xcode_command(
                repository_root,
                args.ios_device_id,
                derived_data,
                "build-for-testing",
            ),
            cwd=repository_root,
            capture_output=True,
            text=True,
        )
        if build.returncode != 0:
            raise RuntimeError(
                f"iOS hardware-test build failed ({build.returncode}):\n"
                f"{build.stdout}\n{build.stderr}"
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

            test = subprocess.Popen(
                xcode_command(
                    repository_root,
                    args.ios_device_id,
                    derived_data,
                    "test-without-building",
                ),
                cwd=repository_root,
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
                bufsize=0,
            )
            assert test.stdout is not None
            xcode_output = bytearray()
            deadline = time.monotonic() + 75
            next_keepalive = time.monotonic() + 1
            notification_sequence: int | None = None
            saw_write = False

            while time.monotonic() < deadline and test.poll() is None:
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

                readable, _, _ = select.select([test.stdout], [], [], 0)
                if readable:
                    xcode_output.extend(os.read(test.stdout.fileno(), 16_384))
                    if len(xcode_output) > 2_000_000:
                        del xcode_output[:-1_000_000]

                if time.monotonic() >= next_keepalive:
                    harness.command("getState")
                    next_keepalive = time.monotonic() + 1
                time.sleep(0.01)

            if test.poll() is None:
                test.terminate()
                raise RuntimeError("physical iOS hardware test timed out")
            remainder = test.stdout.read()
            if remainder:
                xcode_output.extend(remainder)
            output = xcode_output.decode(errors="replace")
            succeeded = (
                "** TEST SUCCEEDED **" in output
                or "** TEST EXECUTE SUCCEEDED **" in output
            )
            if test.returncode != 0 or not succeeded:
                raise RuntimeError(
                    f"physical iOS hardware test failed ({test.returncode}):\n"
                    f"{output[-20_000:]}"
                )
            if not saw_write or notification_sequence is None:
                raise RuntimeError("iOS client did not write the expected bytes")
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
                    "iOS client disconnect",
                )

    result = {
        "schemaVersion": 1,
        "kind": "vesc-test-device-t1-ios",
        "passed": True,
        "gattWriteHex": WRITE_HEX,
        "notificationHex": NOTIFICATION_HEX,
        "xcodeResult": "TEST SUCCEEDED",
    }
    print(json.dumps(result, indent=2))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"T1 iOS verification failed: {error}", file=sys.stderr)
        raise SystemExit(1)
