import CoreBluetooth
import Foundation

public final class CoreBluetoothAdapter: NSObject, BluetoothPort, @unchecked Sendable {
  public typealias EventHandler = @Sendable (BluetoothEvent) -> Void
  public typealias DiagnosticHandler = @Sendable (String) -> Void

  private static var serviceUUID: CBUUID {
    CBUUID(string: "6E400001-B5A3-F393-E0A9-E50E24DCCA9E")
  }

  private static var writeUUID: CBUUID {
    CBUUID(string: "6E400002-B5A3-F393-E0A9-E50E24DCCA9E")
  }

  private static var notifyUUID: CBUUID {
    CBUUID(string: "6E400003-B5A3-F393-E0A9-E50E24DCCA9E")
  }

  private let eventHandler: EventHandler
  private let diagnosticHandler: DiagnosticHandler
  private let queue: DispatchQueue
  private let restorationIdentifier: String?
  private lazy var central = CBCentralManager(
    delegate: self,
    queue: queue,
    options: restorationIdentifier.map {
      [CBCentralManagerOptionRestoreIdentifierKey: $0]
    }
  )
  private var peripheralsByID: [DeviceID: CBPeripheral] = [:]
  private var activeDeviceID: DeviceID?
  private var activePeripheral: CBPeripheral?
  private var writeCharacteristic: CBCharacteristic?
  private var notifyCharacteristic: CBCharacteristic?
  private var pendingWrites: [Data] = []
  private var pendingWriteBytes = 0
  private var writeInFlight = false
  private var savedDeviceIDs: Set<DeviceID> = []
  private var setupDeadline: DispatchWorkItem?
  private var connectionGeneration: UInt64 = 0

  public init(
    queue: DispatchQueue = DispatchQueue(label: "fail.acheron.float-view.bluetooth"),
    eventHandler: @escaping EventHandler,
    diagnosticHandler: @escaping DiagnosticHandler = { _ in },
    restorationIdentifier: String? = nil
  ) {
    self.queue = queue
    self.eventHandler = eventHandler
    self.diagnosticHandler = diagnosticHandler
    self.restorationIdentifier = restorationIdentifier
    super.init()
    _ = central
  }

  public func currentState() -> BluetoothState {
    BluetoothState(
      radio: Self.radioState(central.state),
      scanPermission: Self.permissionState(),
      connectPermission: Self.permissionState()
    )
  }

  public func requestPermission() {
    queue.async { [self] in
      guard central.state == .poweredOn else {
        eventHandler(.stateChanged(currentState()))
        return
      }
      central.scanForPeripherals(withServices: [Self.serviceUUID])
      central.stopScan()
      eventHandler(.stateChanged(currentState()))
    }
  }

  public func startDiscovery() {
    queue.async { [self] in
      guard central.state == .poweredOn else {
        eventHandler(.scanFailed("bluetooth_unavailable"))
        return
      }
      central.scanForPeripherals(
        withServices: nil,
        options: [CBCentralManagerScanOptionAllowDuplicatesKey: true]
      )
    }
  }

  public func stopDiscovery() {
    queue.async { [self] in central.stopScan() }
  }

  public func configureSavedDeviceIDs(_ deviceIDs: Set<DeviceID>) {
    queue.async { [self] in savedDeviceIDs = deviceIDs }
  }

  public func connect(_ deviceID: DeviceID) {
    queue.async { [self] in
      guard activePeripheral == nil || activeDeviceID == deviceID,
            let peripheral = peripheralsByID[deviceID]
      else {
        eventHandler(.connectionChanged(deviceID, .failed))
        return
      }
      central.stopScan()
      activeDeviceID = deviceID
      activePeripheral = peripheral
      writeCharacteristic = nil
      notifyCharacteristic = nil
      pendingWrites.removeAll(keepingCapacity: true)
      pendingWriteBytes = 0
      writeInFlight = false
      connectionGeneration &+= 1
      scheduleSetupDeadline(for: deviceID, generation: connectionGeneration)
      peripheral.delegate = self
      diagnosticHandler("connect_requested")
      if peripheral.state == .connected {
        eventHandler(.connectionChanged(deviceID, .connected))
        eventHandler(.connectionChanged(deviceID, .discoveringServices))
        peripheral.discoverServices([Self.serviceUUID])
      } else {
        central.connect(peripheral)
      }
    }
  }

  public func write(_ bytes: Data, to deviceID: DeviceID) {
    queue.async { [self] in
      guard deviceID == activeDeviceID,
            activePeripheral != nil,
            writeCharacteristic != nil
      else { return }
      let writeMode = VescBluetoothPolicy.writeMode(
        supportsWriteWithResponse: writeCharacteristic?.properties.contains(.write) == true,
        supportsWriteWithoutResponse:
        writeCharacteristic?.properties.contains(.writeWithoutResponse) == true
      )
      guard writeMode != .unsupported else {
        failConnection("vesc_write_characteristic_unavailable")
        return
      }
      let writeType: CBCharacteristicWriteType =
        writeMode == .withResponse ? .withResponse : .withoutResponse
      let negotiated = activePeripheral?.maximumWriteValueLength(for: writeType) ?? 20
      let chunkSize = max(1, min(negotiated, Self.maximumWriteChunkBytes))
      let chunks = VescBluetoothPolicy.chunks(bytes, maximumPayloadBytes: chunkSize)
      guard VescBluetoothPolicy.acceptsQueuedWrite(
        pendingWrites: pendingWrites.count,
        pendingBytes: pendingWriteBytes,
        newWrites: chunks.count,
        newBytes: bytes.count
      )
      else {
        failConnection("vesc_write_queue_full")
        return
      }
      pendingWrites.append(contentsOf: chunks)
      pendingWriteBytes += bytes.count
      drainWriteQueue()
    }
  }

  public func disconnect(_ deviceID: DeviceID) {
    queue.async { [self] in
      guard deviceID == activeDeviceID, let peripheral = activePeripheral else {
        eventHandler(.connectionChanged(deviceID, .disconnected))
        return
      }
      if let notifyCharacteristic, notifyCharacteristic.isNotifying {
        peripheral.setNotifyValue(false, for: notifyCharacteristic)
      }
      central.cancelPeripheralConnection(peripheral)
    }
  }

  private func failConnection(_ reason: String) {
    guard let deviceID = activeDeviceID else { return }
    eventHandler(.scanFailed(reason))
    eventHandler(.connectionChanged(deviceID, .failed))
    if let activePeripheral {
      central.cancelPeripheralConnection(activePeripheral)
    } else {
      clearConnection()
      eventHandler(.connectionChanged(deviceID, .disconnected))
    }
  }

  private func clearConnection() {
    activeDeviceID = nil
    activePeripheral?.delegate = nil
    activePeripheral = nil
    writeCharacteristic = nil
    notifyCharacteristic = nil
    pendingWrites.removeAll(keepingCapacity: true)
    pendingWriteBytes = 0
    writeInFlight = false
    setupDeadline?.cancel()
    setupDeadline = nil
  }

  private func drainWriteQueue() {
    guard !writeInFlight, !pendingWrites.isEmpty,
          let peripheral = activePeripheral,
          let characteristic = writeCharacteristic
    else { return }
    if characteristic.properties.contains(.write) {
      writeInFlight = true
      let chunk = pendingWrites.removeFirst()
      pendingWriteBytes -= chunk.count
      peripheral.writeValue(
        chunk,
        for: characteristic,
        type: .withResponse
      )
      return
    }
    guard characteristic.properties.contains(.writeWithoutResponse),
          peripheral.canSendWriteWithoutResponse
    else { return }
    let chunk = pendingWrites.removeFirst()
    pendingWriteBytes -= chunk.count
    peripheral.writeValue(
      chunk,
      for: characteristic,
      type: .withoutResponse
    )
    drainWriteQueue()
  }

  private func scheduleSetupDeadline(for deviceID: DeviceID, generation: UInt64) {
    setupDeadline?.cancel()
    let deadline = DispatchWorkItem { [weak self] in
      guard let self, self.activeDeviceID == deviceID,
            VescBluetoothPolicy.acceptsCallback(
              callbackGeneration: generation,
              activeGeneration: self.connectionGeneration,
              isCancelled: false
            ),
            self.notifyCharacteristic?.isNotifying != true
      else { return }
      self.failConnection("vesc_setup_timeout")
    }
    setupDeadline = deadline
    queue.asyncAfter(deadline: .now() + .seconds(10), execute: deadline)
  }

  private static let maximumWriteChunkBytes = 512

  private static func radioState(_ state: CBManagerState) -> BluetoothRadioState {
    switch state {
    case .unknown: .unknown
    case .resetting: .resetting
    case .unsupported: .unsupported
    case .unauthorized: .unknown
    case .poweredOff: .poweredOff
    case .poweredOn: .poweredOn
    @unknown default: .unknown
    }
  }

  private static func permissionState() -> BluetoothPermissionState {
    switch CBManager.authorization {
    case .notDetermined: .notDetermined
    case .restricted: .restricted
    case .denied: .denied
    case .allowedAlways: .granted
    @unknown default: .unknown
    }
  }
}

extension CoreBluetoothAdapter: CBCentralManagerDelegate {
  public func centralManager(
    _: CBCentralManager,
    willRestoreState dictionary: [String: Any]
  ) {
    guard let restored = dictionary[CBCentralManagerRestoredStatePeripheralsKey]
      as? [CBPeripheral],
      let peripheral = restored.first
    else {
      diagnosticHandler("restoration_without_peripheral")
      return
    }
    let deviceID = DeviceID(peripheral.identifier.uuidString)
    peripheralsByID[deviceID] = peripheral
    activeDeviceID = deviceID
    activePeripheral = peripheral
    writeCharacteristic = nil
    notifyCharacteristic = nil
    pendingWrites.removeAll(keepingCapacity: true)
    writeInFlight = false
    peripheral.delegate = self
    diagnosticHandler(
      peripheral.state == .connected
        ? "peripheral_binding_restored_connected"
        : "peripheral_binding_restored_pending"
    )
  }

  public func centralManagerDidUpdateState(_: CBCentralManager) {
    eventHandler(.stateChanged(currentState()))
  }

  public func centralManager(
    _: CBCentralManager,
    didDiscover peripheral: CBPeripheral,
    advertisementData: [String: Any],
    rssi RSSI: NSNumber
  ) {
    let deviceID = DeviceID(peripheral.identifier.uuidString)
    peripheralsByID[deviceID] = peripheral
    let advertisedServices = advertisementData[CBAdvertisementDataServiceUUIDsKey]
      as? [CBUUID] ?? []
    guard VescBluetoothPolicy.shouldPublish(
      advertisesUART: advertisedServices.contains(Self.serviceUUID),
      deviceID: deviceID,
      savedDeviceIDs: savedDeviceIDs
    ) else {
      return
    }
    let advertisedName = advertisementData[CBAdvertisementDataLocalNameKey] as? String
    eventHandler(
      .deviceFound(
        deviceID,
        name: advertisedName ?? peripheral.name,
        rssi: RSSI.intValue
      )
    )
  }

  public func centralManager(_: CBCentralManager, didConnect peripheral: CBPeripheral) {
    guard let deviceID = activeDeviceID, peripheral == activePeripheral else { return }
    eventHandler(.connectionChanged(deviceID, .connected))
    eventHandler(.connectionChanged(deviceID, .discoveringServices))
    peripheral.discoverServices([Self.serviceUUID])
  }

  public func centralManager(
    _: CBCentralManager,
    didFailToConnect peripheral: CBPeripheral,
    error _: Error?
  ) {
    guard let deviceID = activeDeviceID, peripheral == activePeripheral else { return }
    eventHandler(.connectionChanged(deviceID, .failed))
    clearConnection()
    eventHandler(.connectionChanged(deviceID, .disconnected))
  }

  public func centralManager(
    _: CBCentralManager,
    didDisconnectPeripheral peripheral: CBPeripheral,
    error _: Error?
  ) {
    guard let deviceID = activeDeviceID, peripheral == activePeripheral else { return }
    clearConnection()
    eventHandler(.connectionChanged(deviceID, .disconnected))
  }
}

extension CoreBluetoothAdapter: CBPeripheralDelegate {
  public func peripheral(_ peripheral: CBPeripheral, didDiscoverServices error: Error?) {
    guard peripheral == activePeripheral, error == nil else {
      failConnection("vesc_service_unavailable")
      return
    }
    guard let service = peripheral.services?.first(where: { $0.uuid == Self.serviceUUID })
    else {
      failConnection(
        VescBluetoothPolicy.setupFailure(
          hasService: false,
          hasWriteCharacteristic: false,
          writeMode: .unsupported,
          hasNotifyCharacteristic: false,
          hasNotifyProperty: false
        ) ?? "vesc_service_unavailable"
      )
      return
    }
    peripheral.discoverCharacteristics([Self.writeUUID, Self.notifyUUID], for: service)
  }

  public func peripheral(
    _ peripheral: CBPeripheral,
    didDiscoverCharacteristicsFor service: CBService,
    error: Error?
  ) {
    guard peripheral == activePeripheral, error == nil else {
      failConnection("vesc_characteristic_discovery_failed")
      return
    }
    let write = service.characteristics?.first { $0.uuid == Self.writeUUID }
    let notify = service.characteristics?.first { $0.uuid == Self.notifyUUID }
    let writeMode = VescBluetoothPolicy.writeMode(
      supportsWriteWithResponse: write?.properties.contains(.write) == true,
      supportsWriteWithoutResponse: write?.properties.contains(.writeWithoutResponse) == true
    )
    if let failure = VescBluetoothPolicy.setupFailure(
      hasService: true,
      hasWriteCharacteristic: write != nil,
      writeMode: writeMode,
      hasNotifyCharacteristic: notify != nil,
      hasNotifyProperty: notify?.properties.contains(.notify) == true
    ) {
      failConnection(failure)
      return
    }
    writeCharacteristic = write
    notifyCharacteristic = notify
    guard let notifyCharacteristic = notify else { return }
    guard let deviceID = activeDeviceID else { return }
    eventHandler(.connectionChanged(deviceID, .enablingNotifications))
    peripheral.setNotifyValue(true, for: notifyCharacteristic)
  }

  public func peripheral(
    _ peripheral: CBPeripheral,
    didUpdateNotificationStateFor characteristic: CBCharacteristic,
    error: Error?
  ) {
    guard peripheral == activePeripheral, characteristic == notifyCharacteristic else { return }
    guard error == nil, characteristic.isNotifying else {
      failConnection("vesc_notification_enable_failed")
      return
    }
    setupDeadline?.cancel()
    setupDeadline = nil
    if let deviceID = activeDeviceID {
      eventHandler(.connectionChanged(deviceID, .ready))
    }
  }

  public func peripheral(
    _ peripheral: CBPeripheral,
    didUpdateValueFor characteristic: CBCharacteristic,
    error: Error?
  ) {
    guard peripheral == activePeripheral, characteristic == notifyCharacteristic,
          error == nil, let bytes = characteristic.value, let deviceID = activeDeviceID
    else { return }
    eventHandler(.notification(deviceID, bytes))
  }

  public func peripheral(
    _ peripheral: CBPeripheral,
    didWriteValueFor characteristic: CBCharacteristic,
    error: Error?
  ) {
    guard peripheral == activePeripheral, characteristic == writeCharacteristic else { return }
    writeInFlight = false
    guard error == nil else {
      failConnection("vesc_write_failed")
      return
    }
    drainWriteQueue()
  }

  public func peripheralIsReady(toSendWriteWithoutResponse peripheral: CBPeripheral) {
    guard peripheral == activePeripheral else { return }
    drainWriteQueue()
  }
}
