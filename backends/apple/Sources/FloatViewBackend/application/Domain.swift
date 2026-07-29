import Foundation

public struct DeviceID: Hashable, Sendable, Codable {
  public let value: String
  public init(_ value: String) { self.value = value }
}

public struct SessionID: Hashable, Sendable, Codable {
  public let value: String
  public init(_ value: String) { self.value = value }
}

public struct JobID: Hashable, Sendable, Codable {
  public let value: String
  public init(_ value: String) { self.value = value }
}

public enum ConnectionState: String, Sendable, Codable {
  case disconnected
  case connecting
  case connected
  case discoveringServices
  case enablingNotifications
  case ready
  case reconnecting
  case disconnecting
  case failed
}

public enum RideState: String, Sendable, Codable {
  case idle
  case riding
  case unknown
}

public enum BluetoothRadioState: String, Sendable, Codable {
  case unknown
  case unsupported
  case poweredOff = "powered_off"
  case poweredOn = "powered_on"
  case resetting
}

public enum BluetoothPermissionState: String, Sendable, Codable {
  case unknown
  case notRequired = "not_required"
  case notDetermined = "not_determined"
  case denied
  case restricted
  case granted
}

public struct BluetoothState: Sendable, Codable, Equatable {
  public let radio: BluetoothRadioState
  public let scanPermission: BluetoothPermissionState
  public let connectPermission: BluetoothPermissionState

  public init(
    radio: BluetoothRadioState,
    scanPermission: BluetoothPermissionState,
    connectPermission: BluetoothPermissionState
  ) {
    self.radio = radio
    self.scanPermission = scanPermission
    self.connectPermission = connectPermission
  }
}

public struct KnownDevice: Sendable, Codable, Equatable {
  public let deviceID: DeviceID
  public let name: String?
  public let rssi: Int?
  public let lastSeenAt: Date

  public init(deviceID: DeviceID, name: String?, rssi: Int?, lastSeenAt: Date) {
    self.deviceID = deviceID
    self.name = name
    self.rssi = rssi
    self.lastSeenAt = lastSeenAt
  }
}

public struct DiscoverySnapshot: Sendable, Equatable {
  public let bluetoothState: BluetoothState
  public let activeScanID: String?
  public let knownDevices: [KnownDevice]

  public init(
    bluetoothState: BluetoothState,
    activeScanID: String?,
    knownDevices: [KnownDevice]
  ) {
    self.bluetoothState = bluetoothState
    self.activeScanID = activeScanID
    self.knownDevices = knownDevices
  }
}

public struct ActiveConnectionSnapshot: Sendable, Equatable {
  public let deviceID: DeviceID
  public let name: String?
  public let state: ConnectionState

  public init(deviceID: DeviceID, name: String?, state: ConnectionState) {
    self.deviceID = deviceID
    self.name = name
    self.state = state
  }
}

public struct FirmwareRecord: Sendable, Codable, Equatable {
  public let major: UInt8
  public let minor: UInt8
  public let hardwareName: String?
  public let firmwareName: String?
  public let hardwareType: UInt8
  public let canID: UInt8?

  public init(
    major: UInt8,
    minor: UInt8,
    hardwareName: String?,
    firmwareName: String?,
    hardwareType: UInt8 = 0,
    canID: UInt8? = nil
  ) {
    self.major = major
    self.minor = minor
    self.hardwareName = hardwareName
    self.firmwareName = firmwareName
    self.hardwareType = hardwareType
    self.canID = canID
  }
}

public enum VescProtocolMessage: Sendable, Equatable {
  case firmware(FirmwareRecord)
  case canDevices([UInt8])
  case refloatInfo(version: String)
  case refloatTelemetryLayout(fields: [String], runtimeFields: [String])
  case refloatTelemetry(RefloatTelemetryRecord)
}

public struct RefloatTelemetryRecord: Sendable, Equatable {
  public let rideState: RideState
  public let boardState: UInt8
  public let footpadState: UInt8
  public let packVoltageV: Double
  public let dutyCycle: Double
  public let controllerTempC: Double?
  public let motorTempC: Double?
  public let motorCurrentA: Double
  public let batteryCurrentA: Double
  public let erpm: Int
  public let speedKph: Double

  public init(
    rideState: RideState,
    boardState: UInt8,
    footpadState: UInt8,
    packVoltageV: Double,
    dutyCycle: Double,
    controllerTempC: Double?,
    motorTempC: Double?,
    motorCurrentA: Double,
    batteryCurrentA: Double,
    erpm: Int,
    speedKph: Double
  ) {
    self.rideState = rideState
    self.boardState = boardState
    self.footpadState = footpadState
    self.packVoltageV = packVoltageV
    self.dutyCycle = dutyCycle
    self.controllerTempC = controllerTempC
    self.motorTempC = motorTempC
    self.motorCurrentA = motorCurrentA
    self.batteryCurrentA = batteryCurrentA
    self.erpm = erpm
    self.speedKph = speedKph
  }
}

public struct TelemetrySample: Sendable, Codable, Equatable {
  public let sequence: Int64
  public let observedAt: Date
  public let elapsedMs: Int64
  public let rideState: RideState
  public let packVoltageV: Double?
  public let dutyCycle: Double?
  public let controllerTempC: Double?
  public let motorTempC: Double?
  public let footpadState: Int?
  public let extensionFields: [String: Double]

  public init(
    sequence: Int64,
    observedAt: Date,
    elapsedMs: Int64,
    rideState: RideState,
    packVoltageV: Double? = nil,
    dutyCycle: Double? = nil,
    controllerTempC: Double? = nil,
    motorTempC: Double? = nil,
    footpadState: Int? = nil,
    extensionFields: [String: Double] = [:]
  ) {
    self.sequence = sequence
    self.observedAt = observedAt
    self.elapsedMs = elapsedMs
    self.rideState = rideState
    self.packVoltageV = packVoltageV
    self.dutyCycle = dutyCycle
    self.controllerTempC = controllerTempC
    self.motorTempC = motorTempC
    self.footpadState = footpadState
    self.extensionFields = extensionFields
  }
}

public struct SessionRecord: Sendable, Codable, Equatable {
  public let sessionID: SessionID
  public let jobID: JobID
  public let deviceID: DeviceID
  public let startedAt: Date
  public let committedSequence: Int64
  public let active: Bool

  public init(
    sessionID: SessionID,
    jobID: JobID,
    deviceID: DeviceID,
    startedAt: Date,
    committedSequence: Int64,
    active: Bool
  ) {
    self.sessionID = sessionID
    self.jobID = jobID
    self.deviceID = deviceID
    self.startedAt = startedAt
    self.committedSequence = committedSequence
    self.active = active
  }
}

public struct SessionSummaryRecord: Sendable, Codable, Equatable {
  public let sessionID: String
  public let deviceHistoryID: String
  public let displayName: String
  public let startedAt: Date
  public let endedAt: Date?
  public let state: String
  public let integrity: String
  public let sampleCount: Int64
  public let eventCount: Int64
  public let ridingElapsedMs: Int64
  public let totalElapsedMs: Int64
  public let distanceM: Double?
  public let maxSpeedMps: Double?
  public let maxDutyCycle: Double?

  public init(
    sessionID: String, deviceHistoryID: String, displayName: String,
    startedAt: Date, endedAt: Date?, state: String, integrity: String,
    sampleCount: Int64, eventCount: Int64, ridingElapsedMs: Int64,
    totalElapsedMs: Int64, distanceM: Double?, maxSpeedMps: Double?,
    maxDutyCycle: Double?
  ) {
    self.sessionID = sessionID
    self.deviceHistoryID = deviceHistoryID
    self.displayName = displayName
    self.startedAt = startedAt
    self.endedAt = endedAt
    self.state = state
    self.integrity = integrity
    self.sampleCount = sampleCount
    self.eventCount = eventCount
    self.ridingElapsedMs = ridingElapsedMs
    self.totalElapsedMs = totalElapsedMs
    self.distanceM = distanceM
    self.maxSpeedMps = maxSpeedMps
    self.maxDutyCycle = maxDutyCycle
  }
}

public struct SessionPage: Sendable, Equatable {
  public let items: [SessionSummaryRecord]
  public let nextCursor: String?

  public init(items: [SessionSummaryRecord], nextCursor: String?) {
    self.items = items
    self.nextCursor = nextCursor
  }
}

public struct SessionListQuery: Sendable, Equatable {
  public let cursor: String?
  public let limit: Int
  public let deviceHistoryID: String?
  public let startedAfter: Date?
  public let startedBefore: Date?
  public let minimumTotalElapsedMs: Int64?
  public let maximumTotalElapsedMs: Int64?
  public let minimumRidingElapsedMs: Int64?
  public let maximumRidingElapsedMs: Int64?
  public let minimumDistanceM: Double?
  public let maximumDistanceM: Double?
  public let state: String?
  public let integrity: String?
  public let sortField: String
  public let sortDescending: Bool

  public init(
    cursor: String? = nil,
    limit: Int = 20,
    deviceHistoryID: String? = nil,
    startedAfter: Date? = nil,
    startedBefore: Date? = nil,
    minimumTotalElapsedMs: Int64? = nil,
    maximumTotalElapsedMs: Int64? = nil,
    minimumRidingElapsedMs: Int64? = nil,
    maximumRidingElapsedMs: Int64? = nil,
    minimumDistanceM: Double? = nil,
    maximumDistanceM: Double? = nil,
    state: String? = nil,
    integrity: String? = nil,
    sortField: String = "startedAt",
    sortDescending: Bool = true
  ) {
    self.cursor = cursor
    self.limit = limit
    self.deviceHistoryID = deviceHistoryID
    self.startedAfter = startedAfter
    self.startedBefore = startedBefore
    self.minimumTotalElapsedMs = minimumTotalElapsedMs
    self.maximumTotalElapsedMs = maximumTotalElapsedMs
    self.minimumRidingElapsedMs = minimumRidingElapsedMs
    self.maximumRidingElapsedMs = maximumRidingElapsedMs
    self.minimumDistanceM = minimumDistanceM
    self.maximumDistanceM = maximumDistanceM
    self.state = state
    self.integrity = integrity
    self.sortField = sortField
    self.sortDescending = sortDescending
  }
}

public struct SessionSeriesPoint: Sendable, Equatable {
  public let elapsedMs: Int64
  public let values: [String: Double?]

  public init(elapsedMs: Int64, values: [String: Double?]) {
    self.elapsedMs = elapsedMs
    self.values = values
  }
}

public struct SessionEventRecord: Sendable, Equatable {
  public let eventID: String
  public let elapsedMs: Int64
  public let kind: String
  public let detailsJSON: String

  public init(eventID: String, elapsedMs: Int64, kind: String, detailsJSON: String) {
    self.eventID = eventID
    self.elapsedMs = elapsedMs
    self.kind = kind
    self.detailsJSON = detailsJSON
  }
}

public struct SessionGroupRecord: Sendable, Equatable {
  public let sessionGroupID: String
  public let deviceHistoryID: String
  public let sessionIDs: [String]
  public let totalElapsedMs: Int64
  public let distanceM: Double?

  public init(
    sessionGroupID: String,
    deviceHistoryID: String,
    sessionIDs: [String],
    totalElapsedMs: Int64,
    distanceM: Double?
  ) {
    self.sessionGroupID = sessionGroupID
    self.deviceHistoryID = deviceHistoryID
    self.sessionIDs = sessionIDs
    self.totalElapsedMs = totalElapsedMs
    self.distanceM = distanceM
  }
}

public struct SessionExportRecord: Sendable, Equatable {
  public let exportID: String
  public let fileName: String
  public let fileURL: URL

  public init(exportID: String, fileName: String, fileURL: URL) {
    self.exportID = exportID
    self.fileName = fileName
    self.fileURL = fileURL
  }
}

public enum ThemePreference: String, Sendable, Codable, Equatable, CaseIterable {
  case system
  case light
  case dark
}

public struct SettingsRecord: Sendable, Codable, Equatable {
  public let revision: Int
  public let idleProbeIntervalMs: Int64
  public let idlePersistIntervalMs: Int64
  public let ridingPersistIntervalMs: Int64
  public let postRideTailMs: Int64
  public let reconnectDurationMs: Int64
  public let automaticSessionLinkingEnabled: Bool
  public let sessionLinkWindowMs: Int64
  public let themePreference: ThemePreference

  public init(
    revision: Int = 0,
    idleProbeIntervalMs: Int64 = 500,
    idlePersistIntervalMs: Int64 = 1_000,
    ridingPersistIntervalMs: Int64 = 100,
    postRideTailMs: Int64 = 15_000,
    reconnectDurationMs: Int64 = 300_000,
    automaticSessionLinkingEnabled: Bool = false,
    sessionLinkWindowMs: Int64 = 7_200_000,
    themePreference: ThemePreference = .system
  ) {
    self.revision = revision
    self.idleProbeIntervalMs = idleProbeIntervalMs
    self.idlePersistIntervalMs = idlePersistIntervalMs
    self.ridingPersistIntervalMs = ridingPersistIntervalMs
    self.postRideTailMs = postRideTailMs
    self.reconnectDurationMs = reconnectDurationMs
    self.automaticSessionLinkingEnabled = automaticSessionLinkingEnabled
    self.sessionLinkWindowMs = sessionLinkWindowMs
    self.themePreference = themePreference
  }
}

public struct SavedDeviceRecord: Sendable, Codable, Equatable {
  public let savedDeviceID: String
  public let deviceHistoryID: String
  public let displayName: String
  public let advertisedName: String?
  public let lastSeenAt: Date?
  public let lastConnectedAt: Date?
  public let profileComplete: Bool

  public init(
    savedDeviceID: String,
    deviceHistoryID: String,
    displayName: String,
    advertisedName: String? = nil,
    lastSeenAt: Date? = nil,
    lastConnectedAt: Date? = nil,
    profileComplete: Bool
  ) {
    self.savedDeviceID = savedDeviceID
    self.deviceHistoryID = deviceHistoryID
    self.displayName = displayName
    self.advertisedName = advertisedName
    self.lastSeenAt = lastSeenAt
    self.lastConnectedAt = lastConnectedAt
    self.profileComplete = profileComplete
  }
}

public struct BoardProfileRecord: Sendable, Codable, Equatable {
  public let savedDeviceID: String
  public let revision: Int
  public let valuesJSON: String
  public let missingFields: [String]

  public init(
    savedDeviceID: String,
    revision: Int,
    valuesJSON: String,
    missingFields: [String]
  ) {
    self.savedDeviceID = savedDeviceID
    self.revision = revision
    self.valuesJSON = valuesJSON
    self.missingFields = missingFields
  }
}

public enum ProductStoreError: Error, Equatable {
  case savedDeviceNotFound
  case profileRevisionConflict(currentRevision: Int)
  case invalidProfilePatch
  case settingsRevisionConflict(currentRevision: Int)
  case invalidSettingsPatch
}

public enum BluetoothEvent: Sendable {
  case stateChanged(BluetoothState)
  case deviceFound(DeviceID, name: String?, rssi: Int?)
  case scanFailed(String)
  case connectionChanged(DeviceID, ConnectionState)
  case notification(DeviceID, Data)
}

public protocol BluetoothPort: AnyObject, Sendable {
  func currentState() -> BluetoothState
  func requestPermission()
  func startDiscovery()
  func stopDiscovery()
  func connect(_ deviceID: DeviceID)
  func write(_ bytes: Data, to deviceID: DeviceID)
  func disconnect(_ deviceID: DeviceID)
}

public protocol VescProtocolPort: AnyObject, Sendable {
  func reset()
  func firmwareRequest(canID: UInt8?) throws -> Data
  func canPingRequest() throws -> Data
  func refloatInfoRequest(canID: UInt8?) throws -> Data
  func refloatTelemetryIDsRequest(canID: UInt8) throws -> Data
  func refloatTelemetryRequest(canID: UInt8) throws -> Data
  func receive(_ bytes: Data) throws -> [VescProtocolMessage]
}

public extension VescProtocolPort {
  func firmwareRequest() throws -> Data { try firmwareRequest(canID: nil) }
  func refloatInfoRequest() throws -> Data { try refloatInfoRequest(canID: nil) }
}

public protocol CancellationPort: AnyObject, Sendable {
  func cancel()
}

public protocol SchedulerPort: Sendable {
  func schedule(
    afterMilliseconds: Int64,
    action: @escaping @Sendable () -> Void
  ) -> any CancellationPort
  func scheduleRepeating(
    everyMilliseconds: Int64,
    action: @escaping @Sendable () -> Void
  ) -> any CancellationPort
}

public final class SystemScheduler: SchedulerPort, @unchecked Sendable {
  private let queue: DispatchQueue

  public init(
    queue: DispatchQueue = DispatchQueue(label: "fail.acheron.float-view.telemetry")
  ) {
    self.queue = queue
  }

  public func schedule(
    afterMilliseconds: Int64,
    action: @escaping @Sendable () -> Void
  ) -> any CancellationPort {
    precondition(afterMilliseconds >= 0)
    let source = DispatchSource.makeTimerSource(queue: queue)
    source.schedule(deadline: .now() + .milliseconds(Int(afterMilliseconds)))
    source.setEventHandler(handler: action)
    source.resume()
    return DispatchTimerCancellation(source)
  }

  public func scheduleRepeating(
    everyMilliseconds: Int64,
    action: @escaping @Sendable () -> Void
  ) -> any CancellationPort {
    precondition(everyMilliseconds > 0)
    let source = DispatchSource.makeTimerSource(queue: queue)
    source.schedule(
      deadline: .now() + .milliseconds(Int(everyMilliseconds)),
      repeating: .milliseconds(Int(everyMilliseconds))
    )
    source.setEventHandler(handler: action)
    source.resume()
    return DispatchTimerCancellation(source)
  }
}

private final class DispatchTimerCancellation: CancellationPort, @unchecked Sendable {
  private let lock = NSLock()
  private var source: DispatchSourceTimer?

  init(_ source: DispatchSourceTimer) {
    self.source = source
  }

  func cancel() {
    lock.lock()
    let current = source
    source = nil
    lock.unlock()
    current?.setEventHandler {}
    current?.cancel()
  }

  deinit { cancel() }
}

public protocol ClockPort: Sendable {
  func monotonicMilliseconds() -> Int64
  func now() -> Date
}

public protocol LogStorePort: AnyObject, Sendable {
  func create(deviceID: DeviceID, at: Date) throws -> SessionRecord
  func append(jobID: JobID, samples: [TelemetrySample]) throws
  func close(jobID: JobID, at: Date, reason: String) throws -> SessionRecord
  func active() -> SessionRecord?
}

public protocol ProductStorePort: AnyObject, Sendable {
  func savedDevices() throws -> [SavedDeviceRecord]
  func boardProfile(savedDeviceID: String) throws -> BoardProfileRecord
  func updateBoardProfile(
    savedDeviceID: String,
    expectedRevision: Int,
    patchJSON: String,
    at: Date
  ) throws -> BoardProfileRecord
  func settings() throws -> SettingsRecord
  func updateSettings(
    expectedRevision: Int,
    patchJSON: String
  ) throws -> SettingsRecord
}

public extension ProductStorePort {
  func settings() throws -> SettingsRecord { SettingsRecord() }
  func updateSettings(
    expectedRevision _: Int,
    patchJSON _: String
  ) throws -> SettingsRecord {
    throw ProductStoreError.invalidSettingsPatch
  }
}

public enum SessionStoreError: Error, Equatable {
  case sessionNotFound
  case sessionActive
  case invalidCursor
  case invalidRange
  case unsupportedField(String)
  case invalidExportFormat
}

public protocol SessionStorePort: AnyObject, Sendable {
  func sessions(_ query: SessionListQuery) throws -> SessionPage
  func session(sessionID: String) throws -> SessionSummaryRecord
  func querySeries(
    sessionID: String,
    startElapsedMs: Int64,
    endElapsedMs: Int64,
    fields: [String],
    maxPoints: Int
  ) throws -> [SessionSeriesPoint]
  func sessionEvents(
    sessionID: String,
    startElapsedMs: Int64,
    endElapsedMs: Int64
  ) throws -> [SessionEventRecord]
  func diagnosticWindow(
    sessionID: String,
    centerElapsedMs: Int64,
    beforeMs: Int64,
    afterMs: Int64,
    fields: [String]
  ) throws -> [SessionSeriesPoint]
  func deleteSession(sessionID: String) throws
  func exportSession(sessionID: String, format: String) throws -> SessionExportRecord
  func sessionGroups() throws -> [SessionGroupRecord]
  func sessionGroup(sessionGroupID: String) throws -> SessionGroupRecord
  func unlinkSessionGroup(sessionGroupID: String) throws
}

public extension SessionStorePort {
  func deleteSession(sessionID _: String) throws { throw SessionStoreError.sessionNotFound }
  func exportSession(sessionID _: String, format _: String) throws -> SessionExportRecord {
    throw SessionStoreError.sessionNotFound
  }
  func sessionGroups() throws -> [SessionGroupRecord] { [] }
  func sessionGroup(sessionGroupID _: String) throws -> SessionGroupRecord {
    throw SessionStoreError.sessionNotFound
  }
  func unlinkSessionGroup(sessionGroupID _: String) throws {}
}

public protocol EventSink: Sendable {
  func publish(_ event: any Sendable)
}

public final class EventHub: EventSink, @unchecked Sendable {
  private let lock = NSLock()
  private var listeners: [UUID: @Sendable (any Sendable) -> Void] = [:]

  public init() {}

  public func publish(_ event: any Sendable) {
    let current = lock.withLock { Array(listeners.values) }
    current.forEach { $0(event) }
  }

  public func subscribe(
    _ listener: @escaping @Sendable (any Sendable) -> Void
  ) -> EventSubscription {
    let id = UUID()
    lock.withLock { listeners[id] = listener }
    return EventSubscription { [weak self] in
      _ = self?.lock.withLock { self?.listeners.removeValue(forKey: id) }
    }
  }
}

public final class EventSubscription: @unchecked Sendable {
  private let cancelHandler: @Sendable () -> Void

  fileprivate init(_ cancelHandler: @escaping @Sendable () -> Void) {
    self.cancelHandler = cancelHandler
  }

  deinit { cancelHandler() }
}
