#!/usr/bin/env python3
"""Read-only BlueZ probe for Express/CAN Refloat schema bootstrap.

Uses distro python3-dbus/python3-gi; no Linux application backend or pairing.
Only firmware, CAN-ID, Refloat INFO and schema-read commands are admitted.
Device addresses, controller UUIDs and raw frames are never printed.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import struct
import sys
import time
import zlib

import dbus
from dbus.mainloop.glib import DBusGMainLoop
from gi.repository import GLib

UART = '6e400001-b5a3-f393-e0a9-e50e24dcca9e'
TX = '6e400002-b5a3-f393-e0a9-e50e24dcca9e'
RX = '6e400003-b5a3-f393-e0a9-e50e24dcca9e'
PROPS = 'org.freedesktop.DBus.Properties'
DBusGMainLoop(set_as_default=True)


def pump_until(predicate, seconds):
    deadline = time.monotonic() + seconds
    context = GLib.MainContext.default()
    while time.monotonic() < deadline:
        context.iteration(False)
        while context.pending():
            context.iteration(False)
        if predicate():
            return True
        time.sleep(0.01)
    return predicate()


def crc16(data):
    crc = 0
    for byte in data:
        crc ^= byte << 8
        for _ in range(8):
            crc = ((crc << 1) ^ (0x1021 if crc & 0x8000 else 0)) & 0xffff
    return crc


def frame(payload):
    if len(payload) > 4096:
        raise ValueError('Request too large')
    header = bytes([2, len(payload)]) if len(payload) < 256 else bytes([3]) + struct.pack('>H', len(payload))
    return header + payload + struct.pack('>H', crc16(payload)) + b'\x03'


class Probe:
    def __init__(self):
        self.bus = dbus.SystemBus()
        self.manager = dbus.Interface(self.bus.get_object('org.bluez', '/'), 'org.freedesktop.DBus.ObjectManager')
        self.buffer = bytearray()
        self.responses = []
        self.device = None
        self.notify = None
        self.subscription = None

    def objects(self):
        return self.manager.GetManagedObjects()

    def scan(self):
        adapters = [path for path, interfaces in self.objects().items()
                    if interfaces.get('org.bluez.Adapter1', {}).get('Powered')]
        if len(adapters) != 1:
            raise ValueError('Expected one powered Bluetooth adapter')
        adapter = dbus.Interface(self.bus.get_object('org.bluez', adapters[0]), 'org.bluez.Adapter1')
        adapter.SetDiscoveryFilter(dbus.Dictionary({'Transport': 'le'}, signature='sv'))
        adapter.StartDiscovery()
        try:
            pump_until(lambda: False, 8)
            return [(path, str(p.get('Name', 'Unnamed UART device')))
                    for path, interfaces in self.objects().items()
                    if (p := interfaces.get('org.bluez.Device1'))
                    and UART in [str(uuid).lower() for uuid in p.get('UUIDs', [])]]
        finally:
            adapter.StopDiscovery()

    def connect(self, path):
        self.device = dbus.Interface(self.bus.get_object('org.bluez', path), 'org.bluez.Device1')
        self.device.Connect(timeout=30)
        props = dbus.Interface(self.bus.get_object('org.bluez', path), PROPS)
        if not pump_until(lambda: bool(props.Get('org.bluez.Device1', 'ServicesResolved')), 15):
            raise TimeoutError('GATT service discovery timed out')
        chars = {str(p.get('UUID', '')).lower(): char_path
                 for char_path, interfaces in self.objects().items()
                 if str(char_path).startswith(str(path) + '/')
                 and (p := interfaces.get('org.bluez.GattCharacteristic1'))}
        if TX not in chars or RX not in chars:
            raise ValueError('UART GATT characteristics are missing')
        self.tx = dbus.Interface(self.bus.get_object('org.bluez', chars[TX]), 'org.bluez.GattCharacteristic1')
        tx_props = dbus.Interface(self.bus.get_object('org.bluez', chars[TX]), PROPS)
        flags = [str(flag) for flag in tx_props.Get('org.bluez.GattCharacteristic1', 'Flags')]
        self.write_type = 'request' if 'write' in flags else 'command'
        print(json.dumps({'write_type': self.write_type}), flush=True)
        self.subscription = self.bus.add_signal_receiver(self.changed, signal_name='PropertiesChanged',
                                                        dbus_interface=PROPS, path=chars[RX])
        self.notify = dbus.Interface(self.bus.get_object('org.bluez', chars[RX]), 'org.bluez.GattCharacteristic1')
        self.notify.StartNotify()

    def changed(self, interface, changed, invalidated):
        if interface != 'org.bluez.GattCharacteristic1' or 'Value' not in changed:
            return
        self.buffer.extend(bytes(changed['Value']))
        if len(self.buffer) > 8192:
            raise ValueError('Notification stream exceeded bound')
        while self.buffer:
            if self.buffer[0] not in (2, 3):
                del self.buffer[0]
                continue
            header = 2 if self.buffer[0] == 2 else 3
            if len(self.buffer) < header:
                break
            length = self.buffer[1] if header == 2 else int.from_bytes(self.buffer[1:3], 'big')
            if length > 4096:
                raise ValueError('Response frame exceeded bound')
            total = header + length + 3
            if len(self.buffer) < total:
                break
            packet = bytes(self.buffer[:total])
            del self.buffer[:total]
            payload = packet[header:header + length]
            if packet[-1] != 3 or int.from_bytes(packet[-3:-1], 'big') != crc16(payload):
                raise ValueError('Invalid response frame')
            if len(self.responses) >= 16:
                raise ValueError('Response queue exceeded bound')
            self.responses.append(payload)

    def request(self, payload, can_id=None, expected=None):
        allowed = payload == b'\x00' or payload == bytes([50, 0, 2, 0, 0]) or payload == bytes([36, 101, 0, 2, 0])
        allowed |= len(payload) == 10 and payload[:2] == bytes([92, 0]) and 0 < int.from_bytes(payload[2:6], 'big') <= 400
        if not allowed:
            raise ValueError('Only read-only probe commands are allowed')
        routed = payload if can_id is None else bytes([34, can_id]) + payload
        encoded = frame(routed)
        self.responses.clear()
        started = time.monotonic()
        for offset in range(0, len(encoded), 20):
            self.tx.WriteValue(dbus.Array(encoded[offset:offset + 20], signature='y'),
                               dbus.Dictionary({'type': self.write_type}, signature='sv'), timeout=5)
        match = expected or (lambda response: response[0] == payload[0])
        if not pump_until(lambda: any(match(response) for response in self.responses), 5):
            raise TimeoutError(f'Read command {payload[0]} timed out')
        reply = next(response for response in self.responses if match(response))
        return reply, round((time.monotonic() - started) * 1000, 1)

    def close(self):
        for effect in (lambda: self.notify.StopNotify() if self.notify else None,
                       lambda: self.subscription.remove() if self.subscription else None,
                       lambda: self.device.Disconnect() if self.device else None):
            try:
                effect()
            except dbus.DBusException:
                pass


def firmware_summary(payload):
    end = payload.index(0, 3)
    start = end + 1
    if len(payload) < start + 16:
        raise ValueError('Truncated firmware identity')
    return {'version': f'{payload[1]}.{payload[2]:02}',
            'hardware': payload[3:end].decode('utf8', errors='replace'),
            'custom_configuration_count': payload[start + 15],
            'test_version': payload[start + 13],
            'release': 'stable' if payload[start + 13] == 0 else 'test'}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--device-name')
    parser.add_argument('--identity-only', action='store_true', help='Stop after firmware and package identity reads')
    parser.add_argument('--can-id', type=int, default=1)
    parser.add_argument('--output', type=Path, default=Path('/tmp/float-view-board-schema.qcompress'))
    args = parser.parse_args()
    if not 0 <= args.can_id <= 253:
        raise ValueError('Invalid CAN ID')
    probe = Probe()
    try:
        devices = probe.scan()
        print(json.dumps({'uart_devices': [name for _, name in devices]}), flush=True)
        if args.device_name is None:
            return
        selected = [path for path, name in devices if name == args.device_name]
        if len(selected) != 1:
            raise ValueError('Expected exactly one matching UART device')
        print('Connecting to Express…', flush=True)
        probe.connect(selected[0])
        print('Connected to UART GATT service', flush=True)
        try:
            reply, ms = probe.request(b'\x00')
        except TimeoutError:
            print('Retrying initial firmware read', flush=True)
            reply, ms = probe.request(b'\x00')
        print(json.dumps({'transport_firmware': firmware_summary(reply), 'response_ms': ms}), flush=True)
        reply, ms = probe.request(b'\x00', args.can_id)
        print(json.dumps({'motor_firmware': firmware_summary(reply), 'response_ms': ms}), flush=True)
        reply, ms = probe.request(bytes([50, 0, 2, 0, 0]), args.can_id)
        if reply != bytes([50, 0, 2, 0, 0, args.can_id]):
            raise ValueError('Controller did not verify the selected CAN ID')
        print(json.dumps({'verified_can_id': args.can_id, 'response_ms': ms}), flush=True)
        reply, ms = probe.request(bytes([36, 101, 0, 2, 0]), args.can_id)
        if len(reply) < 28 or reply[:5] != bytes([36, 101, 0, 2, 0]):
            raise ValueError('Unexpected Refloat INFO reply')
        print(json.dumps({'refloat_version': '.'.join(map(str, reply[25:28])), 'response_ms': ms}), flush=True)
        if args.identity_only:
            return
        schema = bytearray()
        total = None
        times = []
        for _ in range(82):
            offset = len(schema)
            length = 400 if total is None else min(400, total - offset)
            reply, ms = probe.request(bytes([92, 0]) + struct.pack('>II', length, offset), args.can_id,
                                     lambda response: len(response) >= 10 and response[:2] == bytes([92, 0]) and int.from_bytes(response[6:10], 'big') == offset)
            declared = int.from_bytes(reply[2:6], 'big')
            if declared > 32768 or declared == 0 or (total is not None and declared != total):
                raise ValueError('Invalid schema size')
            total = declared
            if len(reply) <= 10 or len(reply) - 10 > 400 or len(schema) + len(reply) - 10 > total:
                raise ValueError('Invalid schema chunk')
            schema.extend(reply[10:])
            times.append(ms)
            if len(schema) == total:
                break
            if len(times) % 10 == 0:
                print(json.dumps({'schema_download_bytes': len(schema), 'total_bytes': total}), flush=True)
        if len(schema) != total:
            raise ValueError('Schema download exceeded page limit')
        fd = os.open(args.output, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
        with os.fdopen(fd, 'wb') as output:
            output.write(schema)
        declared = int.from_bytes(schema[:4], 'big')
        if declared > 393216:
            raise ValueError('Expanded schema exceeds production bound')
        decompressor = zlib.decompressobj()
        xml = decompressor.decompress(schema[4:], declared + 1)
        if len(xml) != declared or not decompressor.eof or decompressor.unused_data or decompressor.unconsumed_tail:
            raise ValueError('Invalid schema compression envelope')
        fingerprint = hashlib.sha256(xml).hexdigest()
        manifest = json.loads((Path(__file__).parent.parent / 'backends/fixtures/refloat-configuration/manifest.json').read_text())
        matches = [release['version'] for release in manifest['releases'] if release['schemaFingerprintSha256'] == fingerprint]
        print(json.dumps({'compressed_bytes': len(schema), 'expanded_bytes': len(xml), 'schema_fingerprint': fingerprint,
                          'audited_matches': matches, 'pages': len(times), 'max_response_ms': max(times),
                          'schema_file': str(args.output)}), flush=True)
    finally:
        probe.close()


if __name__ == '__main__':
    try:
        main()
    except dbus.DBusException as error:
        print('Bluetooth error: ' + error.get_dbus_name(), file=sys.stderr)
        sys.exit(1)
    except (ValueError, TimeoutError, OSError) as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)
