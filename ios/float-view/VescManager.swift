import CoreBluetooth
import Foundation

/**
 * High-level manager for VESC device operations. Handles all BLE connection, command sending, and
 * response parsing.
 */
class VescManager: NSObject, ObservableObject {

    // MARK: - Listener Protocol

    protocol VescManagerListener: AnyObject {
        func onScanDeviceFound(name: String?, address: String)
        func onScanComplete()
        func onScanError(message: String)

        func onConnectionStatusChanged(status: ConnectionStatus, address: String?)
        func onConnectionError(message: String)

        func onVescResponse(command: Int, data: Data)
        func onCommandError(command: Int, message: String)
    }

    // MARK: - Connection Status

    enum ConnectionStatus {
        case connecting
        case connected
        case ready
        case disconnected
    }

    // MARK: - Properties

    private let vescBle = VescBle()
    private var vescCommands: VescCommands?
    weak var listener: VescManagerListener?

    // MARK: - Initialization

    override init() {
        super.init()
        vescBle.delegate = self
    }

    // MARK: - Public Methods

    /** Set the listener for VESC manager events */
    func setListener(_ listener: VescManagerListener?) {
        self.listener = listener
    }

    /** Start scanning for VESC devices */
    func startScan(timeoutMs: Double = 3000.0) -> Bool {
        guard vescBle.hasPermissions() else {
            listener?.onScanError(message: "no permissions")
            return false
        }

        vescBle.startScan(
            serviceUUIDs: [VescBle.vescServiceUUID],
            timeoutMs: timeoutMs
        ) { [weak self] result in
            switch result {
            case .deviceFound(let name, let address):
                self?.listener?.onScanDeviceFound(name: name, address: address)
            case .scanComplete:
                print("VescManager: BLE scan complete")
                self?.listener?.onScanComplete()
            case .error(let message):
                print("VescManager: BLE scan error: \(message)")
                self?.listener?.onScanError(message: message)
            }
        }
        return true
    }

    /** Connect to a VESC device and get firmware version */
    func connectAndGetFirmware(address: String) {
        listener?.onConnectionStatusChanged(status: .connecting, address: nil)

        let _ = vescBle.connect(to: address) { [weak self] result in
            guard let self = self else { return }

            switch result {
            case .connected(let peripheral):
                print("VescManager: Connected to device: \(peripheral.identifier)")
                self.listener?.onConnectionStatusChanged(status: .connected, address: address)

            case .ready(let peripheral):
                print("VescManager: Device ready: \(peripheral.identifier)")
                self.listener?.onConnectionStatusChanged(status: .ready, address: address)

                // Initialize VESC commands and get firmware version
                self.initializeVescCommands(peripheral: peripheral)

            case .disconnected:
                print("VescManager: Disconnected from device")
                self.listener?.onConnectionStatusChanged(status: .disconnected, address: address)
                self.vescCommands?.cleanup()
                self.vescCommands = nil

            case .error(let message):
                print("VescManager: Connection error: \(message)")
                self.listener?.onConnectionError(message: message)
                self.vescCommands?.cleanup()
                self.vescCommands = nil
            }
        }
    }

    /** Disconnect from the current device */
    func disconnect() {
        vescCommands?.cleanup()
        vescBle.disconnect()
        vescCommands = nil
    }

    /** Check if permissions are granted */
    func hasPermissions() -> Bool {
        return vescBle.hasPermissions()
    }

    /** Request permissions if needed */
    func requestPermissions() -> Bool {
        return vescBle.requestPermissionsIfNeeded()
    }

    // MARK: - Private Methods

    /** Initialize VESC commands and request firmware version */
    private func initializeVescCommands(peripheral: CBPeripheral) {
        let vescCommands = VescCommands(peripheral: peripheral)
        self.vescCommands = vescCommands

        // Check if connection is ready
        guard vescCommands.isReady() else {
            print("VescManager: Connection not ready for VESC commands")
            disconnect()
            return
        }

        // Enable notifications
        guard vescCommands.enableNotifications() else {
            print("VescManager: Failed to enable notifications")
            disconnect()
            return
        }

        // Give a small delay for notifications to be fully set up
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) { [weak self] in
            guard let self = self else { return }

            // Log available services and characteristics for debugging
            self.logAvailableCharacteristics(peripheral: peripheral)

            // Request firmware version
            vescCommands.sendCommand(
                command: VescCommands.COMM_FW_VERSION,
                data: []
            ) { [weak self] result in
                switch result {
                case .response(let command, let data):
                    self?.listener?.onVescResponse(command: command, data: data)
                case .error(let command, let message):
                    self?.listener?.onCommandError(command: command, message: message)
                }
            }
        }
    }

    private func logAvailableCharacteristics(peripheral: CBPeripheral) {
        print("VescManager: === Available Services and Characteristics ===")

        guard let services = peripheral.services else {
            print("VescManager: No services found")
            return
        }

        print("VescManager: Total services found: \(services.count)")

        for service in services {
            print("VescManager: Service UUID: \(service.uuid)")

            guard let characteristics = service.characteristics else {
                print("VescManager:   No characteristics found")
                continue
            }

            print("VescManager:   Characteristics count: \(characteristics.count)")

            for characteristic in characteristics {
                print("VescManager:     Characteristic UUID: \(characteristic.uuid)")
                print("VescManager:     Properties: \(getPropertiesString(properties: characteristic.properties))")

                if let descriptors = characteristic.descriptors, !descriptors.isEmpty {
                    print("VescManager:     Descriptors count: \(descriptors.count)")
                    for descriptor in descriptors {
                        print("VescManager:       Descriptor UUID: \(descriptor.uuid)")
                    }
                }
            }
        }
        print("VescManager: === End of Services and Characteristics ===")
    }

    private func getPropertiesString(properties: CBCharacteristicProperties) -> String {
        var props: [String] = []

        if properties.contains(.read) { props.append("READ") }
        if properties.contains(.write) { props.append("WRITE") }
        if properties.contains(.writeWithoutResponse) { props.append("WRITE_NO_RESPONSE") }
        if properties.contains(.notify) { props.append("NOTIFY") }
        if properties.contains(.indicate) { props.append("INDICATE") }
        if properties.contains(.broadcast) { props.append("BROADCAST") }
        if properties.contains(.extendedProperties) { props.append("EXTENDED_PROPS") }
        if properties.contains(.authenticatedSignedWrites) { props.append("SIGNED_WRITE") }

        return props.isEmpty ? "NONE" : props.joined(separator: ", ")
    }
}

// MARK: - VescBle Delegate

extension VescManager: VescBleDelegate {
    func vescBle(_ vescBle: VescBle, didReceiveData data: Data, from characteristic: CBCharacteristic) {
        vescCommands?.onCharacteristicChanged(characteristic: characteristic, value: data)
    }
}

// MARK: - VESC Command Constants (moved to VescCommands.swift)
