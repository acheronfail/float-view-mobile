import Foundation

public struct DeviceID: Hashable, Sendable, Codable {
  public let value: String
  public init(_ value: String) {
    self.value = value
  }
}

public struct SessionID: Hashable, Sendable, Codable {
  public let value: String
  public init(_ value: String) {
    self.value = value
  }
}

public struct JobID: Hashable, Sendable, Codable {
  public let value: String
  public init(_ value: String) {
    self.value = value
  }
}

public enum ConnectionState: String, Sendable, Codable {
  case disconnected
  case connecting
  case connected
  case discoveringServices
  case enablingNotifications
  case identifying
  case selectingController = "selecting_controller"
  case ready
  case reconnecting
  case disconnecting
  case failed
}

public enum TrackingMode: String, Sendable, Codable {
  case saved
  case guest
}

public struct ControllerCandidate: Sendable, Equatable {
  public let canID: UInt8
  public let hardwareName: String?
  public let firmwareName: String?
  public let vescVersion: String
  public let vescSupported: Bool

  public init(
    canID: UInt8,
    hardwareName: String?,
    firmwareName: String?,
    vescVersion: String,
    vescSupported: Bool
  ) {
    self.canID = canID
    self.hardwareName = hardwareName
    self.firmwareName = firmwareName
    self.vescVersion = vescVersion
    self.vescSupported = vescSupported
  }
}

public enum ConnectionIssueCode: String, Sendable, Codable {
  case transportSetupFailed = "transport_setup_failed"
  case identificationTimeout = "identification_timeout"
  case noControllerFound = "no_controller_found"
  case tooManyCanDevices = "too_many_can_devices"
  case tooManyControllers = "too_many_controllers"
  case preferredControllerUnavailable = "preferred_controller_unavailable"
  case controllerChanged = "controller_changed"
  case unsupportedController = "unsupported_controller"
  case unsupportedRefloat = "unsupported_refloat"
}

public struct ConnectionIssue: Sendable, Equatable {
  public let code: ConnectionIssueCode
  public let message: String

  public init(code: ConnectionIssueCode, message: String) {
    self.code = code
    self.message = message
  }
}

public enum RideState: String, Sendable, Codable {
  case idle
  case riding
  case unknown
}

public enum PackageState: String, Sendable, Codable, CaseIterable {
  case disabled, startup, ready, running, unknown
}

public enum PackageMode: String, Sendable, Codable, CaseIterable {
  case normal, handTest, flywheel, unknown
}

public enum FootpadState: String, Sendable, Codable, CaseIterable {
  case none, left, right, both, unknown
}

public enum StopCondition: String, Sendable, Codable, CaseIterable {
  case none, pitch, roll, switchHalf, switchFull, reverseStop, quickStop, unknown
}

public enum SetpointAdjustment: String, Sendable, Codable, CaseIterable {
  case none, centering, reverseStop, speed, duty, error
  case highVoltage, lowVoltage, temperature, unknown
}

public enum AlertReason: String, Sendable, Codable, CaseIterable {
  case none, lowVoltage, highVoltage, controllerTemperature, motorTemperature
  case current, duty, sensors, lowBattery, idle, error, speed
  case cellUnderTemperature, cellOverTemperature, cellLowVoltage, cellHighVoltage
  case cellImbalance, bmsConnection, bmsOverTemperature, firmwareFault, unknown
}

public enum FirmwareFault: String, Sendable, Codable, CaseIterable {
  case overVoltage, underVoltage, driver, absoluteOverCurrent
  case controllerOverTemperature, motorOverTemperature
  case gateDriverOverVoltage, gateDriverUnderVoltage, mcuUnderVoltage, watchdogReset
  case encoderSpi, encoderSinCosBelowMinimum, encoderSinCosAboveMaximum
  case flashCorruption, currentSensorOffset1, currentSensorOffset2, currentSensorOffset3
  case unbalancedCurrents, brake, resolverLossOfTracking, resolverDegradationOfSignal
  case resolverLossOfSignal, appConfigFlashCorruption, motorConfigFlashCorruption
  case encoderNoMagnet, encoderMagnetTooStrong, phaseFilter, unknown
}

public enum RefloatTelemetryKind: Sendable, Equatable {
  case realtime
  case compact
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
  public let savedDeviceID: String?

  public init(deviceID: DeviceID, name: String?, rssi: Int?, lastSeenAt: Date, savedDeviceID: String? = nil) {
    self.deviceID = deviceID
    self.name = name
    self.rssi = rssi
    self.lastSeenAt = lastSeenAt
    self.savedDeviceID = savedDeviceID
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

public struct LiveLocation: Sendable, Codable, Equatable {
  public let latitude: Double
  public let longitude: Double
  public let accuracyM: Double?
  public let observedAt: Date

  public init(latitude: Double, longitude: Double, accuracyM: Double?, observedAt: Date) {
    self.latitude = latitude
    self.longitude = longitude
    self.accuracyM = accuracyM
    self.observedAt = observedAt
  }
}

public enum BmsConnection: Sendable, Equatable, Hashable {
  case direct
  case can(UInt8)
}

public enum BmsStatus: String, Sendable, Codable, Equatable {
  case discovering
  case notFound = "not_found"
  case attached
  case unavailable
  case ambiguous
}

public struct BmsCandidate: Sendable, Equatable {
  public let connection: BmsConnection
  public let hardwareName: String?
  public let vescVersion: String

  public init(connection: BmsConnection, hardwareName: String?, vescVersion: String) {
    self.connection = connection
    self.hardwareName = hardwareName
    self.vescVersion = vescVersion
  }
}

public struct BmsSnapshot: Sendable, Equatable {
  public let observedAt: Date
  public let totalVoltageV: Double
  public let chargeVoltageV: Double
  public let inputCurrentA: Double
  public let inputICCurrentA: Double
  public let sessionAmpHoursAh: Double
  public let sessionWattHoursWh: Double
  public let cellVoltagesV: [Double]
  public let balancing: [Bool]
  public let temperaturesC: [Double]
  public let icTemperatureC: Double
  public let humiditySensorTemperatureC: Double
  public let humidityPercent: Double
  public let highestCellTemperatureC: Double
  public let stateOfCharge: Double
  public let stateOfHealth: Double
  public let reportedCanID: UInt8
  public let lifetimeChargeAmpHoursAh: Double
  public let lifetimeChargeWattHoursWh: Double
  public let lifetimeDischargeAmpHoursAh: Double
  public let lifetimeDischargeWattHoursWh: Double
  public let pressurePa: Double
  public let dataVersion: UInt8?
  public let status: String?

  public init(
    observedAt: Date, totalVoltageV: Double, chargeVoltageV: Double,
    inputCurrentA: Double, inputICCurrentA: Double, sessionAmpHoursAh: Double,
    sessionWattHoursWh: Double, cellVoltagesV: [Double], balancing: [Bool],
    temperaturesC: [Double], icTemperatureC: Double, humiditySensorTemperatureC: Double,
    humidityPercent: Double, highestCellTemperatureC: Double, stateOfCharge: Double,
    stateOfHealth: Double, reportedCanID: UInt8, lifetimeChargeAmpHoursAh: Double,
    lifetimeChargeWattHoursWh: Double, lifetimeDischargeAmpHoursAh: Double,
    lifetimeDischargeWattHoursWh: Double, pressurePa: Double, dataVersion: UInt8?,
    status: String?
  ) {
    self.observedAt = observedAt
    self.totalVoltageV = totalVoltageV
    self.chargeVoltageV = chargeVoltageV
    self.inputCurrentA = inputCurrentA
    self.inputICCurrentA = inputICCurrentA
    self.sessionAmpHoursAh = sessionAmpHoursAh
    self.sessionWattHoursWh = sessionWattHoursWh
    self.cellVoltagesV = cellVoltagesV
    self.balancing = balancing
    self.temperaturesC = temperaturesC
    self.icTemperatureC = icTemperatureC
    self.humiditySensorTemperatureC = humiditySensorTemperatureC
    self.humidityPercent = humidityPercent
    self.highestCellTemperatureC = highestCellTemperatureC
    self.stateOfCharge = stateOfCharge
    self.stateOfHealth = stateOfHealth
    self.reportedCanID = reportedCanID
    self.lifetimeChargeAmpHoursAh = lifetimeChargeAmpHoursAh
    self.lifetimeChargeWattHoursWh = lifetimeChargeWattHoursWh
    self.lifetimeDischargeAmpHoursAh = lifetimeDischargeAmpHoursAh
    self.lifetimeDischargeWattHoursWh = lifetimeDischargeWattHoursWh
    self.pressurePa = pressurePa
    self.dataVersion = dataVersion
    self.status = status
  }
}

public struct BmsState: Sendable, Equatable {
  public let status: BmsStatus
  public let connection: BmsConnection?
  public let candidates: [BmsCandidate]
  public let latestSnapshot: BmsSnapshot?

  public init(
    status: BmsStatus = .discovering,
    connection: BmsConnection? = nil,
    candidates: [BmsCandidate] = [],
    latestSnapshot: BmsSnapshot? = nil
  ) {
    self.status = status
    self.connection = connection
    self.candidates = Array(candidates.prefix(16))
    self.latestSnapshot = latestSnapshot
  }
}

public struct ActiveConnectionSnapshot: Sendable, Equatable {
  public let deviceID: DeviceID
  public let name: String?
  public let state: ConnectionState
  public let deviceKind: DeviceKind?
  public let trackingMode: TrackingMode
  public let savedDevice: SavedDeviceRecord?
  public let activeSession: SessionSummaryRecord?
  public let latestSample: TelemetrySample?
  public let latestLocation: LiveLocation?
  public let currentLimits: ControllerCurrentLimits?
  public let controllerRoute: ControllerRoute?
  public let controllerCandidates: [ControllerCandidate]
  public let connectionIssue: ConnectionIssue?
  public let bms: BmsState
  public let refloatControls: RefloatControls?
  public let controllerActions: ControllerActions

  public init(
    deviceID: DeviceID,
    name: String?,
    state: ConnectionState,
    deviceKind: DeviceKind? = nil,
    trackingMode: TrackingMode = .saved,
    savedDevice: SavedDeviceRecord? = nil,
    activeSession: SessionSummaryRecord? = nil,
    latestSample: TelemetrySample? = nil,
    latestLocation: LiveLocation? = nil,
    currentLimits: ControllerCurrentLimits? = nil,
    controllerRoute: ControllerRoute? = nil,
    controllerCandidates: [ControllerCandidate] = [],
    connectionIssue: ConnectionIssue? = nil,
    bms: BmsState = .init(),
    refloatControls: RefloatControls? = nil,
    controllerActions: ControllerActions = .init()
  ) {
    self.deviceID = deviceID
    self.name = name
    self.state = state
    self.deviceKind = deviceKind
    self.trackingMode = trackingMode
    self.savedDevice = savedDevice
    self.activeSession = activeSession
    self.latestSample = latestSample
    self.latestLocation = latestLocation
    self.currentLimits = currentLimits
    self.controllerRoute = controllerRoute
    self.controllerCandidates = controllerCandidates
    self.connectionIssue = connectionIssue
    self.bms = bms
    self.refloatControls = refloatControls
    self.controllerActions = controllerActions
  }
}

public enum DeviceKind: String, Sendable, Equatable {
  case board
  case bms
}

public struct ControllerCurrentLimits: Sendable, Equatable {
  public let motorCurrentMaxA: Double
  public let motorCurrentMinA: Double
  public let batteryCurrentMaxA: Double
  public let batteryCurrentMinA: Double
  public let batterySeriesCells: UInt8?

  public init(
    motorCurrentMaxA: Double,
    motorCurrentMinA: Double,
    batteryCurrentMaxA: Double,
    batteryCurrentMinA: Double,
    batterySeriesCells: UInt8? = nil
  ) {
    self.motorCurrentMaxA = motorCurrentMaxA
    self.motorCurrentMinA = motorCurrentMinA
    self.batteryCurrentMaxA = batteryCurrentMaxA
    self.batteryCurrentMinA = batteryCurrentMinA
    self.batterySeriesCells = batterySeriesCells
  }
}

public struct FirmwareRecord: Sendable, Codable, Equatable {
  public let major: UInt8
  public let minor: UInt8
  public let testVersion: UInt8
  public let hardwareName: String?
  public let firmwareName: String?
  public let hardwareType: UInt8
  public let canID: UInt8?
  /// Private controller identity. Never serialize this into API or log payloads.
  public let controllerUUID: Data?
  /// Private controller capability. Never serialize into API or log payloads.
  public let customConfigurationCount: UInt8
  /// Private cache hint. Never serialize into API or use alone for compatibility.
  public let hardwareConfigurationCRC: UInt32?

  public init(
    major: UInt8,
    minor: UInt8,
    testVersion: UInt8 = 0,
    hardwareName: String?,
    firmwareName: String?,
    hardwareType: UInt8 = 0,
    canID: UInt8? = nil,
    controllerUUID: Data? = nil,
    customConfigurationCount: UInt8 = 0,
    hardwareConfigurationCRC: UInt32? = nil
  ) {
    self.major = major
    self.minor = minor
    self.testVersion = testVersion
    self.hardwareName = hardwareName
    self.firmwareName = firmwareName
    self.hardwareType = hardwareType
    self.canID = canID
    self.controllerUUID = controllerUUID
    self.customConfigurationCount = customConfigurationCount
    self.hardwareConfigurationCRC = hardwareConfigurationCRC
  }
}

public enum VescProtocolMessage: Sendable, Equatable {
  case firmware(FirmwareRecord)
  case currentLimits(ControllerCurrentLimits)
  case canDevices([UInt8])
  case bmsValuesPayload(Data)
  case refloatInfo(version: String)
  case refloatTelemetryLayout(fields: [String], runtimeFields: [String])
  case refloatTelemetry(RefloatTelemetryRecord)
  case refloatLighting(command: RefloatCommand, state: RefloatLightingState)
  case customConfiguration(VescCustomConfigurationResponse)
}

public struct RefloatTelemetryRecord: Sendable, Equatable {
  public let kind: RefloatTelemetryKind
  public let rideState: RideState
  public let packageState: PackageState?
  public let packageMode: PackageMode?
  public let footpadState: FootpadState?
  public let stopCondition: StopCondition?
  public let setpointAdjustment: SetpointAdjustment?
  public let wheelslip: Bool?
  public let darkride: Bool?
  public let charging: Bool?
  public let alertReason: AlertReason?
  public let firmwareFault: FirmwareFault?
  public let fatalErrorLatched: Bool?
  public let packVoltageV: Double?
  public let dutyCycle: Double?
  public let controllerTempC: Double?
  public let motorTempC: Double?
  public let motorCurrentA: Double?
  public let batteryCurrentA: Double?
  public let erpm: Double?
  public let speedKph: Double?
  public let pitchDeg: Double?
  public let balancePitchDeg: Double?
  public let rollDeg: Double?
  public let directionalMotorCurrentA: Double?
  public let filteredMotorCurrentA: Double?
  public let requestedBalanceCurrentA: Double?
  public let boosterCurrentA: Double?
  public let fieldWeakeningCurrentA: Double?
  public let footpadAdc1V: Double?
  public let footpadAdc2V: Double?
  public let remoteInput: Double?
  public let setpointDeg: Double?
  public let atrSetpointDeg: Double?
  public let brakeTiltSetpointDeg: Double?
  public let torqueTiltSetpointDeg: Double?
  public let turnTiltSetpointDeg: Double?
  public let remoteSetpointDeg: Double?
  public let atrAccelerationDifferenceErpmPerSecond: Double?
  public let atrSpeedBoost: Double?
  public let distanceM: Double?
  public let odometerM: Double?
  public let ampHoursDrawnAh: Double?
  public let ampHoursChargedAh: Double?
  public let wattHoursDrawnWh: Double?
  public let wattHoursChargedWh: Double?
  public let batteryLevel: Double?
  public let chargingCurrentA: Double?
  public let chargingVoltageV: Double?
  public let extensions: [String: Double]

  public init(
    kind: RefloatTelemetryKind = .realtime,
    rideState: RideState,
    packageState: PackageState? = nil,
    packageMode: PackageMode? = nil,
    footpadState: FootpadState? = nil,
    stopCondition: StopCondition? = nil,
    setpointAdjustment: SetpointAdjustment? = nil,
    wheelslip: Bool? = nil,
    darkride: Bool? = nil,
    charging: Bool? = nil,
    alertReason: AlertReason? = nil,
    firmwareFault: FirmwareFault? = nil,
    fatalErrorLatched: Bool? = nil,
    packVoltageV: Double? = nil,
    dutyCycle: Double? = nil,
    controllerTempC: Double? = nil,
    motorTempC: Double? = nil,
    motorCurrentA: Double? = nil,
    batteryCurrentA: Double? = nil,
    erpm: Double? = nil,
    speedKph: Double? = nil,
    pitchDeg: Double? = nil,
    balancePitchDeg: Double? = nil,
    rollDeg: Double? = nil,
    directionalMotorCurrentA: Double? = nil,
    filteredMotorCurrentA: Double? = nil,
    requestedBalanceCurrentA: Double? = nil,
    boosterCurrentA: Double? = nil,
    fieldWeakeningCurrentA: Double? = nil,
    footpadAdc1V: Double? = nil,
    footpadAdc2V: Double? = nil,
    remoteInput: Double? = nil,
    setpointDeg: Double? = nil,
    atrSetpointDeg: Double? = nil,
    brakeTiltSetpointDeg: Double? = nil,
    torqueTiltSetpointDeg: Double? = nil,
    turnTiltSetpointDeg: Double? = nil,
    remoteSetpointDeg: Double? = nil,
    atrAccelerationDifferenceErpmPerSecond: Double? = nil,
    atrSpeedBoost: Double? = nil,
    distanceM: Double? = nil,
    odometerM: Double? = nil,
    ampHoursDrawnAh: Double? = nil,
    ampHoursChargedAh: Double? = nil,
    wattHoursDrawnWh: Double? = nil,
    wattHoursChargedWh: Double? = nil,
    batteryLevel: Double? = nil,
    chargingCurrentA: Double? = nil,
    chargingVoltageV: Double? = nil,
    extensions: [String: Double] = [:]
  ) {
    self.kind = kind
    self.rideState = rideState
    self.packageState = packageState
    self.packageMode = packageMode
    self.footpadState = footpadState
    self.stopCondition = stopCondition
    self.setpointAdjustment = setpointAdjustment
    self.wheelslip = wheelslip
    self.darkride = darkride
    self.charging = charging
    self.alertReason = alertReason
    self.firmwareFault = firmwareFault
    self.fatalErrorLatched = fatalErrorLatched
    self.packVoltageV = packVoltageV
    self.dutyCycle = dutyCycle
    self.controllerTempC = controllerTempC
    self.motorTempC = motorTempC
    self.motorCurrentA = motorCurrentA
    self.batteryCurrentA = batteryCurrentA
    self.erpm = erpm
    self.speedKph = speedKph
    self.pitchDeg = pitchDeg
    self.balancePitchDeg = balancePitchDeg
    self.rollDeg = rollDeg
    self.directionalMotorCurrentA = directionalMotorCurrentA
    self.filteredMotorCurrentA = filteredMotorCurrentA
    self.requestedBalanceCurrentA = requestedBalanceCurrentA
    self.boosterCurrentA = boosterCurrentA
    self.fieldWeakeningCurrentA = fieldWeakeningCurrentA
    self.footpadAdc1V = footpadAdc1V
    self.footpadAdc2V = footpadAdc2V
    self.remoteInput = remoteInput
    self.setpointDeg = setpointDeg
    self.atrSetpointDeg = atrSetpointDeg
    self.brakeTiltSetpointDeg = brakeTiltSetpointDeg
    self.torqueTiltSetpointDeg = torqueTiltSetpointDeg
    self.turnTiltSetpointDeg = turnTiltSetpointDeg
    self.remoteSetpointDeg = remoteSetpointDeg
    self.atrAccelerationDifferenceErpmPerSecond = atrAccelerationDifferenceErpmPerSecond
    self.atrSpeedBoost = atrSpeedBoost
    self.distanceM = distanceM
    self.odometerM = odometerM
    self.ampHoursDrawnAh = ampHoursDrawnAh
    self.ampHoursChargedAh = ampHoursChargedAh
    self.wattHoursDrawnWh = wattHoursDrawnWh
    self.wattHoursChargedWh = wattHoursChargedWh
    self.batteryLevel = batteryLevel
    self.chargingCurrentA = chargingCurrentA
    self.chargingVoltageV = chargingVoltageV
    self.extensions = extensions
  }
}

public struct TelemetrySample: Sendable, Codable, Equatable {
  public let sequence: Int64
  public let observedAt: Date
  public let slowObservedAt: Date?
  public let elapsedMs: Int64
  public let rideState: RideState
  public let packageState: PackageState?
  public let packageMode: PackageMode?
  public let footpadState: FootpadState?
  public let stopCondition: StopCondition?
  public let setpointAdjustment: SetpointAdjustment?
  public let wheelslip: Bool?
  public let darkride: Bool?
  public let charging: Bool?
  public let alertReason: AlertReason?
  public let firmwareFault: FirmwareFault?
  public let packVoltageV: Double?
  public let dutyCycle: Double?
  public let controllerTempC: Double?
  public let motorTempC: Double?
  public let motorCurrentA: Double?
  public let batteryCurrentA: Double?
  public let erpm: Double?
  public let speedKph: Double?
  public let pitchDeg: Double?
  public let balancePitchDeg: Double?
  public let rollDeg: Double?
  public let directionalMotorCurrentA: Double?
  public let filteredMotorCurrentA: Double?
  public let requestedBalanceCurrentA: Double?
  public let boosterCurrentA: Double?
  public let fieldWeakeningCurrentA: Double?
  public let footpadAdc1V: Double?
  public let footpadAdc2V: Double?
  public let remoteInput: Double?
  public let setpointDeg: Double?
  public let atrSetpointDeg: Double?
  public let brakeTiltSetpointDeg: Double?
  public let torqueTiltSetpointDeg: Double?
  public let turnTiltSetpointDeg: Double?
  public let remoteSetpointDeg: Double?
  public let atrAccelerationDifferenceErpmPerSecond: Double?
  public let atrSpeedBoost: Double?
  public let distanceM: Double?
  public let odometerM: Double?
  public let ampHoursDrawnAh: Double?
  public let ampHoursChargedAh: Double?
  public let wattHoursDrawnWh: Double?
  public let wattHoursChargedWh: Double?
  public let batteryLevel: Double?
  public let chargingCurrentA: Double?
  public let chargingVoltageV: Double?
  public let extensionFields: [String: Double]

  public init(
    sequence: Int64,
    observedAt: Date,
    slowObservedAt: Date? = nil,
    elapsedMs: Int64,
    rideState: RideState,
    packageState: PackageState? = nil,
    packageMode: PackageMode? = nil,
    footpadState: FootpadState? = nil,
    stopCondition: StopCondition? = nil,
    setpointAdjustment: SetpointAdjustment? = nil,
    wheelslip: Bool? = nil,
    darkride: Bool? = nil,
    charging: Bool? = nil,
    alertReason: AlertReason? = nil,
    firmwareFault: FirmwareFault? = nil,
    packVoltageV: Double? = nil,
    dutyCycle: Double? = nil,
    controllerTempC: Double? = nil,
    motorTempC: Double? = nil,
    motorCurrentA: Double? = nil,
    batteryCurrentA: Double? = nil,
    erpm: Double? = nil,
    speedKph: Double? = nil,
    pitchDeg: Double? = nil,
    balancePitchDeg: Double? = nil,
    rollDeg: Double? = nil,
    directionalMotorCurrentA: Double? = nil,
    filteredMotorCurrentA: Double? = nil,
    requestedBalanceCurrentA: Double? = nil,
    boosterCurrentA: Double? = nil,
    fieldWeakeningCurrentA: Double? = nil,
    footpadAdc1V: Double? = nil,
    footpadAdc2V: Double? = nil,
    remoteInput: Double? = nil,
    setpointDeg: Double? = nil,
    atrSetpointDeg: Double? = nil,
    brakeTiltSetpointDeg: Double? = nil,
    torqueTiltSetpointDeg: Double? = nil,
    turnTiltSetpointDeg: Double? = nil,
    remoteSetpointDeg: Double? = nil,
    atrAccelerationDifferenceErpmPerSecond: Double? = nil,
    atrSpeedBoost: Double? = nil,
    distanceM: Double? = nil,
    odometerM: Double? = nil,
    ampHoursDrawnAh: Double? = nil,
    ampHoursChargedAh: Double? = nil,
    wattHoursDrawnWh: Double? = nil,
    wattHoursChargedWh: Double? = nil,
    batteryLevel: Double? = nil,
    chargingCurrentA: Double? = nil,
    chargingVoltageV: Double? = nil,
    extensionFields: [String: Double] = [:]
  ) {
    self.sequence = sequence
    self.observedAt = observedAt
    self.slowObservedAt = slowObservedAt
    self.elapsedMs = elapsedMs
    self.rideState = rideState
    self.packageState = packageState
    self.packageMode = packageMode
    self.footpadState = footpadState
    self.stopCondition = stopCondition
    self.setpointAdjustment = setpointAdjustment
    self.wheelslip = wheelslip
    self.darkride = darkride
    self.charging = charging
    self.alertReason = alertReason
    self.firmwareFault = firmwareFault
    self.packVoltageV = packVoltageV
    self.dutyCycle = dutyCycle
    self.controllerTempC = controllerTempC
    self.motorTempC = motorTempC
    self.motorCurrentA = motorCurrentA
    self.batteryCurrentA = batteryCurrentA
    self.erpm = erpm
    self.speedKph = speedKph
    self.pitchDeg = pitchDeg
    self.balancePitchDeg = balancePitchDeg
    self.rollDeg = rollDeg
    self.directionalMotorCurrentA = directionalMotorCurrentA
    self.filteredMotorCurrentA = filteredMotorCurrentA
    self.requestedBalanceCurrentA = requestedBalanceCurrentA
    self.boosterCurrentA = boosterCurrentA
    self.fieldWeakeningCurrentA = fieldWeakeningCurrentA
    self.footpadAdc1V = footpadAdc1V
    self.footpadAdc2V = footpadAdc2V
    self.remoteInput = remoteInput
    self.setpointDeg = setpointDeg
    self.atrSetpointDeg = atrSetpointDeg
    self.brakeTiltSetpointDeg = brakeTiltSetpointDeg
    self.torqueTiltSetpointDeg = torqueTiltSetpointDeg
    self.turnTiltSetpointDeg = turnTiltSetpointDeg
    self.remoteSetpointDeg = remoteSetpointDeg
    self.atrAccelerationDifferenceErpmPerSecond = atrAccelerationDifferenceErpmPerSecond
    self.atrSpeedBoost = atrSpeedBoost
    self.distanceM = distanceM
    self.odometerM = odometerM
    self.ampHoursDrawnAh = ampHoursDrawnAh
    self.ampHoursChargedAh = ampHoursChargedAh
    self.wattHoursDrawnWh = wattHoursDrawnWh
    self.wattHoursChargedWh = wattHoursChargedWh
    self.batteryLevel = batteryLevel
    self.chargingCurrentA = chargingCurrentA
    self.chargingVoltageV = chargingVoltageV
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

public struct SessionPublishedSnapshotRecord: Sendable, Codable, Equatable {
  public let historyRevision: Int64
  public let committedSampleSequence: Int64
  public let committedEventSequence: Int64
  public let committedElapsedMs: Int64
  public let recordedSampleCount: Int64
  public let storedSampleCount: Int64
  public let committedLocationSequence: Int64
  public let committedLocationElapsedMs: Int64
  public let recordedLocationCount: Int64
  public let storedLocationCount: Int64
  public let locationStatus: String

  public init(
    historyRevision: Int64,
    committedSampleSequence: Int64,
    committedEventSequence: Int64,
    committedElapsedMs: Int64,
    recordedSampleCount: Int64,
    storedSampleCount: Int64,
    committedLocationSequence: Int64 = 0,
    committedLocationElapsedMs: Int64 = 0,
    recordedLocationCount: Int64 = 0,
    storedLocationCount: Int64 = 0,
    locationStatus: String = "disabled"
  ) {
    self.historyRevision = historyRevision
    self.committedSampleSequence = committedSampleSequence
    self.committedEventSequence = committedEventSequence
    self.committedElapsedMs = committedElapsedMs
    self.recordedSampleCount = recordedSampleCount
    self.storedSampleCount = storedSampleCount
    self.committedLocationSequence = committedLocationSequence
    self.committedLocationElapsedMs = committedLocationElapsedMs
    self.recordedLocationCount = recordedLocationCount
    self.storedLocationCount = storedLocationCount
    self.locationStatus = locationStatus
  }
}

public struct SessionLocationPointRecord: Sendable, Codable, Equatable {
  public let locationSequence: Int64
  public let elapsedMs: Int64
  public let latitude: Double
  public let longitude: Double
  public let accuracyM: Double?
  public let observedAt: Date

  public init(
    locationSequence: Int64,
    elapsedMs: Int64,
    latitude: Double,
    longitude: Double,
    accuracyM: Double?,
    observedAt: Date
  ) {
    self.locationSequence = locationSequence
    self.elapsedMs = elapsedMs
    self.latitude = latitude
    self.longitude = longitude
    self.accuracyM = accuracyM
    self.observedAt = observedAt
  }
}

public struct SessionLocationGapRecord: Sendable, Codable, Equatable {
  public let startElapsedMs: Int64
  public let endElapsedMs: Int64
  public let reason: String

  public init(startElapsedMs: Int64, endElapsedMs: Int64, reason: String) {
    self.startElapsedMs = startElapsedMs
    self.endElapsedMs = endElapsedMs
    self.reason = reason
  }
}

public struct SessionLocationPageRecord: Sendable, Codable, Equatable {
  public let points: [SessionLocationPointRecord]
  public let gaps: [SessionLocationGapRecord]
  public let nextCursor: String?
  public let snapshot: SessionPublishedSnapshotRecord

  public init(
    points: [SessionLocationPointRecord],
    gaps: [SessionLocationGapRecord],
    nextCursor: String?,
    snapshot: SessionPublishedSnapshotRecord
  ) {
    self.points = points
    self.gaps = gaps
    self.nextCursor = nextCursor
    self.snapshot = snapshot
  }
}

public struct LocationObservation: Sendable, Codable, Equatable {
  public let observedAt: Date
  public let elapsedMs: Int64
  public let latitude: Double
  public let longitude: Double
  public let accuracyM: Double?

  public init(observedAt: Date, elapsedMs: Int64, latitude: Double, longitude: Double, accuracyM: Double?) {
    self.observedAt = observedAt
    self.elapsedMs = elapsedMs
    self.latitude = latitude
    self.longitude = longitude
    self.accuracyM = accuracyM
  }
}

public enum LocationProviderAuthorization: String, Sendable, Codable {
  case notDetermined
  case foregroundOnly
  case authorized
  case denied
  case restricted
  case unavailable
}

public enum LocationProviderBackgroundCapture: String, Sendable, Codable {
  case available
  case foregroundOnly
  case unavailable
}

public struct LocationProviderState: Sendable, Codable, Equatable {
  public let authorization: LocationProviderAuthorization
  public let backgroundCapture: LocationProviderBackgroundCapture
  public let reason: String?

  public init(
    authorization: LocationProviderAuthorization,
    backgroundCapture: LocationProviderBackgroundCapture,
    reason: String? = nil
  ) {
    self.authorization = authorization
    self.backgroundCapture = backgroundCapture
    self.reason = reason
  }
}

public struct LocationProviderObservation: Sendable, Codable, Equatable {
  public let observedAt: Date
  public let latitude: Double
  public let longitude: Double
  public let accuracyM: Double?

  public init(observedAt: Date, latitude: Double, longitude: Double, accuracyM: Double?) {
    self.observedAt = observedAt
    self.latitude = latitude
    self.longitude = longitude
    self.accuracyM = accuracyM
  }
}

public struct LocationUpdated: Sendable {}

public protocol LocationProviderPort: AnyObject, Sendable {
  func state() -> LocationProviderState
  func requestPermission()
  func openSettings()
  func startUpdates(_ handler: @escaping @Sendable (LocationProviderObservation) -> Void)
  func stopUpdates()
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
  public let sequence: Int64
  public let rideState: String?

  public init(
    elapsedMs: Int64,
    values: [String: Double?],
    sequence: Int64 = 0,
    rideState: String? = nil
  ) {
    self.elapsedMs = elapsedMs
    self.values = values
    self.sequence = sequence
    self.rideState = rideState
  }
}

public struct SessionSeriesReductionRecord: Sendable {
  public let columns: [String: SessionSeriesColumnRecord]
  public let sourceRows: Int
  public let sourceMs: Double
  public let reducerMs: Double
  public let elapsedMs: Double

  public init(
    columns: [String: SessionSeriesColumnRecord],
    sourceRows: Int,
    sourceMs: Double,
    reducerMs: Double,
    elapsedMs: Double
  ) {
    self.columns = columns
    self.sourceRows = sourceRows
    self.sourceMs = sourceMs
    self.reducerMs = reducerMs
    self.elapsedMs = elapsedMs
  }
}

public struct SessionHistoryConfiguration: Sendable, Equatable {
  public let expectedRidingPersistenceMs: Int64
  public let expectedIdlePersistenceMs: Int64
  public let retainedResolutionSeconds: Int64

  public init(
    expectedRidingPersistenceMs: Int64,
    expectedIdlePersistenceMs: Int64,
    retainedResolutionSeconds: Int64 = 0
  ) {
    self.expectedRidingPersistenceMs = expectedRidingPersistenceMs
    self.expectedIdlePersistenceMs = expectedIdlePersistenceMs
    self.retainedResolutionSeconds = retainedResolutionSeconds
  }
}

public struct SessionPruneMetadataRecord: Sendable, Equatable {
  public let targetResolutionSeconds: Int64?
  public let recordedSampleCount: Int64
  public let storedSampleCount: Int64
  public let policy: String?

  public init(
    targetResolutionSeconds: Int64?,
    recordedSampleCount: Int64,
    storedSampleCount: Int64,
    policy: String?
  ) {
    self.targetResolutionSeconds = targetResolutionSeconds
    self.recordedSampleCount = recordedSampleCount
    self.storedSampleCount = storedSampleCount
    self.policy = policy
  }
}

public struct SessionPrunePreviewRecord: Sendable, Equatable {
  public let recordedSampleCount: Int64
  public let estimatedStoredSampleCount: Int64
  public let estimatedReclaimableBytes: Int64?
  public let skipped: Bool
  public let skipReason: String?

  public init(
    recordedSampleCount: Int64,
    estimatedStoredSampleCount: Int64,
    estimatedReclaimableBytes: Int64?,
    skipped: Bool,
    skipReason: String?
  ) {
    self.recordedSampleCount = recordedSampleCount
    self.estimatedStoredSampleCount = estimatedStoredSampleCount
    self.estimatedReclaimableBytes = estimatedReclaimableBytes
    self.skipped = skipped
    self.skipReason = skipReason
  }
}

public struct SessionMaintenanceJobRecord: Sendable, Equatable {
  public let operationID: String
  public let kind: String
  public let scope: String
  public let state: String
  public let sessionID: String?
  public let targetResolutionSeconds: Int64
  public let upperSessionID: String?
  public let lastSessionID: String?
  public let completedSessions: Int64
  public let skippedSessions: Int64
  public let retainedSamples: Int64
  public let reclaimableBytes: Int64
  public let error: String?

  public init(
    operationID: String,
    kind: String,
    scope: String,
    state: String,
    sessionID: String?,
    targetResolutionSeconds: Int64,
    upperSessionID: String?,
    lastSessionID: String?,
    completedSessions: Int64,
    skippedSessions: Int64,
    retainedSamples: Int64,
    reclaimableBytes: Int64,
    error: String?
  ) {
    self.operationID = operationID
    self.kind = kind
    self.scope = scope
    self.state = state
    self.sessionID = sessionID
    self.targetResolutionSeconds = targetResolutionSeconds
    self.upperSessionID = upperSessionID
    self.lastSessionID = lastSessionID
    self.completedSessions = completedSessions
    self.skippedSessions = skippedSessions
    self.retainedSamples = retainedSamples
    self.reclaimableBytes = reclaimableBytes
    self.error = error
  }
}

public struct SessionEventRecord: Sendable, Equatable {
  public let eventID: String
  public let elapsedMs: Int64
  public let kind: String
  public let detailsJSON: String
  public let sourceSampleSequence: Int64?

  public init(
    eventID: String,
    elapsedMs: Int64,
    kind: String,
    detailsJSON: String,
    sourceSampleSequence: Int64? = nil
  ) {
    self.eventID = eventID
    self.elapsedMs = elapsedMs
    self.kind = kind
    self.detailsJSON = detailsJSON
    self.sourceSampleSequence = sourceSampleSequence
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

public enum UnitPreference: String, Sendable, Codable, Equatable, CaseIterable {
  case metric
  case imperial
}

public struct SettingsRecord: Sendable, Codable, Equatable {
  public let revision: Int
  public let idleProbeIntervalMs: Int64
  public let idlePersistIntervalMs: Int64
  public let ridingPersistIntervalMs: Int64
  public let postRideTailMs: Int64
  public let reconnectDurationMs: Int64
  public let minimumSessionDistance: Double
  public let themePreference: ThemePreference
  public let unitPreference: UnitPreference
  public let historyGapThresholdSeconds: Double?
  public let locationRecordingEnabled: Bool
  public let includeGpsLocationWhenExporting: Bool

  public init(
    revision: Int = 0,
    idleProbeIntervalMs: Int64 = 500,
    idlePersistIntervalMs: Int64 = 1_000,
    ridingPersistIntervalMs: Int64 = 100,
    postRideTailMs: Int64 = 5_000,
    reconnectDurationMs: Int64 = 300_000,
    minimumSessionDistance: Double = 100,
    themePreference: ThemePreference = .system,
    unitPreference: UnitPreference = .metric,
    historyGapThresholdSeconds: Double? = nil,
    locationRecordingEnabled: Bool = false,
    includeGpsLocationWhenExporting: Bool = false
  ) {
    self.revision = revision
    self.idleProbeIntervalMs = idleProbeIntervalMs
    self.idlePersistIntervalMs = idlePersistIntervalMs
    self.ridingPersistIntervalMs = ridingPersistIntervalMs
    self.postRideTailMs = postRideTailMs
    self.reconnectDurationMs = reconnectDurationMs
    self.minimumSessionDistance = minimumSessionDistance
    self.themePreference = themePreference
    self.unitPreference = unitPreference
    self.historyGapThresholdSeconds = historyGapThresholdSeconds
    self.locationRecordingEnabled = locationRecordingEnabled
    self.includeGpsLocationWhenExporting = includeGpsLocationWhenExporting
  }
}

public struct BoardHistorySummaryRecord: Sendable, Codable, Equatable {
  public let sessionCount: Int
  public let totalRidingElapsedMs: Int64
  public let totalDistanceM: Double?
  public let mostRecentSessionAt: Date?

  public init(
    sessionCount: Int = 0,
    totalRidingElapsedMs: Int64 = 0,
    totalDistanceM: Double? = nil,
    mostRecentSessionAt: Date? = nil
  ) {
    self.sessionCount = sessionCount
    self.totalRidingElapsedMs = totalRidingElapsedMs
    self.totalDistanceM = totalDistanceM
    self.mostRecentSessionAt = mostRecentSessionAt
  }
}

public struct SavedDeviceRecord: Sendable, Codable, Equatable {
  public let savedDeviceID: String
  public let deviceHistoryID: String
  public let revision: Int
  public let displayName: String
  public let customDisplayName: String?
  public let advertisedName: String?
  public let bluetoothIdentifier: String?
  public let lastSeenAt: Date?
  public let lastConnectedAt: Date?
  public let profileComplete: Bool
  public let summary: BoardHistorySummaryRecord

  public init(
    savedDeviceID: String,
    deviceHistoryID: String,
    revision: Int = 0,
    displayName: String? = nil,
    customDisplayName: String? = nil,
    advertisedName: String? = nil,
    bluetoothIdentifier: String? = nil,
    lastSeenAt: Date? = nil,
    lastConnectedAt: Date? = nil,
    profileComplete: Bool,
    summary: BoardHistorySummaryRecord = .init()
  ) {
    self.savedDeviceID = savedDeviceID
    self.deviceHistoryID = deviceHistoryID
    self.revision = revision
    self.customDisplayName = customDisplayName
    self.advertisedName = advertisedName
    self.bluetoothIdentifier = bluetoothIdentifier
    self.displayName = displayName ?? Self.resolveDisplayName(
      customDisplayName: customDisplayName,
      advertisedName: advertisedName,
      bluetoothIdentifier: bluetoothIdentifier
    )
    self.lastSeenAt = lastSeenAt
    self.lastConnectedAt = lastConnectedAt
    self.profileComplete = profileComplete
    self.summary = summary
  }

  public static func resolveDisplayName(
    customDisplayName: String?,
    advertisedName: String?,
    bluetoothIdentifier: String?
  ) -> String {
    for candidate in [customDisplayName, advertisedName] {
      if let value = candidate?.trimmingCharacters(in: .whitespacesAndNewlines),
         !value.isEmpty
      {
        return value
      }
    }
    return bluetoothIdentifier ?? "Board"
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

public struct ControllerRoutingHint: Sendable, Equatable {
  public let savedDeviceID: String
  public let canID: UInt8
  /// Private 12-byte controller identity, if the firmware supplied one.
  public let controllerUUID: Data?

  public init(savedDeviceID: String, canID: UInt8, controllerUUID: Data?) {
    self.savedDeviceID = savedDeviceID
    self.canID = canID
    self.controllerUUID = controllerUUID
  }
}

public enum ProductStoreError: Error, Equatable {
  case savedDeviceNotFound
  case savedDeviceRevisionConflict(currentRevision: Int)
  case invalidSavedDevicePatch
  case profileRevisionConflict(currentRevision: Int)
  case invalidProfilePatch
  case settingsRevisionConflict(currentRevision: Int)
  case invalidSettingsPatch
  case savedDeviceActive
  case refloatConfigurationNotFound
  case refloatConfigurationRevisionConflict(currentRevision: Int)
  case invalidRefloatConfiguration
}

public struct RefloatConfigurationIdentityRecord: Equatable, Sendable {
  public let packageName: String
  public let packageVersion: String
  public let schemaFingerprint: String
  public let wireSignature: UInt32

  public init(packageName: String, packageVersion: String, schemaFingerprint: String, wireSignature: UInt32) {
    self.packageName = packageName
    self.packageVersion = packageVersion
    self.schemaFingerprint = schemaFingerprint
    self.wireSignature = wireSignature
  }
}

public struct RefloatConfigurationRecord: Equatable, Sendable {
  public let configurationID: String
  public let revision: Int
  public let name: String
  public let description: String?
  public let identity: RefloatConfigurationIdentityRecord
  public let valuesJSON: String
  public let createdAt: Date
  public let updatedAt: Date
}

public struct RefloatConfigurationListPage: Equatable, Sendable {
  public let items: [RefloatConfigurationRecord]
  public let nextCursor: String?
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
  func configureSavedDeviceIDs(_ deviceIDs: Set<DeviceID>)
}

public extension BluetoothPort {
  func configureSavedDeviceIDs(_: Set<DeviceID>) {}
}

public protocol VescProtocolPort: AnyObject, Sendable {
  var supportsBmsTelemetry: Bool { get }
  func reset()
  func firmwareRequest(route: ControllerRoute) throws -> Data
  func motorConfigurationRequest(route: ControllerRoute) throws -> Data
  func canPingRequest() throws -> Data
  func controllerShutdownRequest(route: ControllerRoute) throws -> Data
  func refloatInfoRequest(route: ControllerRoute) throws -> Data
  func refloatTelemetryIDsRequest(route: ControllerRoute) throws -> Data
  func refloatTelemetryRequest(route: ControllerRoute) throws -> Data
  func refloatLegacyTelemetryRequest(route: ControllerRoute) throws -> Data
  func refloatCompactTelemetryRequest(route: ControllerRoute) throws -> Data
  func customConfigurationSchemaRequest(route: ControllerRoute, length: Int, offset: Int) throws -> Data
  func customConfigurationValuesRequest(route: ControllerRoute, defaults: Bool) throws -> Data
  func customConfigurationWriteRequest(route: ControllerRoute, values: Data) throws -> Data
  func bmsValuesRequest(connection: BmsConnection) throws -> Data
  func receive(_ bytes: Data) throws -> [VescProtocolMessage]
}

public extension VescProtocolPort {
  var supportsBmsTelemetry: Bool {
    false
  }

  func bmsValuesRequest(connection _: BmsConnection) throws -> Data {
    throw VescProtocolError.unsupportedCommand(VescCommand.bmsGetValues.rawValue)
  }

  func motorConfigurationRequest(route _: ControllerRoute) throws -> Data {
    throw VescProtocolError.unsupportedCommand(VescCommand.getMotorConfiguration.rawValue)
  }

  func controllerShutdownRequest(route _: ControllerRoute) throws -> Data {
    throw VescProtocolError.unsupportedCommand(VescCommand.shutdown.rawValue)
  }

  func refloatLegacyTelemetryRequest(route: ControllerRoute) throws -> Data {
    try refloatTelemetryRequest(route: route)
  }

  func refloatCompactTelemetryRequest(route: ControllerRoute) throws -> Data {
    try refloatLegacyTelemetryRequest(route: route)
  }

  func customConfigurationSchemaRequest(route _: ControllerRoute, length _: Int, offset _: Int) throws -> Data {
    throw VescProtocolError.unsupportedCommand(VescCommand.getCustomConfigurationXML.rawValue)
  }

  func customConfigurationValuesRequest(route _: ControllerRoute, defaults: Bool) throws -> Data {
    throw VescProtocolError.unsupportedCommand(
      defaults ? VescCommand.getCustomConfigurationDefault.rawValue : VescCommand.getCustomConfiguration.rawValue
    )
  }

  func customConfigurationWriteRequest(route _: ControllerRoute, values _: Data) throws -> Data {
    throw VescProtocolError.unsupportedCommand(VescCommand.setCustomConfiguration.rawValue)
  }
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
  func create(deviceID: DeviceID, advertisedName: String?, at: Date) throws -> SessionRecord
  func append(jobID: JobID, samples: [TelemetrySample]) throws
  func appendLocations(jobID: JobID, observations: [LocationObservation]) throws
  func close(jobID: JobID, at: Date, reason: String) throws -> SessionRecord
  func active() -> SessionRecord?
}

public extension LogStorePort {
  func appendLocations(jobID _: JobID, observations _: [LocationObservation]) throws {}
}

public protocol ProductStorePort: AnyObject, Sendable, RefloatSchemaCachePort {
  func ensureSavedDevice(
    deviceID: DeviceID,
    advertisedName: String?,
    at: Date
  ) throws -> SavedDeviceRecord
  func ensureSavedDevice(
    deviceID: DeviceID,
    advertisedName: String?,
    stableIdentity: String?,
    at: Date
  ) throws -> SavedDeviceRecord
  func savedDevices() throws -> [SavedDeviceRecord]
  func updateSavedDevice(
    savedDeviceID: String,
    expectedRevision: Int,
    customDisplayName: String?
  ) throws -> SavedDeviceRecord
  func boardProfile(savedDeviceID: String) throws -> BoardProfileRecord
  func updateBoardProfile(
    savedDeviceID: String,
    expectedRevision: Int,
    patchJSON: String,
    at: Date
  ) throws -> BoardProfileRecord
  func selectController(
    savedDeviceID: String,
    canID: UInt8,
    controllerUUID: Data?,
    at: Date
  ) throws -> BoardProfileRecord
  func controllerRoutingHint(savedDeviceID: String) throws -> ControllerRoutingHint?
  func storeControllerRoutingHint(_ hint: ControllerRoutingHint, at: Date) throws
  func clearControllerRoutingHint(savedDeviceID: String) throws
  func deleteSavedDevice(savedDeviceID: String) throws
  func ensureImportedSavedDevice(
    deviceHistoryID: String,
    displayName: String?,
    profileSnapshotJSON: String,
    at: Date
  ) throws -> SavedDeviceRecord
  func settings() throws -> SettingsRecord
  func updateSettings(
    expectedRevision: Int,
    patchJSON: String
  ) throws -> SettingsRecord
  func refloatConfigurations(
    identity: RefloatConfigurationIdentityRecord, limit: Int, cursor: String?
  ) throws -> RefloatConfigurationListPage
  func refloatConfiguration(configurationID: String) throws -> RefloatConfigurationRecord
  func createRefloatConfiguration(
    name: String, description: String?, identity: RefloatConfigurationIdentityRecord,
    valuesJSON: String, at: Date
  ) throws -> RefloatConfigurationRecord
  func updateRefloatConfiguration(
    configurationID: String, expectedRevision: Int, name: String,
    description: String?, valuesJSON: String, at: Date
  ) throws -> RefloatConfigurationRecord
  func deleteRefloatConfiguration(configurationID: String, expectedRevision: Int) throws
}

public extension ProductStorePort {
  func refloatConfigurations(
    identity _: RefloatConfigurationIdentityRecord, limit _: Int, cursor _: String?
  ) throws -> RefloatConfigurationListPage {
    throw ProductStoreError.invalidRefloatConfiguration
  }

  func refloatConfiguration(configurationID _: String) throws -> RefloatConfigurationRecord {
    throw ProductStoreError.refloatConfigurationNotFound
  }

  func createRefloatConfiguration(
    name _: String, description _: String?, identity _: RefloatConfigurationIdentityRecord,
    valuesJSON _: String, at _: Date
  ) throws -> RefloatConfigurationRecord {
    throw ProductStoreError.invalidRefloatConfiguration
  }

  func updateRefloatConfiguration(
    configurationID _: String, expectedRevision _: Int, name _: String,
    description _: String?, valuesJSON _: String, at _: Date
  ) throws -> RefloatConfigurationRecord {
    throw ProductStoreError.invalidRefloatConfiguration
  }

  func deleteRefloatConfiguration(configurationID _: String, expectedRevision _: Int) throws {
    throw ProductStoreError.refloatConfigurationNotFound
  }

  func ensureSavedDevice(
    deviceID _: DeviceID,
    advertisedName _: String?,
    at _: Date
  ) throws -> SavedDeviceRecord {
    throw ProductStoreError.savedDeviceNotFound
  }

  func updateSavedDevice(
    savedDeviceID _: String,
    expectedRevision _: Int,
    customDisplayName _: String?
  ) throws -> SavedDeviceRecord {
    throw ProductStoreError.invalidSavedDevicePatch
  }

  func ensureSavedDevice(
    deviceID: DeviceID,
    advertisedName: String?,
    stableIdentity _: String?,
    at: Date
  ) throws -> SavedDeviceRecord {
    try ensureSavedDevice(deviceID: deviceID, advertisedName: advertisedName, at: at)
  }

  func deleteSavedDevice(savedDeviceID _: String) throws {
    throw ProductStoreError.savedDeviceNotFound
  }

  func ensureImportedSavedDevice(
    deviceHistoryID _: String,
    displayName _: String?,
    profileSnapshotJSON _: String,
    at _: Date
  ) throws -> SavedDeviceRecord {
    throw ProductStoreError.invalidSavedDevicePatch
  }

  func settings() throws -> SettingsRecord {
    SettingsRecord()
  }

  func selectController(
    savedDeviceID: String,
    canID: UInt8,
    controllerUUID _: Data?,
    at: Date
  ) throws -> BoardProfileRecord {
    let current = try boardProfile(savedDeviceID: savedDeviceID)
    return try updateBoardProfile(
      savedDeviceID: savedDeviceID,
      expectedRevision: current.revision,
      patchJSON: "{\"preferredCanId\":\(canID)}",
      at: at
    )
  }

  func controllerRoutingHint(savedDeviceID _: String) throws -> ControllerRoutingHint? {
    nil
  }

  func storeControllerRoutingHint(_: ControllerRoutingHint, at _: Date) throws {}
  func clearControllerRoutingHint(savedDeviceID _: String) throws {}
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
  case maintenanceInProgress
  case invalidCursor
  case invalidRange
  case unsupportedField(String)
  case invalidExportFormat
  case invalidArchive
}

/// Cancellation owned by one session-data request. Storage adapters register
/// only the native work started by that request, so sibling requests for the
/// same session remain independent.
public final class SessionReadCancellation: @unchecked Sendable {
  private let lock = NSLock()
  private var callbacks: [UUID: @Sendable () -> Void] = [:]
  private var cancelled = false

  public init() {}

  public func cancel() {
    let pending = lock.withLock { () -> [@Sendable () -> Void] in
      guard !cancelled else { return [] }
      cancelled = true
      defer { callbacks.removeAll() }
      return Array(callbacks.values)
    }
    pending.forEach { $0() }
  }

  public func register(_ callback: @escaping @Sendable () -> Void) -> UUID? {
    let registration = UUID()
    let invokeImmediately = lock.withLock {
      guard !cancelled else { return true }
      callbacks[registration] = callback
      return false
    }
    if invokeImmediately {
      callback()
      return nil
    }
    return registration
  }

  public func unregister(_ registration: UUID?) {
    guard let registration else { return }
    _ = lock.withLock { callbacks.removeValue(forKey: registration) }
  }
}

public protocol SessionStorePort: AnyObject, Sendable {
  func sessions(_ query: SessionListQuery) throws -> SessionPage
  func session(sessionID: String) throws -> SessionSummaryRecord
  func sessionSnapshot(sessionID: String) throws -> SessionPublishedSnapshotRecord
  func sessionHistoryConfiguration(sessionID: String) throws -> SessionHistoryConfiguration
  func sessionPruneMetadata(sessionID: String) throws -> SessionPruneMetadataRecord
  func prunePreview(sessionID: String, targetResolutionSeconds: Int64) throws -> SessionPrunePreviewRecord
  func pruneSession(sessionID: String, targetResolutionSeconds: Int64) throws -> SessionPrunePreviewRecord
  func maintenanceJob() throws -> SessionMaintenanceJobRecord?
  func saveMaintenanceJob(_ job: SessionMaintenanceJobRecord) throws
  func cancelMaintenanceJob(operationID: String) throws -> SessionMaintenanceJobRecord?
  func querySeries(
    sessionID: String,
    startElapsedMs: Int64,
    endElapsedMs: Int64,
    fields: [String],
    maxPoints: Int,
    publishedSampleSequence: Int64?
  ) throws -> [SessionSeriesPoint]
  func querySeries(
    sessionID: String,
    startElapsedMs: Int64,
    endElapsedMs: Int64,
    fields: [String],
    maxPoints: Int
  ) throws -> [SessionSeriesPoint]
  func querySeriesSource(
    sessionID: String,
    startElapsedMs: Int64,
    endElapsedMs: Int64,
    fields: [String],
    maxRows: Int,
    publishedSampleSequence: Int64?
  ) throws -> [SessionSeriesPoint]
  func reduceSeries(
    sessionID: String,
    startElapsedMs: Int64,
    endElapsedMs: Int64,
    fields: [String],
    buckets: Int,
    gapPolicy: SessionGapPolicyRecord,
    expectedPersistenceMs: Int64,
    expectedRidingPersistenceMs: Int64?,
    expectedIdlePersistenceMs: Int64?,
    retainedResolutionSeconds: Int64,
    publishedSampleSequence: Int64?
  ) throws -> SessionSeriesReductionRecord
  func sessionEvents(
    sessionID: String,
    startElapsedMs: Int64,
    endElapsedMs: Int64,
    publishedEventSequence: Int64?
  ) throws -> [SessionEventRecord]
  func queryLocations(
    sessionID: String,
    startElapsedMs: Int64,
    endElapsedMs: Int64,
    limit: Int,
    cursor: String?,
    publishedLocationSequence: Int64?
  ) throws -> SessionLocationPageRecord
  func sessionEvents(
    sessionID: String,
    startElapsedMs: Int64,
    endElapsedMs: Int64
  ) throws -> [SessionEventRecord]
  func sessionExtensionFields(sessionID: String) throws -> [String]
  func withSessionReadCancellation<T>(
    _ cancellation: SessionReadCancellation,
    body: () throws -> T
  ) rethrows -> T
  func querySampleWindow(
    sessionID: String,
    centerElapsedMs: Int64,
    beforeMs: Int64,
    afterMs: Int64,
    fields: [String],
    publishedSampleSequence: Int64?
  ) throws -> [SessionSeriesPoint]
  /// Interrupts every historical read for explicit session maintenance.
  /// HTTP request cancellation uses `withSessionReadCancellation` instead.
  func cancelHistoricalReads(sessionID: String)
  func querySampleWindow(
    sessionID: String,
    centerElapsedMs: Int64,
    beforeMs: Int64,
    afterMs: Int64,
    fields: [String]
  ) throws -> [SessionSeriesPoint]
  func deleteSession(sessionID: String) throws
  func cancelPendingDelete(operationID: String) throws -> Bool
  func exportSession(sessionID: String, format: String) throws -> SessionExportRecord
  func importSessionArchive(data: Data) throws -> String
}

public extension SessionStorePort {
  func withSessionReadCancellation<T>(
    _: SessionReadCancellation,
    body: () throws -> T
  ) rethrows -> T {
    try body()
  }

  func queryLocations(
    sessionID _: String,
    startElapsedMs _: Int64,
    endElapsedMs _: Int64,
    limit _: Int,
    cursor _: String?,
    publishedLocationSequence _: Int64?
  ) throws -> SessionLocationPageRecord {
    throw SessionStoreError.sessionNotFound
  }

  func querySeries(
    sessionID: String,
    startElapsedMs: Int64,
    endElapsedMs: Int64,
    fields: [String],
    maxPoints: Int,
    publishedSampleSequence _: Int64?
  ) throws -> [SessionSeriesPoint] {
    try querySeries(
      sessionID: sessionID, startElapsedMs: startElapsedMs, endElapsedMs: endElapsedMs,
      fields: fields, maxPoints: maxPoints
    )
  }

  func querySeriesSource(
    sessionID: String,
    startElapsedMs: Int64,
    endElapsedMs: Int64,
    fields: [String],
    maxRows: Int,
    publishedSampleSequence _: Int64?
  ) throws -> [SessionSeriesPoint] {
    guard maxRows >= 1 else { throw SessionStoreError.invalidRange }
    return try querySeries(
      sessionID: sessionID, startElapsedMs: startElapsedMs, endElapsedMs: endElapsedMs,
      fields: fields, maxPoints: min(maxRows, 2_000)
    )
  }

  func reduceSeries(
    sessionID: String,
    startElapsedMs: Int64,
    endElapsedMs: Int64,
    fields: [String],
    buckets: Int,
    gapPolicy: SessionGapPolicyRecord,
    expectedPersistenceMs: Int64,
    expectedRidingPersistenceMs: Int64?,
    expectedIdlePersistenceMs: Int64?,
    retainedResolutionSeconds: Int64,
    publishedSampleSequence: Int64?
  ) throws -> SessionSeriesReductionRecord {
    let started = DispatchTime.now().uptimeNanoseconds
    let rows = try querySeriesSource(
      sessionID: sessionID,
      startElapsedMs: startElapsedMs,
      endElapsedMs: endElapsedMs,
      fields: fields,
      maxRows: 200_000,
      publishedSampleSequence: publishedSampleSequence
    )
    let columns = try SessionHistoryReducer.reduceSeries(
      rows: rows,
      fields: fields,
      range: SessionElapsedRange(startElapsedMs: startElapsedMs, endElapsedMs: endElapsedMs),
      buckets: buckets,
      gapPolicy: gapPolicy,
      expectedPersistenceMs: expectedPersistenceMs,
      expectedRidingPersistenceMs: expectedRidingPersistenceMs,
      expectedIdlePersistenceMs: expectedIdlePersistenceMs,
      retainedResolutionSeconds: retainedResolutionSeconds,
      rowsAreOrdered: true
    )
    return SessionSeriesReductionRecord(
      columns: columns,
      sourceRows: rows.count,
      sourceMs: 0,
      reducerMs: Double(DispatchTime.now().uptimeNanoseconds - started) / 1_000_000,
      elapsedMs: Double(DispatchTime.now().uptimeNanoseconds - started) / 1_000_000
    )
  }

  func sessionEvents(
    sessionID: String,
    startElapsedMs: Int64,
    endElapsedMs: Int64,
    publishedEventSequence _: Int64?
  ) throws -> [SessionEventRecord] {
    try sessionEvents(
      sessionID: sessionID, startElapsedMs: startElapsedMs, endElapsedMs: endElapsedMs
    )
  }

  func querySampleWindow(
    sessionID: String,
    centerElapsedMs: Int64,
    beforeMs: Int64,
    afterMs: Int64,
    fields: [String],
    publishedSampleSequence _: Int64?
  ) throws -> [SessionSeriesPoint] {
    try querySampleWindow(
      sessionID: sessionID, centerElapsedMs: centerElapsedMs,
      beforeMs: beforeMs, afterMs: afterMs, fields: fields
    )
  }

  func sessionSnapshot(sessionID _: String) throws -> SessionPublishedSnapshotRecord {
    throw SessionStoreError.sessionNotFound
  }

  func cancelHistoricalReads(sessionID _: String) {}
  func sessionHistoryConfiguration(sessionID _: String) throws -> SessionHistoryConfiguration {
    SessionHistoryConfiguration(expectedRidingPersistenceMs: 1_000, expectedIdlePersistenceMs: 1_000)
  }

  func sessionPruneMetadata(sessionID _: String) throws -> SessionPruneMetadataRecord {
    SessionPruneMetadataRecord(targetResolutionSeconds: nil, recordedSampleCount: 0, storedSampleCount: 0, policy: nil)
  }

  func prunePreview(sessionID _: String, targetResolutionSeconds _: Int64) throws -> SessionPrunePreviewRecord {
    throw SessionStoreError.sessionNotFound
  }

  func pruneSession(sessionID _: String, targetResolutionSeconds _: Int64) throws -> SessionPrunePreviewRecord {
    throw SessionStoreError.sessionNotFound
  }

  func maintenanceJob() throws -> SessionMaintenanceJobRecord? {
    nil
  }

  func saveMaintenanceJob(_: SessionMaintenanceJobRecord) throws {}
  func cancelMaintenanceJob(operationID _: String) throws -> SessionMaintenanceJobRecord? {
    nil
  }

  func sessionExtensionFields(sessionID _: String) throws -> [String] {
    []
  }

  func deleteSession(sessionID _: String) throws {
    throw SessionStoreError.sessionNotFound
  }

  func cancelPendingDelete(operationID _: String) throws -> Bool {
    false
  }

  func exportSession(sessionID _: String, format _: String) throws -> SessionExportRecord {
    throw SessionStoreError.sessionNotFound
  }

  func importSessionArchive(data _: Data) throws -> String {
    throw SessionStoreError.invalidArchive
  }
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
