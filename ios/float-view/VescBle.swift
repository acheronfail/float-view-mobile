import CoreBluetooth
import Foundation

protocol VescBleDelegate: AnyObject {
    func vescBle(_ vescBle: VescBle, didReceiveData data: Data, from characteristic: CBCharacteristic)
}

class VescBle: NSObject {

    // MARK: - UUIDs

    static let bleClientCharacteristicConfig = CBUUID(string: "00002902-0000-1000-8000-00805f9b34fb")

    static let vescServiceUUID = CBUUID(string: "6e400001-b5a3-f393-e0a9-e50e24dcca9e")
    static let vescCharacteristicsRx = CBUUID(string: "6e400002-b5a3-f393-e0a9-e50e24dcca9e")
    static let vescCharacteristicsTx = CBUUID(string: "6e400003-b5a3-f393-e0a9-e50e24dcca9e")
    static let rescueServiceUUID = CBUUID(string: "99eb1511-a9e9-4024-b0a4-3dc4b4fabfb0")
    static let rescueCharacteristicConf = CBUUID(string: "99eb1513-a9e9-4024-b0a4-3dc4b4fabfb0")
    static let rescueCharacteristicHwVer = CBUUID(string: "99eb1515-a9e9-4024-b0a4-3dc4b4fabfb0")
    static let characteristicFw = CBUUID(string: "99eb1514-a9e9-4024-b0a4-3dc4b4fabfb0")

    // MARK: - Scan Result Types

    enum ScanResult {
        case deviceFound(name: String?, address: String)
        case scanComplete
        case error(String)
    }

    enum ConnectionResult {
        case connected(CBPeripheral)
        case ready(CBPeripheral)
        case disconnected
        case error(String)
    }

    // MARK: - Properties

    weak var delegate: VescBleDelegate?

    private var centralManager: CBCentralManager!
    private var foundDevices: [String: CBPeripheral] = [:]
    private var connectedPeripheral: CBPeripheral?

    private var isScanning = false
    private var scanCompletion: ((ScanResult) -> Void)?
    private var scanTimer: Timer?

    private var connectionCompletion: ((ConnectionResult) -> Void)?
    private var requiredServices: [CBUUID] = []

    // MARK: - Initialization

    override init() {
        super.init()
        centralManager = CBCentralManager(delegate: self, queue: nil)
    }

    // MARK: - Permissions

    func hasPermissions() -> Bool {
        if #available(iOS 13.1, *) {
            switch CBCentralManager.authorization {
            case .allowedAlways, .restricted:
                return true
            case .denied, .notDetermined:
                return false
            @unknown default:
                return false
            }
        } else {
            // For iOS 13.0 and earlier, assume permissions are granted
            // when Bluetooth is powered on
            return centralManager.state == .poweredOn
        }
    }

    func requestPermissionsIfNeeded() -> Bool {
        // On iOS, Bluetooth permissions are requested automatically when
        // CBCentralManager is first accessed (in init)
        // We just need to check the current state
        return centralManager.state == .poweredOn && hasPermissions()
    }

    // MARK: - Scanning

    func startScan(serviceUUIDs: [CBUUID], timeoutMs: Double = 10000.0, completion: @escaping (ScanResult) -> Void) {
        guard !isScanning else {
            completion(.error("Scan already in progress"))
            return
        }

        guard centralManager.state == .poweredOn else {
            completion(.error("Bluetooth not powered on. State: \(centralManager.state.rawValue)"))
            return
        }

        // For iOS 13.1+, check explicit permissions
        if #available(iOS 13.1, *) {
            guard hasPermissions() else {
                completion(.error("Bluetooth permissions not granted. Authorization: \(CBCentralManager.authorization.rawValue)"))
                return
            }
        }

        foundDevices.removeAll()
        scanCompletion = completion

        let scanOptions: [String: Any] = [
            CBCentralManagerScanOptionAllowDuplicatesKey: false
        ]

        isScanning = true

        if !serviceUUIDs.isEmpty {
            print("VescBle: Starting scan with service filters: \(serviceUUIDs)")
            centralManager.scanForPeripherals(withServices: serviceUUIDs, options: scanOptions)
        } else {
            print("VescBle: Starting scan for all devices")
            centralManager.scanForPeripherals(withServices: nil, options: scanOptions)
        }

        // Stop scan after timeout
        scanTimer = Timer.scheduledTimer(withTimeInterval: timeoutMs / 1000.0, repeats: false) { [weak self] _ in
            self?.stopScan()
        }
    }

    private func stopScan() {
        guard isScanning else { return }

        centralManager.stopScan()
        scanTimer?.invalidate()
        scanTimer = nil
        isScanning = false

        let devices = Array(foundDevices.values)
        print("VescBle: Scan stopped. Devices found: \(devices.count)")
        scanCompletion?(.scanComplete)
        scanCompletion = nil
    }

    // MARK: - Connection

    func connect(to address: String, completion: @escaping (ConnectionResult) -> Void) -> Bool {
        guard let peripheral = foundDevices[address] else {
            completion(.error("Device not found"))
            return false
        }

        connectionCompletion = completion
        requiredServices = [VescBle.vescServiceUUID] // Still require VESC service, but discover all
        connectedPeripheral = peripheral
        peripheral.delegate = self

        centralManager.connect(peripheral, options: nil)
        return true
    }

    func disconnect() {
        guard let peripheral = connectedPeripheral else { return }
        centralManager.cancelPeripheralConnection(peripheral)
    }
}

// MARK: - CBCentralManagerDelegate

extension VescBle: CBCentralManagerDelegate {
    func centralManagerDidUpdateState(_ central: CBCentralManager) {
        switch central.state {
        case .poweredOn:
            print("VescBle: Bluetooth powered on")
        case .poweredOff:
            print("VescBle: Bluetooth powered off")
            if isScanning {
                stopScan()
                scanCompletion?(.error("Bluetooth powered off"))
            }
        case .resetting:
            print("VescBle: Bluetooth resetting")
        case .unauthorized:
            print("VescBle: Bluetooth unauthorized")
            if isScanning {
                stopScan()
                scanCompletion?(.error("Bluetooth unauthorized"))
            }
        case .unsupported:
            print("VescBle: Bluetooth unsupported")
            if isScanning {
                stopScan()
                scanCompletion?(.error("Bluetooth unsupported"))
            }
        case .unknown:
            print("VescBle: Bluetooth state unknown")
        @unknown default:
            print("VescBle: Bluetooth state unknown default")
        }
    }

    func centralManager(_ central: CBCentralManager, didDiscover peripheral: CBPeripheral, advertisementData: [String: Any], rssi RSSI: NSNumber) {
        let address = peripheral.identifier.uuidString

        if foundDevices[address] == nil {
            foundDevices[address] = peripheral
            print("VescBle: Found device \(address) (\(peripheral.name ?? "Unnamed"))")
            scanCompletion?(.deviceFound(name: peripheral.name, address: address))
        }
    }

    func centralManager(_ central: CBCentralManager, didConnect peripheral: CBPeripheral) {
        print("VescBle: Connected to \(peripheral.identifier)")
        connectionCompletion?(.connected(peripheral))

        // Discover services (discover all services like Android, not just required ones)
        peripheral.discoverServices(nil)
    }

    func centralManager(_ central: CBCentralManager, didDisconnectPeripheral peripheral: CBPeripheral, error: Error?) {
        print("VescBle: Disconnected from \(peripheral.identifier)")

        if let error = error {
            print("VescBle: Disconnection error: \(error.localizedDescription)")
            connectionCompletion?(.error("Disconnection error: \(error.localizedDescription)"))
        } else {
            connectionCompletion?(.disconnected)
        }

        connectedPeripheral = nil
        connectionCompletion = nil
    }

    func centralManager(_ central: CBCentralManager, didFailToConnect peripheral: CBPeripheral, error: Error?) {
        print("VescBle: Failed to connect to \(peripheral.identifier)")

        let errorMessage = error?.localizedDescription ?? "Unknown connection error"
        connectionCompletion?(.error("Failed to connect: \(errorMessage)"))

        connectedPeripheral = nil
        connectionCompletion = nil
    }
}

// MARK: - CBPeripheralDelegate

extension VescBle: CBPeripheralDelegate {
    func peripheral(_ peripheral: CBPeripheral, didDiscoverServices error: Error?) {
        if let error = error {
            print("VescBle: Service discovery error: \(error.localizedDescription)")
            connectionCompletion?(.error("Service discovery error: \(error.localizedDescription)"))
            return
        }

        guard let services = peripheral.services else {
            connectionCompletion?(.error("No services found"))
            return
        }

        print("VescBle: Discovered \(services.count) services")

        // Check if we have the required services
        let hasRequiredServices = requiredServices.isEmpty || requiredServices.allSatisfy { requiredUUID in
            services.contains { $0.uuid == requiredUUID }
        }

        guard hasRequiredServices else {
            connectionCompletion?(.error("Required services not found"))
            return
        }

        // Discover characteristics for each service
        for service in services {
            peripheral.discoverCharacteristics(nil, for: service)
        }
    }

    func peripheral(_ peripheral: CBPeripheral, didDiscoverCharacteristicsFor service: CBService, error: Error?) {
        if let error = error {
            print("VescBle: Characteristic discovery error: \(error.localizedDescription)")
            connectionCompletion?(.error("Characteristic discovery error: \(error.localizedDescription)"))
            return
        }

        guard let characteristics = service.characteristics else {
            print("VescBle: No characteristics found for service \(service.uuid)")
            return
        }

        print("VescBle: Discovered \(characteristics.count) characteristics for service \(service.uuid)")

        // Check if all required characteristics are available
        if service.uuid == VescBle.vescServiceUUID {
            let hasRx = characteristics.contains { $0.uuid == VescBle.vescCharacteristicsRx }
            let hasTx = characteristics.contains { $0.uuid == VescBle.vescCharacteristicsTx }

            if hasRx && hasTx {
                print("VescBle: VESC service ready with all required characteristics")
                connectionCompletion?(.ready(peripheral))
            } else {
                connectionCompletion?(.error("VESC service missing required characteristics"))
            }
        }
    }

    func peripheral(_ peripheral: CBPeripheral, didUpdateValueFor characteristic: CBCharacteristic, error: Error?) {
        if let error = error {
            print("VescBle: Characteristic update error: \(error.localizedDescription)")
            return
        }

        guard let data = characteristic.value else {
            print("VescBle: No data received from characteristic \(characteristic.uuid)")
            return
        }

        print("VescBle: Received data from \(characteristic.uuid): \(data.map { String(format: "%02X", $0) }.joined(separator: " "))")
        delegate?.vescBle(self, didReceiveData: data, from: characteristic)
    }

    func peripheral(_ peripheral: CBPeripheral, didWriteValueFor characteristic: CBCharacteristic, error: Error?) {
        if let error = error {
            print("VescBle: Write error for characteristic \(characteristic.uuid): \(error.localizedDescription)")
        } else {
            print("VescBle: Successfully wrote to characteristic \(characteristic.uuid)")
        }
    }

    func peripheral(_ peripheral: CBPeripheral, didUpdateNotificationStateFor characteristic: CBCharacteristic, error: Error?) {
        if let error = error {
            print("VescBle: Notification state update error for \(characteristic.uuid): \(error.localizedDescription)")
        } else {
            print("VescBle: Notification state updated for \(characteristic.uuid): \(characteristic.isNotifying)")
        }
    }
}
