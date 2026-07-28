#!/usr/bin/env swift

import CoreBluetooth
import Foundation

private let serviceUUID = CBUUID(string: "6E400001-B5A3-F393-E0A9-E50E24DCCA9E")
private let writeUUID = CBUUID(string: "6E400002-B5A3-F393-E0A9-E50E24DCCA9E")
private let notifyUUID = CBUUID(string: "6E400003-B5A3-F393-E0A9-E50E24DCCA9E")
private let expectedName = "FloatView Test VESC"
private let writeBytes = Data([0x10, 0x20, 0x30, 0x40, 0x50, 0x60])
private let expectedNotification = Data([0xde, 0xad, 0xbe, 0xef, 0x01, 0x02])

private func emit(_ kind: String, _ fields: [String: Any] = [:]) {
    var event = fields
    event["kind"] = kind
    event["schemaVersion"] = 1
    let data = try! JSONSerialization.data(
        withJSONObject: event,
        options: [.sortedKeys]
    )
    print(String(decoding: data, as: UTF8.self))
    fflush(stdout)
}

final class Client: NSObject, CBCentralManagerDelegate, CBPeripheralDelegate {
    private var manager: CBCentralManager!
    private var peripheral: CBPeripheral?
    private var writeCharacteristic: CBCharacteristic?
    private var notifyCharacteristic: CBCharacteristic?
    private var receivedExpectedNotification = false
    private var timeout: Timer?

    override init() {
        super.init()
        manager = CBCentralManager(delegate: self, queue: .main)
        timeout = Timer.scheduledTimer(withTimeInterval: 25, repeats: false) { _ in
            emit("failed", ["code": "timeout"])
            exit(1)
        }
    }

    func centralManagerDidUpdateState(_ central: CBCentralManager) {
        guard central.state == .poweredOn else {
            if central.state == .unsupported || central.state == .unauthorized {
                emit("failed", [
                    "code": "bluetoothUnavailable",
                    "state": central.state.rawValue,
                ])
                exit(1)
            }
            return
        }
        central.scanForPeripherals(withServices: [serviceUUID])
    }

    func centralManager(
        _ central: CBCentralManager,
        didDiscover peripheral: CBPeripheral,
        advertisementData: [String: Any],
        rssi RSSI: NSNumber
    ) {
        let name = advertisementData[CBAdvertisementDataLocalNameKey] as? String
            ?? peripheral.name
        let marker = advertisementData[CBAdvertisementDataManufacturerDataKey]
            as? Data
        guard name == expectedName,
              marker == Data([0xff, 0xff, 0x46, 0x56, 0x54, 0x31]) else {
            return
        }
        central.stopScan()
        self.peripheral = peripheral
        peripheral.delegate = self
        emit("discovered", ["name": name ?? "", "rssi": RSSI.intValue])
        central.connect(peripheral)
    }

    func centralManager(
        _ central: CBCentralManager,
        didConnect peripheral: CBPeripheral
    ) {
        emit("connected")
        peripheral.discoverServices([serviceUUID])
    }

    func peripheral(
        _ peripheral: CBPeripheral,
        didDiscoverServices error: Error?
    ) {
        guard error == nil,
              let service = peripheral.services?.first(where: {
                  $0.uuid == serviceUUID
              }) else {
            emit("failed", ["code": "serviceDiscovery"])
            exit(1)
        }
        peripheral.discoverCharacteristics([writeUUID, notifyUUID], for: service)
    }

    func peripheral(
        _ peripheral: CBPeripheral,
        didDiscoverCharacteristicsFor service: CBService,
        error: Error?
    ) {
        guard error == nil else {
            emit("failed", ["code": "characteristicDiscovery"])
            exit(1)
        }
        writeCharacteristic = service.characteristics?.first {
            $0.uuid == writeUUID
        }
        notifyCharacteristic = service.characteristics?.first {
            $0.uuid == notifyUUID
        }
        guard let writeCharacteristic, let notifyCharacteristic,
              writeCharacteristic.properties.contains(.write),
              writeCharacteristic.properties.contains(.writeWithoutResponse),
              notifyCharacteristic.properties.contains(.notify) else {
            emit("failed", ["code": "characteristicShape"])
            exit(1)
        }
        emit("characteristicsDiscovered")
        peripheral.setNotifyValue(true, for: notifyCharacteristic)
    }

    func peripheral(
        _ peripheral: CBPeripheral,
        didUpdateNotificationStateFor characteristic: CBCharacteristic,
        error: Error?
    ) {
        guard error == nil, characteristic.isNotifying,
              let writeCharacteristic else {
            emit("failed", ["code": "subscription"])
            exit(1)
        }
        emit("subscribed")
        peripheral.writeValue(
            writeBytes,
            for: writeCharacteristic,
            type: .withResponse
        )
    }

    func peripheral(
        _ peripheral: CBPeripheral,
        didWriteValueFor characteristic: CBCharacteristic,
        error: Error?
    ) {
        guard error == nil else {
            emit("failed", ["code": "gattWrite"])
            exit(1)
        }
        emit("writeCompleted", ["hex": writeBytes.map {
            String(format: "%02x", $0)
        }.joined()])
    }

    func peripheral(
        _ peripheral: CBPeripheral,
        didUpdateValueFor characteristic: CBCharacteristic,
        error: Error?
    ) {
        guard error == nil, characteristic.value == expectedNotification else {
            emit("failed", ["code": "notificationBytes"])
            exit(1)
        }
        receivedExpectedNotification = true
        emit("notificationReceived", ["hex": expectedNotification.map {
            String(format: "%02x", $0)
        }.joined()])
    }

    func centralManager(
        _ central: CBCentralManager,
        didDisconnectPeripheral peripheral: CBPeripheral,
        timestamp: CFAbsoluteTime,
        isReconnecting: Bool,
        error: (any Error)?
    ) {
        finishDisconnect()
    }

    func centralManager(
        _ central: CBCentralManager,
        didDisconnectPeripheral peripheral: CBPeripheral,
        error: Error?
    ) {
        finishDisconnect()
    }

    private func finishDisconnect() {
        timeout?.invalidate()
        guard receivedExpectedNotification else {
            emit("failed", ["code": "prematureDisconnect"])
            exit(1)
        }
        emit("passed", ["disconnected": true])
        exit(0)
    }
}

let client = Client()
withExtendedLifetime(client) {
    RunLoop.main.run()
}
