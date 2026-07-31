@testable import FloatViewBackend
import Foundation
import Testing

@Test func dynamicRefloatDiscoveryRequiresAuditedCompleteLayout() {
  let supported = ControllerDiscovery()
  #expect(supported.acceptRefloatInfo("1.2.1") == .requestTelemetryLayout)
  #expect(supported.acceptTelemetryLayout(
    fields: RefloatCompatibility.requiredFields + ["future.energy"],
    runtimeFields: RefloatCompatibility.requiredRuntimeFields
  ) == .ready)

  let incomplete = ControllerDiscovery()
  #expect(incomplete.acceptRefloatInfo("1.1.2") == .requestTelemetryLayout)
  guard case let .failed(issue) = incomplete.acceptTelemetryLayout(
    fields: Array(RefloatCompatibility.requiredFields.dropFirst()),
    runtimeFields: RefloatCompatibility.requiredRuntimeFields
  ) else {
    Issue.record("Expected an incomplete Refloat layout to fail")
    return
  }
  #expect(issue.code == .unsupportedRefloat)

  let unknown = ControllerDiscovery()
  guard case let .failed(issue) = unknown.acceptRefloatInfo("1.3.0") else {
    Issue.record("Expected an unknown Refloat release to fail")
    return
  }
  #expect(issue.code == .unsupportedRefloat)
}

@Test func sharedLifecycleVectorsMatchAppleDiscovery() throws {
  for vector in try discoveryVectors() {
    let discovery = ControllerDiscovery()
    let (_, initial) = discovery.begin()
    #expect(initial == .probeFirmware(.direct), "\(vector.name): initial probe")
    let hint = vector.hintCanID.map {
      ControllerRoutingHint(
        savedDeviceID: "saved",
        canID: $0,
        controllerUUID: uuid(vector.hintUUIDByte)
      )
    }
    var effect = discovery.acceptTransport(
      firmware(hardwareType: vector.transportHardwareType),
      preferredCanID: vector.preferredCanID,
      routingHint: hint
    )
    let continuity = ControllerContinuity(
      hasActiveSession: vector.activeRouteCanID != nil,
      route: vector.activeRouteCanID.map(ControllerRoute.can),
      controllerUUID: uuid(vector.activeUUIDByte)
    )
    for _ in 0 ..< 128 {
      switch effect {
      case let .probeFirmware(.can(canID)):
        if let target = vector.controllers.first(where: { $0.canID == canID }) {
          effect = discovery.acceptCANFirmware(target.firmware, continuity: continuity)
        } else {
          effect = discovery.canProbeTimedOut(
            canID: canID,
            generation: discovery.canProbeGeneration
          )
        }
      case .pingCAN:
        effect = discovery.acceptCANIDs(vector.canIDs, continuity: continuity)
      case let .clearHintThen(next):
        effect = next
      default:
        break
      }
      if effect.isTerminal { break }
    }
    #expect(effect.stableName == vector.expectedEffect, "\(vector.name): effect")
    #expect(discovery.controllerRoute.stableName == vector.expectedRoute, "\(vector.name): route")
    #expect(
      discovery.controllerCandidates.map(\.canID).map(String.init).joined(separator: ",")
        == vector.expectedCandidates,
      "\(vector.name): candidates"
    )
    #expect(
      (discovery.connectionIssue?.code.rawValue ?? "") == vector.expectedIssue,
      "\(vector.name): issue"
    )
  }
}

private struct DiscoveryVector {
  let name: String
  let transportHardwareType: UInt8
  let preferredCanID: UInt8?
  let hintCanID: UInt8?
  let hintUUIDByte: UInt8?
  let canIDs: [UInt8]
  let controllers: [VectorController]
  let activeRouteCanID: UInt8?
  let activeUUIDByte: UInt8?
  let expectedEffect: String
  let expectedRoute: String
  let expectedCandidates: String
  let expectedIssue: String
}

private struct VectorController {
  let canID: UInt8
  let firmware: FirmwareRecord
}

private func discoveryVectors() throws -> [DiscoveryVector] {
  var root = URL(fileURLWithPath: #filePath)
  for _ in 0 ..< 4 {
    root.deleteLastPathComponent()
  }
  let content = try String(
    contentsOf: root.appending(path: "fixtures/connection/controller-discovery.tsv"),
    encoding: .utf8
  )
  return content.split(separator: "\n")
    .filter { !$0.isEmpty && !$0.hasPrefix("#") }
    .map { line in
      let columns = line.split(separator: "|", omittingEmptySubsequences: false).map(String.init)
      return DiscoveryVector(
        name: columns[0],
        transportHardwareType: UInt8(columns[1])!,
        preferredCanID: UInt8(columns[2]),
        hintCanID: UInt8(columns[3]),
        hintUUIDByte: UInt8(columns[4]),
        canIDs: integers(columns[5]),
        controllers: controllers(columns[6]),
        activeRouteCanID: UInt8(columns[7]),
        activeUUIDByte: UInt8(columns[8]),
        expectedEffect: columns[9],
        expectedRoute: columns[10],
        expectedCandidates: columns[11],
        expectedIssue: columns[12]
      )
    }
}

private func integers(_ value: String) -> [UInt8] {
  value.isEmpty ? [] : value.split(separator: ",").compactMap { UInt8($0) }
}

private func controllers(_ value: String) -> [VectorController] {
  guard !value.isEmpty else { return [] }
  return value.split(separator: ";").map { encoded in
    let values = encoded.split(separator: ":").compactMap { UInt8($0) }
    return VectorController(
      canID: values[0],
      firmware: firmware(
        hardwareType: values[1],
        canID: values[0],
        controllerUUID: uuid(values[2]),
        minor: values[3]
      )
    )
  }
}

private func firmware(
  hardwareType: UInt8,
  canID: UInt8? = nil,
  controllerUUID: Data? = nil,
  minor: UInt8 = 6
) -> FirmwareRecord {
  FirmwareRecord(
    major: 6,
    minor: minor,
    hardwareName: hardwareType == 0 ? "THOR300" : "Gateway",
    firmwareName: nil,
    hardwareType: hardwareType,
    canID: canID,
    controllerUUID: controllerUUID
  )
}

private func uuid(_ value: UInt8?) -> Data? {
  value.flatMap { $0 == 0 ? nil : Data(repeating: $0, count: 12) }
}

private extension ControllerDiscoveryEffect {
  var isTerminal: Bool {
    switch self {
    case .requestRefloatInfo, .selectingController, .failed, .requestTelemetryLayout, .ready:
      true
    default:
      false
    }
  }

  var stableName: String {
    switch self {
    case .requestRefloatInfo: "request_refloat"
    case .selectingController: "selecting"
    case .failed: "failed"
    case .probeFirmware: "probe"
    case .pingCAN: "ping"
    case .clearHintThen: "clear_hint"
    case .requestTelemetryLayout: "layout"
    case .ready: "ready"
    case .none: "none"
    }
  }
}

private extension Optional where Wrapped == ControllerRoute {
  var stableName: String {
    switch self {
    case .some(.direct): "direct"
    case let .some(.can(canID)): "can:\(canID)"
    case .none: ""
    }
  }
}
