import CoreBluetooth
import Foundation

private var uartService: CBUUID {
  CBUUID(string: "6E400001-B5A3-F393-E0A9-E50E24DCCA9E")
}

private var uartWrite: CBUUID {
  CBUUID(string: "6E400002-B5A3-F393-E0A9-E50E24DCCA9E")
}

private var uartNotify: CBUUID {
  CBUUID(string: "6E400003-B5A3-F393-E0A9-E50E24DCCA9E")
}

public final class CoreBluetoothAdapter: NSObject, BluetoothPort, @unchecked Sendable {
  public typealias EventHandler = @Sendable (BluetoothEvent) -> Void
  public typealias DiagnosticHandler = @Sendable (String) -> Void
  private let event: EventHandler
  private let diagnostic: DiagnosticHandler
  private let queue: DispatchQueue
  private let restorationIdentifier: String?
  private lazy var central = CBCentralManager(delegate: self, queue: queue, options:
    restorationIdentifier.map { [CBCentralManagerOptionRestoreIdentifierKey: $0] })
  private var session: BluetoothSession?

  public init(queue: DispatchQueue = DispatchQueue(label: "fail.acheron.float-view.bluetooth"),
              eventHandler: @escaping EventHandler, diagnosticHandler: @escaping DiagnosticHandler = { _ in },
              restorationIdentifier: String? = nil)
  {
    self.queue = queue; event = eventHandler; diagnostic = diagnosticHandler
    self.restorationIdentifier = restorationIdentifier
    super.init(); _ = central
  }

  public func currentState() -> BluetoothState {
    let radio: BluetoothRadioState = switch central.state {
    case .poweredOn: .poweredOn
    case .poweredOff: .poweredOff
    case .unsupported: .unsupported
    case .resetting: .resetting
    default: .unknown
    }
    let permission: BluetoothPermissionState = switch CBManager.authorization {
    case .allowedAlways: .granted
    case .denied: .denied
    case .restricted: .restricted
    default: .notDetermined
    }
    return BluetoothState(radio: radio, scanPermission: permission, connectPermission: permission)
  }

  public func requestPermission() {
    queue.async { [self] in
      if central.state == .poweredOn {
        central.scanForPeripherals(withServices: [uartService]); central.stopScan()
      }
      event(.stateChanged(currentState()))
    }
  }

  public func startDiscovery() {
    queue.async { [self] in
      guard central.state == .poweredOn else { event(.scanFailed("bluetooth_unavailable")); return }
      central.scanForPeripherals(withServices: nil, options: [CBCentralManagerScanOptionAllowDuplicatesKey: true])
    }
  }

  public func stopDiscovery() {
    queue.async { [self] in central.stopScan() }
  }

  public func connect(_ deviceID: DeviceID, generation: BluetoothConnection) {
    queue.async { [self] in
      session?.cancel()
      central.stopScan()
      session = BluetoothSession(deviceID, generation: generation, queue: queue, event: event, diagnostic: diagnostic)
    }
  }

  public func enableNotifications(generation: BluetoothConnection, operation: UInt64) {
    queue.async { [self] in
      guard let session, session.generation == generation else { return }
      session.enable(operation)
    }
  }

  public func write(_ bytes: Data, to _: DeviceID, generation: BluetoothConnection, operation: UInt64, response: Bool) {
    queue.async { [self] in
      guard let session, session.generation == generation else { return }
      session.write(bytes, operation: operation, response: response)
    }
  }

  public func disconnect(_: DeviceID) {
    queue.async { [self] in session?.cancel(); session = nil }
  }
}

extension CoreBluetoothAdapter: CBCentralManagerDelegate {
  public func centralManagerDidUpdateState(_: CBCentralManager) {
    event(.stateChanged(currentState()))
  }

  public func centralManager(_: CBCentralManager, willRestoreState _: [String: Any]) {
    // The service explicitly adopts saved identity; restoration never publishes ready.
    diagnostic("restoration_observed")
  }

  public func centralManager(_: CBCentralManager, didDiscover peripheral: CBPeripheral,
                             advertisementData: [String: Any], rssi: NSNumber)
  {
    let services = advertisementData[CBAdvertisementDataServiceUUIDsKey] as? [CBUUID] ?? []
    event(.deviceFound(DeviceID(peripheral.identifier.uuidString),
                       name: advertisementData[CBAdvertisementDataLocalNameKey] as? String ?? peripheral.name,
                       rssi: rssi.intValue, uart: services.contains(uartService)))
  }
}

/// A separate OS delegate/object lifetime per attempt prevents an old callback from
/// acquiring the identity of a replacement connection to the same peripheral.
private final class BluetoothSession: NSObject, CBCentralManagerDelegate, CBPeripheralDelegate {
  let generation: BluetoothConnection
  private let device: DeviceID
  private let event: CoreBluetoothAdapter.EventHandler
  private let diagnostic: CoreBluetoothAdapter.DiagnosticHandler
  private var central: CBCentralManager!
  private var peripheral: CBPeripheral?
  private var tx: CBCharacteristic?
  private var rx: CBCharacteristic?
  private var setup: UInt64?
  private var current: (id: UInt64, bytes: Data, response: Bool, submitted: Bool)?
  private var cancelled = false
  private var reportedWrite = false

  init(_ device: DeviceID, generation: BluetoothConnection, queue: DispatchQueue,
       event: @escaping CoreBluetoothAdapter.EventHandler,
       diagnostic: @escaping CoreBluetoothAdapter.DiagnosticHandler)
  {
    self.device = device; self.generation = generation; self.event = event; self.diagnostic = diagnostic
    super.init(); central = CBCentralManager(delegate: self, queue: queue)
  }

  func cancel() {
    cancelled = true; current = nil; setup = nil
    peripheral?.delegate = nil
    if let peripheral {
      central.cancelPeripheralConnection(peripheral)
    }
    central.delegate = nil; peripheral = nil; tx = nil; rx = nil
  }

  private func link(_ state: ConnectionState) {
    event(.connectionChanged(device, generation, state))
  }

  private func fail() {
    guard !cancelled else { return }; link(.failed); cancel()
  }

  func centralManagerDidUpdateState(_ central: CBCentralManager) {
    guard !cancelled, peripheral == nil else { return }
    guard central.state == .poweredOn else {
      if central.state != .unknown, central.state != .resetting {
        fail()
      }
      return
    }
    guard let id = UUID(uuidString: device.value),
          let found = central.retrievePeripherals(withIdentifiers: [id]).first else { fail(); return }
    peripheral = found; found.delegate = self
    // Acquire this manager's link even if another manager already knows it as connected.
    central.connect(found)
  }

  func centralManager(_: CBCentralManager, didConnect peripheral: CBPeripheral) {
    guard !cancelled, peripheral === self.peripheral else { return }
    link(.discoveringServices); peripheral.discoverServices([uartService])
  }

  func centralManager(_: CBCentralManager, didFailToConnect _: CBPeripheral, error _: Error?) {
    fail()
  }

  func centralManager(_: CBCentralManager, didDisconnectPeripheral _: CBPeripheral, error _: Error?) {
    guard !cancelled else { return }; link(.disconnected); cancel()
  }

  func peripheral(_ peripheral: CBPeripheral, didDiscoverServices error: Error?) {
    guard !cancelled, peripheral === self.peripheral else { return }
    guard error == nil, let service = peripheral.services?.first(where: { $0.uuid == uartService }) else {
      event(.profile(
        generation,
        BluetoothProfile(service: false, notify: false, withResponse: 0, withoutResponse: 0)
      )); return
    }
    peripheral.discoverCharacteristics([uartWrite, uartNotify], for: service)
  }

  func peripheral(_ peripheral: CBPeripheral, didDiscoverCharacteristicsFor service: CBService, error: Error?) {
    guard !cancelled, peripheral === self.peripheral else { return }
    tx = service.characteristics?.first { $0.uuid == uartWrite }
    rx = service.characteristics?.first { $0.uuid == uartNotify }
    event(.profile(generation, BluetoothProfile(service: error == nil,
                                                notify: rx?.properties.contains(.notify) == true,
                                                withResponse: tx?.properties.contains(.write) == true ? peripheral
                                                  .maximumWriteValueLength(for: .withResponse) : 0,
                                                withoutResponse: tx?.properties
                                                  .contains(.writeWithoutResponse) == true ? peripheral
                                                  .maximumWriteValueLength(for: .withoutResponse) : 0)))
  }

  func enable(_ operation: UInt64) {
    guard !cancelled, setup == nil, let peripheral, let rx else { return }
    setup = operation; peripheral.setNotifyValue(true, for: rx)
  }

  func peripheral(
    _ peripheral: CBPeripheral,
    didUpdateNotificationStateFor characteristic: CBCharacteristic,
    error: Error?
  ) {
    guard !cancelled, peripheral === self.peripheral, characteristic === rx, let setup else { return }
    event(.notifications(generation, setup, error == nil && characteristic.isNotifying))
  }

  func write(_ bytes: Data, operation: UInt64, response: Bool) {
    guard !cancelled, current == nil, bytes.count <= 244 else { fail(); return }
    current = (operation, bytes, response, false); submit()
  }

  private func progress(_ operation: UInt64, _ result: String) {
    event(.writeProgress(generation, operation, result))
  }

  private func submit() {
    guard let peripheral, let tx, let value = current else { return }
    if !value.submitted {
      guard value.response || peripheral.canSendWriteWithoutResponse else { return }
      current?.submitted = true
      peripheral.writeValue(value.bytes, for: tx, type: value.response ? .withResponse : .withoutResponse)
      progress(value.id, "submitted")
      if !reportedWrite {
        diagnostic(
          "gatt_write mode=\(value.response ? "with_response" : "without_response") negotiated=\(peripheral.maximumWriteValueLength(for: value.response ? .withResponse : .withoutResponse)) cap=244 payload=\(value.bytes.count)"
        )
        reportedWrite = true
      }
    }
    if !value.response, peripheral.canSendWriteWithoutResponse {
      current = nil; progress(value.id, "available")
    }
  }

  func peripheralIsReady(toSendWriteWithoutResponse _: CBPeripheral) {
    if !cancelled {
      submit()
    }
  }

  func peripheral(_ peripheral: CBPeripheral, didWriteValueFor characteristic: CBCharacteristic, error: Error?) {
    guard !cancelled, peripheral === self.peripheral, characteristic === tx, let value = current,
          value.response else { return }
    current = nil; progress(value.id, error == nil ? "acknowledged" : "failed")
  }

  func peripheral(_ peripheral: CBPeripheral, didUpdateValueFor characteristic: CBCharacteristic, error: Error?) {
    guard !cancelled, peripheral === self.peripheral, characteristic === rx, error == nil,
          let bytes = characteristic.value, bytes.count <= 8192 else { return }
    event(.notification(device, generation, bytes))
  }
}
