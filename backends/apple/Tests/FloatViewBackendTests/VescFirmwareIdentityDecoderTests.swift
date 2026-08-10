import CVescFirmwareIdentity
@testable import FloatViewBackend
import Foundation
import Testing

@Test func rustFirmwareIdentityBridgePreservesReplacementDecodedFields() throws {
  let payload = [UInt8(0), 6, 6, 0x48, 0xFF, 0] +
    Array(repeating: UInt8(7), count: 12) + [0, 9, 2, 3] +
    [0, 0, 0, 0, 0x46, 0xFE, 0, 0x12, 0x34, 0x56, 0x78]
  let identity = try VescFirmwareIdentityDecoder.decode(payload)

  #expect(float_view_vesc_firmware_identity_abi_version() == 1)
  #expect(identity.major == 6)
  #expect(identity.minor == 6)
  #expect(identity.testVersion == 9)
  #expect(identity.hardwareName == "H\u{FFFD}")
  #expect(identity.firmwareName == "F\u{FFFD}")
  #expect(identity.hardwareType == 2)
  #expect(identity.controllerUUID == Data(repeating: 7, count: 12))
  #expect(identity.customConfigurationCount == 3)
  #expect(identity.hardwareConfigurationCRC == 0x1234_5678)
}

@Test func rustFirmwareIdentityBridgeMapsEstablishedFailures() throws {
  #expect(throws: VescProtocolError.unsupportedCommand(37)) {
    try VescFirmwareIdentityDecoder.decode([37])
  }
  #expect(throws: VescProtocolError.truncatedPayload) {
    try VescFirmwareIdentityDecoder.decode([0])
  }
  #expect(throws: VescProtocolError.truncatedPayload) {
    try VescFirmwareIdentityDecoder.requireCompatibleABI(2)
  }
}

@Test func rustFirmwareIdentityAdapterRejectsImpossiblePrivateRecords() throws {
  var valid = [UInt8](repeating: 0, count: 28)
  valid[3] = 2
  valid[5] = 3
  valid[7] = 1
  valid[9] = 1
  valid[22 ... 25] = [0x12, 0x34, 0x56, 0x78]
  valid[26] = UInt8(ascii: "H")
  valid[27] = UInt8(ascii: "F")
  let identity = try VescFirmwareIdentityDecoder.firmwareInfo(from: valid)
  #expect(identity.hardwareName == "H")
  #expect(identity.firmwareName == "F")
  #expect(identity.hardwareConfigurationCRC == 0x1234_5678)

  var unknownFlags = valid
  unknownFlags[5] |= 0x80
  #expect(throws: VescProtocolError.truncatedPayload) {
    try VescFirmwareIdentityDecoder.firmwareInfo(from: unknownFlags)
  }
  var mismatchedLength = valid
  mismatchedLength[7] = 2
  #expect(throws: VescProtocolError.truncatedPayload) {
    try VescFirmwareIdentityDecoder.firmwareInfo(from: mismatchedLength)
  }
  var absentCRCWithValue = valid
  absentCRCWithValue[5] &= ~2
  #expect(throws: VescProtocolError.truncatedPayload) {
    try VescFirmwareIdentityDecoder.firmwareInfo(from: absentCRCWithValue)
  }
}
