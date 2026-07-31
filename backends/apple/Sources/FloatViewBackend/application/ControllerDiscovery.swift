import Foundation

/// Pure bounds and identity rules shared by the connection state machine.
enum ControllerDiscoveryPolicy {
  static let maximumCanIDs = 64
  static let maximumCandidates = 16

  static func normalizedCanIDs(_ values: [UInt8]) -> [UInt8] {
    Array(Set(values.filter { $0 <= 253 })).sorted()
  }

  static func meaningfulUUID(_ value: Data?) -> Data? {
    guard let value, value.count == 12,
          value.contains(where: { $0 != 0 }),
          value.contains(where: { $0 != 0xFF })
    else { return nil }
    return value
  }

  static func preferredCanID(_ valuesJSON: String) -> UInt8? {
    guard let values = try? JSONSerialization.jsonObject(with: Data(valuesJSON.utf8))
      as? [String: Any],
      let number = values["preferredCanId"] as? NSNumber,
      number.doubleValue.rounded() == number.doubleValue,
      (0 ... 253).contains(number.intValue)
    else { return nil }
    return UInt8(number.intValue)
  }

  static func version(_ value: FirmwareRecord) -> String {
    "\(value.major).\(String(format: "%02d", value.minor))"
  }

  static func samePublicIdentity(_ left: FirmwareRecord, _ right: FirmwareRecord) -> Bool {
    left.major == right.major
      && left.minor == right.minor
      && left.hardwareName == right.hardwareName
      && left.firmwareName == right.firmwareName
      && left.hardwareType == right.hardwareType
  }

  static func boundedDisplayValue(_ value: String?) -> String? {
    value.map { String($0.prefix(80)) }
  }
}

enum ControllerProbeIntent: Equatable {
  case preferred(UInt8)
  case hint(canID: UInt8, controllerUUID: Data)
  case enumerating
}

struct ControllerContinuity {
  let hasActiveSession: Bool
  let route: ControllerRoute?
  let controllerUUID: Data?
}

enum ControllerDiscoveryEffect: Equatable {
  case probeFirmware(ControllerRoute)
  case pingCAN
  indirect case clearHintThen(ControllerDiscoveryEffect)
  case requestRefloatInfo(route: ControllerRoute, rememberController: FirmwareRecord?)
  case requestTelemetryLayout
  case ready
  case selectingController
  case failed(ConnectionIssue)
  case none
}

/// Pure, generation-aware owner of every direct/CAN route decision.
final class ControllerDiscovery {
  private(set) var firmware: FirmwareRecord?
  private(set) var transportFirmware: FirmwareRecord?
  private(set) var refloatPackageVersion: String?
  private(set) var refloatFamily: RefloatTelemetryFamily?
  private(set) var controllerRoute: ControllerRoute?
  private(set) var controllerCandidates: [ControllerCandidate] = []
  private(set) var connectionIssue: ConnectionIssue?
  private(set) var started = false
  private(set) var identificationGeneration = 0
  private(set) var canProbeGeneration = 0
  private(set) var activeCanProbeID: UInt8?

  private var probedControllers: [UInt8: FirmwareRecord] = [:]
  private var pendingCanIDs: [UInt8] = []
  private var probeIntent: ControllerProbeIntent?
  private var recoveryControllerUUID: Data?
  private var continuity = ControllerContinuity(
    hasActiveSession: false,
    route: nil,
    controllerUUID: nil
  )

  func begin() -> (Int, ControllerDiscoveryEffect) {
    guard !started else { return (identificationGeneration, .none) }
    started = true
    identificationGeneration += 1
    return (identificationGeneration, .probeFirmware(.direct))
  }

  func acceptTransport(
    _ value: FirmwareRecord,
    preferredCanID: UInt8?,
    routingHint: ControllerRoutingHint?
  ) -> ControllerDiscoveryEffect {
    transportFirmware = value
    if value.hardwareType == Self.vescHardwareType {
      controllerRoute = .direct
      firmware = value
      return VescCompatibility.supports(value)
        ? .requestRefloatInfo(route: .direct, rememberController: nil)
        : fail(
          .unsupportedController,
          "Direct controller firmware \(ControllerDiscoveryPolicy.version(value)) is not supported."
        )
    }
    if let preferredCanID {
      if routingHint?.canID == preferredCanID {
        recoveryControllerUUID =
          ControllerDiscoveryPolicy.meaningfulUUID(routingHint?.controllerUUID)
      }
      probeIntent = .preferred(preferredCanID)
      return probe(preferredCanID)
    }
    if let routingHint,
       let uuid = ControllerDiscoveryPolicy.meaningfulUUID(routingHint.controllerUUID)
    {
      recoveryControllerUUID = uuid
      probeIntent = .hint(canID: routingHint.canID, controllerUUID: uuid)
      return probe(routingHint.canID)
    }
    return beginEnumeration()
  }

  func acceptCANFirmware(
    _ value: FirmwareRecord,
    continuity: ControllerContinuity
  ) -> ControllerDiscoveryEffect {
    self.continuity = continuity
    guard let canID = value.canID, activeCanProbeID == canID else { return .none }
    activeCanProbeID = nil
    switch probeIntent {
    case let .preferred(expectedID):
      guard expectedID == canID else { return .none }
      guard value.hardwareType == Self.vescHardwareType else {
        return fail(
          .preferredControllerUnavailable,
          "The configured CAN controller is unavailable."
        )
      }
      if let expectedUUID = recoveryControllerUUID,
         ControllerDiscoveryPolicy.meaningfulUUID(value.controllerUUID) != expectedUUID
      {
        return fail(
          .controllerChanged,
          "The configured CAN route now identifies a different controller."
        )
      }
      return choose(value)
    case let .hint(expectedID, expectedUUID):
      guard expectedID == canID else { return .none }
      if value.hardwareType == Self.vescHardwareType,
         ControllerDiscoveryPolicy.meaningfulUUID(value.controllerUUID) == expectedUUID
      {
        return choose(value)
      }
      return .clearHintThen(beginEnumeration())
    case .enumerating:
      if value.hardwareType == Self.vescHardwareType {
        probedControllers[canID] = value
        if probedControllers.count > ControllerDiscoveryPolicy.maximumCandidates {
          return fail(
            .tooManyControllers,
            "More than \(ControllerDiscoveryPolicy.maximumCandidates) VESC controllers were found."
          )
        }
      }
      return nextCANProbe()
    case nil:
      return .none
    }
  }

  func acceptCANIDs(
    _ rawCanIDs: [UInt8],
    continuity: ControllerContinuity
  ) -> ControllerDiscoveryEffect {
    self.continuity = continuity
    guard probeIntent == .enumerating else { return .none }
    let normalized = ControllerDiscoveryPolicy.normalizedCanIDs(rawCanIDs)
    guard normalized.count <= ControllerDiscoveryPolicy.maximumCanIDs else {
      return fail(
        .tooManyCanDevices,
        "The CAN network reported more than \(ControllerDiscoveryPolicy.maximumCanIDs) devices."
      )
    }
    pendingCanIDs = normalized
    return nextCANProbe()
  }

  func acceptRefloatInfo(_ version: String) -> ControllerDiscoveryEffect {
    guard let entry = RefloatCompatibility.entry(wireVersion: version) else {
      return fail(
        .unsupportedRefloat,
        "This controller's Refloat version is not supported."
      )
    }
    refloatPackageVersion = version
    refloatFamily = entry.family
    return entry.family == .dynamic ? .requestTelemetryLayout : .ready
  }

  func acceptTelemetryLayout(
    fields: [String],
    runtimeFields: [String]
  ) -> ControllerDiscoveryEffect {
    guard refloatFamily == .dynamic,
          RefloatCompatibility.supportsDynamicLayout(
            fields: fields,
            runtimeFields: runtimeFields
          )
    else {
      return fail(
        .unsupportedRefloat,
        "This controller's Refloat telemetry layout is not supported."
      )
    }
    return .ready
  }

  func canSelect(canID: UInt8) -> Bool {
    controllerCandidates.contains {
      $0.canID == canID && $0.vescSupported && probedControllers[canID] != nil
    }
  }

  func selectController(canID: UInt8) -> ControllerDiscoveryEffect {
    guard canSelect(canID: canID), let selected = probedControllers[canID]
    else { preconditionFailure("Controller selection is not valid for the current state") }
    return choose(selected)
  }

  func identificationTimedOut(generation: Int) -> ControllerDiscoveryEffect {
    guard generation == identificationGeneration else { return .none }
    switch probeIntent {
    case .preferred:
      return fail(
        .preferredControllerUnavailable,
        "The configured CAN controller did not respond."
      )
    case .hint:
      identificationGeneration += 1
      return .clearHintThen(beginEnumeration())
    default:
      return fail(.identificationTimeout, "Controller identification timed out.")
    }
  }

  func canProbeTimedOut(canID: UInt8, generation: Int) -> ControllerDiscoveryEffect {
    guard activeCanProbeID == canID, canProbeGeneration == generation else { return .none }
    activeCanProbeID = nil
    switch probeIntent {
    case .preferred:
      return fail(
        .preferredControllerUnavailable,
        "The configured CAN controller did not respond."
      )
    case .hint:
      return .clearHintThen(beginEnumeration())
    case .enumerating:
      return nextCANProbe()
    case nil:
      return .none
    }
  }

  @discardableResult
  func markFailure(_ code: ConnectionIssueCode, _ message: String) -> ControllerDiscoveryEffect {
    fail(code, message)
  }

  func clearIssue() {
    connectionIssue = nil
  }

  func reset() {
    firmware = nil
    transportFirmware = nil
    refloatPackageVersion = nil
    refloatFamily = nil
    pendingCanIDs.removeAll()
    probedControllers.removeAll()
    probeIntent = nil
    controllerCandidates.removeAll()
    controllerRoute = nil
    connectionIssue = nil
    recoveryControllerUUID = nil
    activeCanProbeID = nil
    continuity = ControllerContinuity(hasActiveSession: false, route: nil, controllerUUID: nil)
    started = false
    identificationGeneration += 1
    canProbeGeneration += 1
  }

  private func beginEnumeration() -> ControllerDiscoveryEffect {
    probeIntent = .enumerating
    pendingCanIDs.removeAll()
    probedControllers.removeAll()
    activeCanProbeID = nil
    return .pingCAN
  }

  private func probe(_ canID: UInt8) -> ControllerDiscoveryEffect {
    activeCanProbeID = canID
    canProbeGeneration += 1
    return .probeFirmware(.can(canID))
  }

  private func nextCANProbe() -> ControllerDiscoveryEffect {
    guard !pendingCanIDs.isEmpty else { return finishCandidates() }
    return probe(pendingCanIDs.removeFirst())
  }

  private func finishCandidates() -> ControllerDiscoveryEffect {
    let records = probedControllers.values.sorted { ($0.canID ?? 255) < ($1.canID ?? 255) }
    controllerCandidates = records.compactMap { value in
      guard let canID = value.canID else { return nil }
      return ControllerCandidate(
        canID: canID,
        hardwareName: ControllerDiscoveryPolicy.boundedDisplayValue(value.hardwareName),
        firmwareName: ControllerDiscoveryPolicy.boundedDisplayValue(value.firmwareName),
        vescVersion: ControllerDiscoveryPolicy.version(value),
        vescSupported: VescCompatibility.supports(value)
      )
    }
    guard !records.isEmpty else {
      return fail(
        continuity.hasActiveSession ? .controllerChanged : .noControllerFound,
        continuity.hasActiveSession
          ? "The controller from the active session was not found."
          : "No VESC motor controller was found."
      )
    }
    if let expected = recoveryControllerUUID,
       let recovered = records.first(where: {
         ControllerDiscoveryPolicy.meaningfulUUID($0.controllerUUID) == expected
       })
    {
      return choose(recovered)
    }
    if continuity.hasActiveSession {
      if continuity.controllerUUID != nil {
        return fail(
          .controllerChanged,
          "The controller from the active session was not found."
        )
      }
      if case let .can(previousCanID) = continuity.route,
         let recovered = records.first(where: {
           $0.canID == previousCanID && VescCompatibility.supports($0)
         })
      {
        return choose(recovered)
      }
      return fail(
        .controllerChanged,
        "The controller from the active session was not found."
      )
    }
    if records.count == 1, let only = records.first {
      guard VescCompatibility.supports(only) else {
        controllerRoute = only.canID.map(ControllerRoute.can)
        firmware = only
        return fail(
          .unsupportedController,
          "Controller firmware \(ControllerDiscoveryPolicy.version(only)) is not supported."
        )
      }
      return choose(only)
    }
    return .selectingController
  }

  private func choose(_ value: FirmwareRecord) -> ControllerDiscoveryEffect {
    guard let canID = value.canID else { return .none }
    controllerRoute = .can(canID)
    firmware = value
    controllerCandidates.removeAll()
    guard VescCompatibility.supports(value) else {
      return fail(
        .unsupportedController,
        "Controller firmware \(ControllerDiscoveryPolicy.version(value)) is not supported."
      )
    }
    return .requestRefloatInfo(route: .can(canID), rememberController: value)
  }

  private func fail(
    _ code: ConnectionIssueCode,
    _ message: String
  ) -> ControllerDiscoveryEffect {
    let issue = ConnectionIssue(code: code, message: message)
    connectionIssue = issue
    return .failed(issue)
  }

  private static let vescHardwareType: UInt8 = 0
}
