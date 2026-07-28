#!/usr/bin/env swift

import CoreBluetooth
import Foundation

private let service = CBUUID(string: "6E400001-B5A3-F393-E0A9-E50E24DCCA9E")
private let expectedName = "FloatView Test VESC"

final class Scanner: NSObject, CBCentralManagerDelegate {
    private var manager: CBCentralManager!
    private var timeout: Timer?

    override init() {
        super.init()
        manager = CBCentralManager(delegate: self, queue: .main)
        timeout = Timer.scheduledTimer(withTimeInterval: 12, repeats: false) { _ in
            fputs("Timed out waiting for \(expectedName)\n", stderr)
            exit(1)
        }
    }

    func centralManagerDidUpdateState(_ central: CBCentralManager) {
        guard central.state == .poweredOn else {
            if central.state == .unauthorized || central.state == .unsupported {
                fputs("CoreBluetooth unavailable: \(central.state.rawValue)\n", stderr)
                exit(1)
            }
            return
        }
        central.scanForPeripherals(
            withServices: [service],
            options: [CBCentralManagerScanOptionAllowDuplicatesKey: false]
        )
    }

    func centralManager(
        _ central: CBCentralManager,
        didDiscover peripheral: CBPeripheral,
        advertisementData: [String: Any],
        rssi RSSI: NSNumber
    ) {
        let name = advertisementData[CBAdvertisementDataLocalNameKey] as? String
            ?? peripheral.name
        guard name == expectedName else {
            return
        }
        central.stopScan()
        timeout?.invalidate()
        let result: [String: Any] = [
            "schemaVersion": 1,
            "kind": "vesc-test-device-t0-ble",
            "name": name ?? "",
            "serviceUuid": service.uuidString.lowercased(),
            "rssi": RSSI.intValue,
            "passed": true,
        ]
        let data = try! JSONSerialization.data(
            withJSONObject: result,
            options: [.prettyPrinted, .sortedKeys]
        )
        print(String(decoding: data, as: UTF8.self))
        exit(0)
    }
}

let scanner = Scanner()
withExtendedLifetime(scanner) {
    RunLoop.main.run()
}
