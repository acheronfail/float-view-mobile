@testable import FloatViewBackend
import Testing

private func auxiliaryFirmware(
  hardwareType: UInt8, canID: UInt8? = nil, name: String = "node", major: UInt8 = 6,
  minor: UInt8 = 0
) -> FirmwareRecord {
  FirmwareRecord(
    major: major, minor: minor, hardwareName: name, firmwareName: nil,
    hardwareType: hardwareType, canID: canID
  )
}

@Test func auxiliaryDiscoveryRecognizesOnlyTheObservedVBMS32Identity() {
  let vbms32 = AuxiliaryNodeDiscovery()
  #expect(vbms32.begin(
    localIdentity: auxiliaryFirmware(hardwareType: 0),
    reusableInventory: [
      auxiliaryFirmware(hardwareType: 2, canID: 10, name: "VBMS32", minor: 6),
    ], preferredCanID: nil
  ) == .requestValues(.can(10)))

  for identity in [
    auxiliaryFirmware(hardwareType: 2, canID: 10, name: "Gateway", minor: 6),
    auxiliaryFirmware(hardwareType: 2, canID: 10, name: "VBMS32", minor: 5),
    auxiliaryFirmware(hardwareType: 2, canID: 10, name: "VBMS32", major: 7, minor: 0),
  ] {
    let otherCustomModule = AuxiliaryNodeDiscovery()
    #expect(otherCustomModule.begin(
      localIdentity: auxiliaryFirmware(hardwareType: 0),
      reusableInventory: [identity], preferredCanID: nil
    ) == .none)
    #expect(otherCustomModule.state.status == .notFound)
  }
}

@Test func auxiliaryDiscoverySelectsDirectAndReusableUniqueVBMS() {
  let direct = AuxiliaryNodeDiscovery()
  #expect(direct.begin(
    localIdentity: auxiliaryFirmware(hardwareType: 1),
    reusableInventory: nil, preferredCanID: nil
  ) == .requestValues(.direct))

  let can = AuxiliaryNodeDiscovery()
  #expect(can.begin(
    localIdentity: auxiliaryFirmware(hardwareType: 0),
    reusableInventory: [auxiliaryFirmware(hardwareType: 1, canID: 42)],
    preferredCanID: nil
  ) == .requestValues(.can(42)))
}

@Test func auxiliaryDiscoveryBoundsAndClassifiesCandidates() {
  let none = AuxiliaryNodeDiscovery()
  #expect(none.begin(
    localIdentity: auxiliaryFirmware(hardwareType: 0), reusableInventory: [],
    preferredCanID: nil
  ) == .none)
  #expect(none.state.status == .notFound)

  let multiple = AuxiliaryNodeDiscovery()
  #expect(multiple.begin(
    localIdentity: auxiliaryFirmware(hardwareType: 0),
    reusableInventory: [
      auxiliaryFirmware(hardwareType: 1, canID: 8),
      auxiliaryFirmware(hardwareType: 1, canID: 3),
    ], preferredCanID: nil
  ) == .none)
  #expect(multiple.state.status == .ambiguous)
  #expect(multiple.state.candidates.map(\.connection) == [.can(3), .can(8)])

  let overflow = AuxiliaryNodeDiscovery()
  #expect(overflow.begin(
    localIdentity: auxiliaryFirmware(hardwareType: 0), reusableInventory: nil,
    preferredCanID: nil
  ) == .pingCAN)
  #expect(overflow.acceptCANIDs(Array(0 ... 64), generation: overflow.generation) == .none)
  #expect(overflow.state.status == .unavailable)
}

@Test func auxiliaryDiscoveryDirectlyProbesPreferencesAndRejectsWrongIdentity() {
  let discovery = AuxiliaryNodeDiscovery()
  #expect(discovery.begin(
    localIdentity: auxiliaryFirmware(hardwareType: 0), reusableInventory: [],
    preferredCanID: 42
  ) == .probeFirmware(.can(42)))
  #expect(discovery.acceptFirmware(
    auxiliaryFirmware(hardwareType: 0, canID: 42), generation: discovery.generation
  ) == .none)
  #expect(discovery.state.status == .unavailable)

  let selected = AuxiliaryNodeDiscovery()
  #expect(selected.begin(
    localIdentity: auxiliaryFirmware(hardwareType: 0), reusableInventory: nil,
    preferredCanID: 42
  ) == .probeFirmware(.can(42)))
  #expect(selected.acceptFirmware(
    auxiliaryFirmware(hardwareType: 1, canID: 42), generation: selected.generation
  ) == .requestValues(.can(42)))
}

@Test func auxiliaryDiscoveryRetainsStaleSnapshotRecoversAndResetsAcrossGenerations() {
  let discovery = AuxiliaryNodeDiscovery()
  #expect(discovery.begin(
    localIdentity: auxiliaryFirmware(hardwareType: 0),
    reusableInventory: [auxiliaryFirmware(hardwareType: 1, canID: 42)], preferredCanID: nil
  ) == .requestValues(.can(42)))
  let snapshot = bmsSnapshot(canID: 42)
  discovery.acceptSnapshot(snapshot)
  #expect(discovery.state.status == .attached)
  discovery.markUnavailable()
  #expect(discovery.state.status == .unavailable)
  #expect(discovery.state.latestSnapshot == snapshot)
  discovery.acceptSnapshot(snapshot)
  #expect(discovery.state.status == .attached)

  #expect(discovery.begin(
    localIdentity: auxiliaryFirmware(hardwareType: 0), reusableInventory: nil, preferredCanID: 43
  ) == .probeFirmware(.can(43)))
  #expect(discovery.state.latestSnapshot == nil)
  discovery.reset()
  #expect(discovery.state == BmsState())
}

private func bmsSnapshot(canID: UInt8) -> BmsSnapshot {
  BmsSnapshot(
    observedAt: .init(timeIntervalSince1970: 1), totalVoltageV: 50, chargeVoltageV: 54,
    inputCurrentA: 0, inputICCurrentA: 0, sessionAmpHoursAh: 0, sessionWattHoursWh: 0,
    cellVoltagesV: [4.1], balancing: [false], temperaturesC: [25], icTemperatureC: 25,
    humiditySensorTemperatureC: 24, humidityPercent: 50, highestCellTemperatureC: 25,
    stateOfCharge: 0.8, stateOfHealth: 0.9, reportedCanID: canID,
    lifetimeChargeAmpHoursAh: 1, lifetimeChargeWattHoursWh: 2,
    lifetimeDischargeAmpHoursAh: 1, lifetimeDischargeWattHoursWh: 2,
    pressurePa: 101_325, dataVersion: 1, status: "OK"
  )
}
