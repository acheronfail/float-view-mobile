import Foundation

enum AuxiliaryDiscoveryEffect: Equatable {
  case pingCAN
  case probeFirmware(ControllerRoute)
  case requestValues(BmsConnection)
  case none
}

/// Generation-owned, controller-independent selection of one read-only VBMS route.
final class AuxiliaryNodeDiscovery {
  private(set) var generation = 0
  private(set) var state = BmsState()
  private(set) var selectedConnection: BmsConnection?
  private(set) var activeProbeID: UInt8?

  private var pendingCanIDs: [UInt8] = []
  private var discovered: [BmsConnection: FirmwareRecord] = [:]
  private var preferredCanID: UInt8?

  func begin(
    localIdentity: FirmwareRecord,
    reusableInventory: [FirmwareRecord]?,
    preferredCanID: UInt8?
  ) -> AuxiliaryDiscoveryEffect {
    reset()
    self.preferredCanID = preferredCanID
    if Self.isVBMS(localIdentity) {
      discovered[.direct] = localIdentity
      return select(.direct)
    }
    if let preferredCanID {
      if let reusableInventory,
         let identity = reusableInventory.first(where: { $0.canID == preferredCanID })
      {
        return Self.isVBMS(identity) ? select(.can(preferredCanID)) : unavailable()
      }
      return probe(preferredCanID)
    }
    if let reusableInventory {
      for identity in reusableInventory where Self.isVBMS(identity) {
        if let canID = identity.canID { discovered[.can(canID)] = identity }
      }
      return finish()
    }
    return .pingCAN
  }

  func acceptCANIDs(_ rawIDs: [UInt8], generation: Int) -> AuxiliaryDiscoveryEffect {
    guard generation == self.generation, preferredCanID == nil else { return .none }
    let ids = Array(Set(rawIDs.filter { $0 <= 253 })).sorted()
    guard ids.count <= 64 else { return unavailable() }
    pendingCanIDs = ids
    return nextProbe()
  }

  func acceptFirmware(_ value: FirmwareRecord, generation: Int) -> AuxiliaryDiscoveryEffect {
    guard generation == self.generation, let canID = value.canID, activeProbeID == canID else {
      return .none
    }
    activeProbeID = nil
    if let preferredCanID {
      return preferredCanID == canID && Self.isVBMS(value) ? select(.can(canID)) : unavailable()
    }
    if Self.isVBMS(value) { discovered[.can(canID)] = value }
    return nextProbe()
  }

  func probeTimedOut(canID: UInt8, generation: Int) -> AuxiliaryDiscoveryEffect {
    guard generation == self.generation, activeProbeID == canID else { return .none }
    activeProbeID = nil
    return preferredCanID == nil ? nextProbe() : unavailable()
  }

  func markUnavailable() {
    state = BmsState(
      status: .unavailable, connection: selectedConnection,
      candidates: state.candidates, latestSnapshot: state.latestSnapshot
    )
  }

  func acceptSnapshot(_ snapshot: BmsSnapshot) {
    guard let selectedConnection else { return }
    state = BmsState(status: .attached, connection: selectedConnection, latestSnapshot: snapshot)
  }

  func reset() {
    generation += 1
    state = BmsState()
    selectedConnection = nil
    activeProbeID = nil
    pendingCanIDs.removeAll()
    discovered.removeAll()
    preferredCanID = nil
  }

  private func probe(_ canID: UInt8) -> AuxiliaryDiscoveryEffect {
    activeProbeID = canID
    return .probeFirmware(.can(canID))
  }

  private func nextProbe() -> AuxiliaryDiscoveryEffect {
    guard !pendingCanIDs.isEmpty else { return finish() }
    return probe(pendingCanIDs.removeFirst())
  }

  private func finish() -> AuxiliaryDiscoveryEffect {
    let entries = discovered.sorted { Self.sortKey($0.key) < Self.sortKey($1.key) }
    let candidates = entries.prefix(16).map { connection, value in
      BmsCandidate(
        connection: connection,
        hardwareName: ControllerDiscoveryPolicy.boundedDisplayValue(value.hardwareName),
        vescVersion: ControllerDiscoveryPolicy.version(value)
      )
    }
    guard entries.count <= 16 else {
      state = BmsState(status: .ambiguous, candidates: candidates)
      return .none
    }
    guard entries.count == 1, let connection = entries.first?.key else {
      state = BmsState(status: entries.isEmpty ? .notFound : .ambiguous, candidates: candidates)
      return .none
    }
    return select(connection)
  }

  private func select(_ connection: BmsConnection) -> AuxiliaryDiscoveryEffect {
    selectedConnection = connection
    state = BmsState(status: .discovering, connection: connection)
    return .requestValues(connection)
  }

  private func unavailable() -> AuxiliaryDiscoveryEffect {
    markUnavailable()
    return .none
  }

  private static func isVBMS(_ value: FirmwareRecord) -> Bool {
    supportedBMSIdentities.contains { $0.matches(value) }
  }

  /// Fail-closed compatibility catalog backed by immutable upstream identity fixtures.
  private static let supportedBMSIdentities = [
    SupportedBMSIdentity(hardwareType: 1, major: 6, minor: 0),
    SupportedBMSIdentity(hardwareType: 2, major: 6, minor: 6, hardwareName: "VBMS32"),
  ]

  private struct SupportedBMSIdentity {
    let hardwareType: UInt8
    let major: UInt8
    let minor: UInt8
    var hardwareName: String?

    init(hardwareType: UInt8, major: UInt8, minor: UInt8, hardwareName: String? = nil) {
      self.hardwareType = hardwareType
      self.major = major
      self.minor = minor
      self.hardwareName = hardwareName
    }

    func matches(_ value: FirmwareRecord) -> Bool {
      value.hardwareType == hardwareType && value.major == major && value.minor == minor
        && (hardwareName == nil || value.hardwareName == hardwareName)
    }
  }

  private static func sortKey(_ connection: BmsConnection) -> Int {
    switch connection {
    case .direct: -1
    case let .can(canID): Int(canID)
    }
  }
}
