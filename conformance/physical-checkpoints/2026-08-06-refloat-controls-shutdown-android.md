# Refloat controls and controller shutdown physical checkpoint

Date: 2026-08-06  
Native target: Android (`SM_G965F`, ADB serial `1cfb7634120d7ece`)  
BLE fixture: ESP32 `/dev/cu.usbmodem2101`  
Supported controller: retained THOR controller on the Thor301/VBMS32 board

## ESP32 physical conformance

The Android production backend was exercised over real BLE against the
canonical ESP32 byte bridge. The direct and CAN-routed Refloat control
checkpoints passed, including lighting, package disable/enable, flywheel
enable, and flywheel disable after active flywheel telemetry normalized the
package run state as riding. The fixture recorded only allowlisted control or
telemetry requests and no motor-motion request.

The direct shutdown, CAN-routed shutdown, and unsupported-hardware no-op
checkpoints passed together. Each shutdown route emitted exactly one eligible
shutdown intent; simulated controller loss returned the app home without
automatic reconnect, while the no-op case remained connected and cleared its
pending intent.

The unfiltered Android matrix passed 16 of 17 automated scenarios. The final
background-location assertion observed authorized background capability but no
persisted location row during its ten-second dwell. The connected Samsung had
no cached location and is a known device-specific GPS issue. The user accepted
this physical checkpoint as passed with that explicit waiver; it does not
replace or weaken the Refloat-control or shutdown assertions.

## Supported-controller shutdown and recovery

The first real-controller attempt proved that the exact command powered off
the controller and attached BLE transport, but Android reported GATT status 8
after the original three-second shutdown-intent window and entered
`Reconnecting`. The retained device log correlated the delayed BLE loss with
that incorrect classification.

The shutdown-intent window was increased to a bounded 30 seconds on both native
backends to cover BLE link-supervision delay. On the repeated Android checkpoint:

- the stationary supported THOR controller powered off after confirmation;
- Float View remained connected for the few seconds before Android surfaced
  transport loss, then exited LiveView to the home screen;
- Float View did not enter automatic reconnect;
- manual controller power-on restored discovery and normal connection.

This confirms actual supported-hardware shutdown, intentional-loss handling,
and manual power recovery without claiming that command transmission itself is
power-off acknowledgement.
