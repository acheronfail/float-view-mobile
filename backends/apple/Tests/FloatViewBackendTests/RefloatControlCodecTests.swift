@testable import FloatViewBackend
import Foundation
import Testing

private struct ControlFixture: Decodable {
  struct Item: Decodable {
    let name: String
    let version: String
    let request: [UInt8]
    let response: [UInt8]?
  }

  let cases: [Item]
}

private struct ShutdownFixture: Decodable {
  struct Item: Decodable {
    let name: String
    let version: String
    let route: String
    let canId: UInt8?
    let request: [UInt8]
  }

  let cases: [Item]
}

private func controlFixture() throws -> ControlFixture {
  var root = URL(fileURLWithPath: #filePath)
  for _ in 0 ..< 4 {
    root.deleteLastPathComponent()
  }
  let data = try Data(contentsOf: root.appending(path: "fixtures/refloat/product-controls.json"))
  return try JSONDecoder().decode(ControlFixture.self, from: data)
}

private func shutdownFixture() throws -> ShutdownFixture {
  var root = URL(fileURLWithPath: #filePath)
  for _ in 0 ..< 4 {
    root.deleteLastPathComponent()
  }
  let data = try Data(contentsOf: root.appending(path: "fixtures/vesc/controller-shutdown.json"))
  return try JSONDecoder().decode(ShutdownFixture.self, from: data)
}

@Test func sourceDerivedControllerShutdownRequestsAreExact() throws {
  let service = VescProtocolService()
  for item in try shutdownFixture().cases {
    let route: ControllerRoute = try item.route == "can" ? .can(#require(item.canId)) : .direct
    #expect(try VescFrameCodec.decode(Array(service.controllerShutdownRequest(route: route))) == item.request)
  }
  #expect(VescCompatibility.supportsShutdown(.init(
    major: 6, minor: 5, hardwareName: "Any", firmwareName: "Refloat"
  )))
  #expect(!VescCompatibility.supportsShutdown(.init(
    major: 6, minor: 2, hardwareName: "Thor300", firmwareName: "Refloat"
  )))
  #expect(!VescCompatibility.supportsShutdown(.init(
    major: 6, minor: 6, testVersion: 1, hardwareName: "Thor300", firmwareName: "Refloat"
  )))
}

@Test func exactRefloatControlProfilesFailClosed() {
  for version in ["1.0", "1.1.0", "1.1.1", "1.1.2"] {
    #expect(RefloatCompatibility.controlProfile(wireVersion: version) == .init(
      lighting: .legacyCommand202, fatalClearAvailable: false
    ))
  }
  for version in ["1.2.0", "1.2.1", "1.2.2", "1.2.3"] {
    #expect(RefloatCompatibility.controlProfile(wireVersion: version) == .init(
      lighting: .stableCommand20, fatalClearAvailable: true
    ))
  }
  #expect(RefloatCompatibility.controlProfile(wireVersion: "1.0.0") == nil)
  #expect(RefloatCompatibility.controlProfile(wireVersion: "1.2.4") == nil)
  #expect(RefloatCompatibility.controlProfile(wireVersion: "1.2.3-beta1") == nil)
}

@Test func sourceDerivedRefloatControlRequestsAndLightingResponsesAreExact() throws {
  let service = VescProtocolService()
  for item in try controlFixture().cases {
    let wireVersion = item.version.hasPrefix("1.0.") ? "1.0" : item.version
    let profile = try #require(RefloatCompatibility.controlProfile(wireVersion: wireVersion))
    let frame: Data = switch item.name {
    case "package-enable": try service.refloatPackageControlRequest(route: .direct, enabled: true)
    case "package-disable": try service.refloatPackageControlRequest(route: .direct, enabled: false)
    case "flywheel-enable-defaults-footpad-abort":
      try service.refloatFlywheelControlRequest(route: .direct, enabled: true)
    case "flywheel-disable-defaults-footpad-abort":
      try service.refloatFlywheelControlRequest(route: .direct, enabled: false)
    case "legacy-lighting-leds-off-headlights-on":
      try service.refloatLightingControlRequest(
        route: .direct, profile: profile, control: .leds, enabled: false,
        current: .init(ledsEnabled: true, headlightsEnabled: true)
      )
    case "stable-lighting-headlights-off-leds-on":
      try service.refloatLightingControlRequest(
        route: .direct, profile: profile, control: .headlights, enabled: false,
        current: .init(ledsEnabled: true, headlightsEnabled: true)
      )
    case "fatal-clear": try service.refloatFatalClearRequest(route: .direct, profile: profile)
    default: throw VescProtocolError.invalidFrame
    }
    #expect(try VescFrameCodec.decode(Array(frame)) == item.request)
    if let response = item.response {
      let messages = try service.receive(Data(VescFrameCodec.encode(response)))
      let command: RefloatCommand = response[2] == 202 ? .legacyLights : .stableLights
      #expect(messages == [.refloatLighting(
        command: command,
        state: .init(
          ledsEnabled: response[3] & 1 != 0,
          headlightsEnabled: response[3] & 2 != 0
        )
      )])
    }
  }
}

@Test func refloatControlRoutingAndLightingBoundsAreExact() throws {
  let service = VescProtocolService()
  #expect(
    try VescFrameCodec.decode(Array(service.refloatPackageControlRequest(route: .can(42), enabled: true)))
      == [34, 42, 36, 101, 12, 0]
  )
  let profile = try #require(RefloatCompatibility.controlProfile(wireVersion: "1.2.3"))
  #expect(throws: VescProtocolError.self) {
    try RefloatControlCodec.decodeLightingResponse([36, 101, 20], profile: profile)
  }
  #expect(throws: VescProtocolError.self) {
    try RefloatControlCodec.decodeLightingResponse([36, 101, 20, 3, 0], profile: profile)
  }
  #expect(throws: VescProtocolError.self) {
    try RefloatControlCodec.decodeLightingResponse([36, 101, 202, 3], profile: profile)
  }
}

@Test func rustRefloatControlAdapterExhaustsLightingAndRejectsInvalidBridgeResults() throws {
  let profiles: [RefloatControlProfile] = [
    .init(lighting: .legacyCommand202, fatalClearAvailable: false),
    .init(lighting: .stableCommand20, fatalClearAvailable: true),
  ]
  for profile in profiles {
    for currentFlags: UInt8 in 0 ... 3 {
      let current = RefloatLightingState(
        ledsEnabled: currentFlags & 1 != 0,
        headlightsEnabled: currentFlags & 2 != 0
      )
      let query = try RefloatControlCodec.lightingRequest(
        profile: profile, control: nil, current: current
      )
      #expect(query.last == currentFlags)
      for control in [RefloatBooleanControl.leds, .headlights] {
        for enabled in [false, true] {
          let request = try RefloatControlCodec.lightingRequest(
            profile: profile, control: control, enabled: enabled, current: current
          )
          let bit: UInt8 = control == .leds ? 1 : 2
          #expect(request.last == (enabled ? currentFlags | bit : currentFlags & ~bit))
        }
      }
      let command: UInt8 = profile.lighting == .legacyCommand202 ? 202 : 20
      #expect(try RefloatControlCodec.decodeLightingResponse(
        [36, 101, command, currentFlags], profile: profile
      ) == current)
    }
  }

  #expect(throws: VescProtocolError.self) {
    try RefloatControlCodec.requireCompatibleABI(2)
  }
  #expect(throws: VescProtocolError.self) {
    try RefloatControlCodec.lightingState(flags: 4)
  }
  let legacy = RefloatControlProfile(
    lighting: .legacyCommand202, fatalClearAvailable: false
  )
  #expect(throws: VescProtocolError.self) {
    try RefloatControlCodec.fatalClearRequest(profile: legacy)
  }
}

@Test func fatalLatchAndActiveFirmwareFaultDecodeIndependently() throws {
  let latchedPayload: [UInt8] = [
    36, 101, 31, 4, 8, 0, 0, 0, 0, 2, 0, 0, 0,
    0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0,
  ]
  let latched = try RefloatDynamicTelemetryDecoder.decodeData(
    latchedPayload, fields: ["motor.speed"], runtimeFields: []
  )
  #expect(latched.fatalErrorLatched == true)
  #expect(latched.firmwareFault == nil)

  var activePayload = latchedPayload
  activePayload[4] = 0
  activePayload[15] = 0
  activePayload[16] = 0
  activePayload[17] = 0
  activePayload[18] = 1
  activePayload[23] = 1
  let active = try RefloatDynamicTelemetryDecoder.decodeData(
    activePayload, fields: ["motor.speed"], runtimeFields: []
  )
  #expect(active.fatalErrorLatched == false)
  #expect(active.firmwareFault == .overVoltage)
}
