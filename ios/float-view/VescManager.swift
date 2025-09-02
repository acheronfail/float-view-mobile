import CoreBluetooth
import Foundation

/// High-level manager for VESC device operations. Handles all BLE connection, command sending, and
/// response parsing.
class VescManager: NSObject, ObservableObject {

  static let bleClientCharacteristicConfig = CBUUID(string: "00002902-0000-1000-8000-00805f9b34fb")

  static let vescService = CBUUID(string: "6e400001-b5a3-f393-e0a9-e50e24dcca9e")
  static let vescCharacteristicsRx = CBUUID(string: "6e400002-b5a3-f393-e0a9-e50e24dcca9e")
  static let vescCharacteristicsTx = CBUUID(string: "6e400003-b5a3-f393-e0a9-e50e24dcca9e")
  static let rescueService = CBUUID(string: "99eb1511-a9e9-4024-b0a4-3dc4b4fabfb0")
  static let rescueCharacteristicConf = CBUUID(string: "99eb1513-a9e9-4024-b0a4-3dc4b4fabfb0")
  static let rescueCharacteristicHwVer = CBUUID(string: "99eb1515-a9e9-4024-b0a4-3dc4b4fabfb0")
  static let characteristicFw = CBUUID(string: "99eb1514-a9e9-4024-b0a4-3dc4b4fabfb0")

  // MARK: - Connection Status

  enum ConnectionStatus {
    case connecting
    case connected
    case ready
    case disconnected
  }

  // MARK: - Properties

  private var centralManager: CBCentralManager!
  private var vescMessaging: VescMessaging?
  private let vescParser = VescCommandParser()

  // MARK: - Initialization

  override init() {
    super.init()
    centralManager = CBCentralManager(delegate: self, queue: nil)
  }

  // MARK: - Public Methods

  private var connectedDevice: CBPeripheral?

  /** Connect to a VESC device and get firmware version */
  func connect(address: String) {
    guard let peripheral = foundDevices[address] else {
      return LocalServer.shared.broadcastMessage(
        wsMessage(topic: "connect", event: "error", data: ["message": "Device not found"])
      )
    }

    // setup to receive events from the device
    peripheral.delegate = self

    LocalServer.shared.broadcastMessage(
      wsMessage(topic: "connect", event: "status", data: ["status": "connecting"])
    )

    // start connecting to the device - we'll handle the rest in the delegate methods
    centralManager.connect(peripheral, options: nil)
    connectedDevice = peripheral
  }

  /** Disconnect from the current device */
  func disconnect() {
    if let peripheral = connectedDevice {
      centralManager.cancelPeripheralConnection(peripheral)
    }

    connectedDevice = nil

    LocalServer.shared.broadcastMessage(
      wsMessage(
        topic: "connect",
        event: "disconnect",
        data: ["reason": "user requested"]
      )
    )
  }

  /** Check if permissions are granted */
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
      // For iOS 13.0 and earlier, assume permissions are granted when Bluetooth is powered on
      return centralManager.state == .poweredOn
    }
  }

  /** Request permissions if needed */
  func requestPermissions() -> Bool {
    // On iOS, Bluetooth permissions are requested automatically when
    // CBCentralManager is first accessed (in init)
    // We just need to check the current state
    return centralManager.state == .poweredOn && hasPermissions()
  }

  func send(command: Int, payload data: [UInt8] = []) {
    guard let vescMessaging = vescMessaging else {
      return LocalServer.shared.broadcastMessage(
        wsMessage(
          topic: "command",
          event: "error",
          data: ["message": "VescCommands not setup!"]
        )
      )
    }

    vescMessaging.send(command: command, data: data) { [self] result in
      switch result {
      case .response(let command, let data):
        vescParser.parse(command: command, data: data)
      case .error(let command, let message):
        LocalServer.shared.broadcastMessage(
          wsMessage(
            topic: "command",
            event: "error",
            data: ["command": command, "message": message]
          )
        )
      }
    }
  }

  // MARK: - Scanning

  private var isScanning = false
  private var scanTimeoutWorkItem: DispatchWorkItem?
  private var foundDevices: [String: CBPeripheral] = [:]

  func startScan(serviceUUIDs: [CBUUID], timeoutMs: Double = 3000.0) {
    guard !isScanning else {
      return LocalServer.shared.broadcastMessage(
        wsMessage(topic: "scan", event: "error", data: ["message": "Scan already in progress"])
      )
    }

    guard centralManager.state == .poweredOn else {
      return LocalServer.shared.broadcastMessage(
        wsMessage(
          topic: "scan",
          event: "error",
          data: ["message": "Bluetooth not powered on. State: \(centralManager.state.rawValue)"]
        )
      )
    }

    guard hasPermissions() else {
      return LocalServer.shared.broadcastMessage(
        wsMessage(
          topic: "scan",
          event: "error",
          data: [
            "message":
              "Bluetooth permissions not granted. Authorization: \(CBCentralManager.authorization.rawValue)"
          ]
        )
      )
    }

    foundDevices.removeAll()

    let scanOptions: [String: Any] = [
      CBCentralManagerScanOptionAllowDuplicatesKey: false
    ]

    isScanning = true
    LocalServer.shared.broadcastMessage(
      wsMessage(topic: "scan", event: "status", data: ["status": "start"])
    )

    if serviceUUIDs.isEmpty {
      centralManager.scanForPeripherals(withServices: nil, options: scanOptions)
    } else {
      centralManager.scanForPeripherals(withServices: serviceUUIDs, options: scanOptions)
    }

    // Create new timeout work item
    scanTimeoutWorkItem?.cancel()
    scanTimeoutWorkItem = DispatchWorkItem { [weak self] in self?.stopScan() }
    DispatchQueue.main.asyncAfter(
      deadline: .now() + (timeoutMs / 1000.0), execute: scanTimeoutWorkItem!)
  }

  private func stopScan() {
    guard isScanning else { return }

    centralManager.stopScan()
    scanTimeoutWorkItem?.cancel()
    scanTimeoutWorkItem = nil
    isScanning = false

    print("VescBle: Scan stopped. Devices found: \(Array(foundDevices.values).count)")
    LocalServer.shared.broadcastMessage(
      wsMessage(topic: "scan", event: "status", data: ["status": "complete"])
    )
  }
}

extension VescManager: CBCentralManagerDelegate {
  /**
   * Bluetooth state changed
   */
  func centralManagerDidUpdateState(_ central: CBCentralManager) {
    switch central.state {
    case .poweredOn:
      print("Bluetooth powered on")
    case .poweredOff:
      print("Bluetooth powered off")
      connectedDevice = nil
      if isScanning {
        stopScan()
      }
    case .resetting:
      print("Bluetooth resetting")
      connectedDevice = nil
    case .unauthorized:
      print("Bluetooth unauthorized")
      if isScanning {
        stopScan()
      }
    case .unsupported:
      print("Bluetooth unsupported")
      if isScanning {
        stopScan()
      }
    case .unknown:
      print("Bluetooth state unknown")
    @unknown default:
      print("Bluetooth state unknown default")
    }
  }

  /*
   * New devices found while scanning
   */
  func centralManager(
    _ central: CBCentralManager, didDiscover peripheral: CBPeripheral,
    advertisementData: [String: Any], rssi RSSI: NSNumber
  ) {
    let address = peripheral.identifier.uuidString
    if foundDevices[address] == nil {
      foundDevices[address] = peripheral
      LocalServer.shared.broadcastMessage(
        wsMessage(
          topic: "scan",
          event: "device",
          data: ["name": peripheral.name ?? "Unnamed", "address": address]
        )
      )
    }
  }

  /*
   * Connected to the device
   */
  func centralManager(_ central: CBCentralManager, didConnect peripheral: CBPeripheral) {
    LocalServer.shared.broadcastMessage(
      wsMessage(topic: "connect", event: "status", data: ["status": "connected"])
    )

    // after connection attempt to discover all services
    peripheral.discoverServices(nil)
  }

  /**
   * Failed to connect to device
   */
  func centralManager(
    _ central: CBCentralManager, didFailToConnect peripheral: CBPeripheral, error: Error?
  ) {
    connectedDevice = nil
    let errorMessage = error?.localizedDescription ?? "Unknown connection error"
    LocalServer.shared.broadcastMessage(
      wsMessage(
        topic: "connect",
        event: "error",
        data: ["message": errorMessage]
      )
    )
  }

  /*
   * Disconnected from device
   */
  func centralManager(
    _ central: CBCentralManager, didDisconnectPeripheral peripheral: CBPeripheral, error: Error?
  ) {
    connectedDevice = nil
    if let error = error {
      LocalServer.shared.broadcastMessage(
        wsMessage(
          topic: "connect",
          event: "disconnect",
          data: ["reason": error.localizedDescription]
        )
      )
    }
  }
}

extension VescManager: CBPeripheralDelegate {
  /*
   * When we discover services check for required ones and discover all their characteristics
   */
  func peripheral(_ peripheral: CBPeripheral, didDiscoverServices error: Error?) {
    if let error = error {
      return LocalServer.shared.broadcastMessage(
        wsMessage(
          topic: "connect",
          event: "error",
          data: ["message": "Service discovery error: \(error.localizedDescription)"]
        )
      )
    }

    guard let services = peripheral.services else {
      return LocalServer.shared.broadcastMessage(
        wsMessage(
          topic: "connect",
          event: "error",
          data: ["message": "No services found"]
        )
      )
    }

    guard services.contains(where: { service in service.uuid == VescManager.vescService }) else {
      disconnect()
      return LocalServer.shared.broadcastMessage(
        wsMessage(
          topic: "connect",
          event: "error",
          data: ["message": "No VESC service found"]
        )
      )
    }

    // discover all characteristics for each service
    for service in services {
      peripheral.discoverCharacteristics(nil, for: service)
    }
  }

  /*
   * When we discover characteristics setup notifications on them
   */
  func peripheral(
    _ peripheral: CBPeripheral, didDiscoverCharacteristicsFor service: CBService, error: Error?
  ) {
    if let error = error {
      return LocalServer.shared.broadcastMessage(
        wsMessage(
          topic: "connect",
          event: "error",
          data: ["message": "Characteristic discovery error: \(error.localizedDescription)"]
        )
      )
    }

    // for now, we only care about the VESC service
    guard service.uuid == VescManager.vescService else { return }

    // TODO: check if this func is called multiple times or just once...
    // i.e., do we need to disconnect and error here if the characteristics aren't found? or timeout?
    guard let characteristics = service.characteristics else { return }
    for char in characteristics {
      // enable notifications on the VESC transmission line, so we can get responses to commands
      if char.uuid == VescManager.vescCharacteristicsTx {
        peripheral.setNotifyValue(true, for: char)
      }
    }
  }

  /*
   * When the notification state for a characteristic is updated
   */
  func peripheral(
    _ peripheral: CBPeripheral, didUpdateNotificationStateFor characteristic: CBCharacteristic,
    error: Error?
  ) {
    if let error = error {
      return LocalServer.shared.broadcastMessage(
        wsMessage(
          topic: "connect",
          event: "error",
          data: ["message": "Characteristic notification error: \(error.localizedDescription)"]
        )
      )
    }

    // If we've enabled notifications for the VESC transmission line...
    if characteristic.uuid == VescManager.vescCharacteristicsTx && characteristic.isNotifying {
      // ... and we've discovered the VESC receiving line, then setup our command parser - we're ready!
      let chars =
        peripheral.services?.first(where: { $0.uuid == VescManager.vescService })?.characteristics
        ?? []
      if let rx = chars.first(where: { $0.uuid == VescManager.vescCharacteristicsRx }) {
        vescMessaging = VescMessaging(peripheral: peripheral, vescRx: rx)
        LocalServer.shared.broadcastMessage(
          wsMessage(
            topic: "connect",
            event: "status",
            data: ["status": "ready"]
          )
        )
      } else {
        LocalServer.shared.broadcastMessage(
          wsMessage(
            topic: "connect",
            event: "error",
            data: [
              "message": "Notifications enabled, but failed to find all required characteristics"
            ]
          )
        )
      }

    }
  }

  /*
   * When we receive data on the VESC transmission line
   */
  func peripheral(
    _ peripheral: CBPeripheral, didUpdateValueFor characteristic: CBCharacteristic, error: Error?
  ) {
    if let error = error {
      return LocalServer.shared.broadcastMessage(
        wsMessage(
          topic: "command",
          event: "error",
          data: ["message": "Error receiving update from VESC tx: \(error.localizedDescription)"]
        )
      )
    }

    guard let data = characteristic.value else { return }
    guard let vescCommands = vescMessaging else {
      return LocalServer.shared.broadcastMessage(
        wsMessage(
          topic: "command",
          event: "error",
          data: ["message": "Received data but VescCommands not initialized"]
        )
      )
    }

    vescCommands.parseResponse(data)
  }
}
