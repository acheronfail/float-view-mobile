import CVescRefloatConfigurationCodec
import Foundation

public enum RefloatConfigurationLimits {
  public static let compressedBytes = 32 * 1024
  public static let expandedBytes = 384 * 1024
  public static let fields = 512
  public static let groups = 16
  public static let subgroups = 64
  public static let enumChoices = 128
  public static let nameBytes = 128
  public static let descriptionBytes = 4096
  public static let stringValueBytes = 4096
  public static let xmlDepth = 32
  public static let xmlElements = 12_000
}

public enum RefloatConfigurationCodecError: String, Error, Codable, Sendable {
  case truncatedEnvelope
  case compressedTooLarge
  case expandedTooLarge
  case expandedLengthMismatch
  case decompressionFailed
  case prohibitedXML
  case malformedXML
  case structuralLimit
  case duplicateField
  case duplicateOrder
  case missingReference
  case unsupportedType
  case unsupportedEncoding
  case invalidMetadata
  case invalidNumber
  case signatureMismatch
  case truncatedValues
  case remainder
  case missingValue
  case extraValue
  case invalidValue
}

public enum RefloatConfigurationFieldType: Int, Codable, Sendable {
  case double = 1
  case integer = 2
  case string = 3
  case enumeration = 4
  case boolean = 5
  case bitfield = 6
}

public enum RefloatConfigurationWireType: Int, Codable, Sendable {
  case uint8 = 1
  case int8 = 2
  case uint16 = 3
  case int16 = 4
  case uint32 = 5
  case int32 = 6
  case double16 = 7
  case double32 = 8
  case double32Auto = 9
}

public enum RefloatConfigurationValue: Equatable, Sendable, Codable {
  case double(Double)
  case integer(Int64)
  case boolean(Bool)
  case string(String)
}

public struct RefloatConfigurationField: Equatable, Sendable, Codable {
  public let name: String
  public let longName: String
  public let type: RefloatConfigurationFieldType
  public let wireType: RefloatConfigurationWireType?
  public let defaultValue: RefloatConfigurationValue
  public let minimum: Double?
  public let maximum: Double?
  public let step: Double?
  public let precision: Int?
  public let scale: Double?
  public let suffix: String
  public let choices: [String]
  public let description: String
}

public struct RefloatConfigurationSubgroup: Equatable, Sendable, Codable {
  public let name: String
  public let fields: [String]
}

public struct RefloatConfigurationGroup: Equatable, Sendable, Codable {
  public let name: String
  public let subgroups: [RefloatConfigurationSubgroup]
}

public struct RefloatConfigurationSchema: Sendable, Codable, Equatable {
  public let fingerprint: String
  public let wireSignature: UInt32
  public let fields: [RefloatConfigurationField]
  public let groups: [RefloatConfigurationGroup]
  let canonicalRecord: Data?

  public init(
    fingerprint: String, wireSignature: UInt32, fields: [RefloatConfigurationField],
    groups: [RefloatConfigurationGroup]
  ) {
    self.fingerprint = fingerprint
    self.wireSignature = wireSignature
    self.fields = fields
    self.groups = groups
    canonicalRecord = nil
  }

  init(
    fingerprint: String, wireSignature: UInt32, fields: [RefloatConfigurationField],
    groups: [RefloatConfigurationGroup], canonicalRecord: Data
  ) {
    self.fingerprint = fingerprint
    self.wireSignature = wireSignature
    self.fields = fields
    self.groups = groups
    self.canonicalRecord = canonicalRecord
  }

  enum CodingKeys: String, CodingKey { case fingerprint, wireSignature, fields, groups }

  public init(from decoder: any Decoder) throws {
    let container = try decoder.container(keyedBy: CodingKeys.self)
    fingerprint = try container.decode(String.self, forKey: .fingerprint)
    wireSignature = try container.decode(UInt32.self, forKey: .wireSignature)
    fields = try container.decode([RefloatConfigurationField].self, forKey: .fields)
    groups = try container.decode([RefloatConfigurationGroup].self, forKey: .groups)
    canonicalRecord = nil
  }

  public func encode(to encoder: any Encoder) throws {
    var container = encoder.container(keyedBy: CodingKeys.self)
    try container.encode(fingerprint, forKey: .fingerprint)
    try container.encode(wireSignature, forKey: .wireSignature)
    try container.encode(fields, forKey: .fields)
    try container.encode(groups, forKey: .groups)
  }

  public static func == (left: Self, right: Self) -> Bool {
    left.fingerprint == right.fingerprint && left.wireSignature == right.wireSignature
      && left.fields == right.fields && left.groups == right.groups
  }
}

public struct RefloatConfigurationSchemaIdentity: Equatable, Sendable, Codable {
  public let packageName: String
  public let packageVersion: String
  public let fingerprint: String
  public let wireSignature: UInt32
}

public struct RefloatConfigurationPage: Equatable, Sendable, Codable {
  public let identity: RefloatConfigurationSchemaIdentity
  public let group: String
  public let subgroup: String
  public let fields: [RefloatConfigurationField]
  public let nextCursor: String?
}

public struct RefloatConfigurationValues: Equatable, Sendable, Codable {
  public let revision: UInt64
  public let source: String
  public let values: [String: RefloatConfigurationValue]
}

public enum RefloatConfigurationOperation: String, Sendable, Codable {
  case schema, readCurrent, readDefault, write, applyTune
}

public enum RefloatConfigurationAvailability: String, Sendable, Codable {
  case loading, ready, unavailable
}

private enum CanonicalRecord {
  static let version: UInt8 = 1
  static let schemaKind: UInt8 = 1
  static let valuesKind: UInt8 = 2
  static let schemaHeaderBytes = 48
  static let valuesHeaderBytes = 12
  static let decimalKind: UInt8 = 1
  static let integerKind: UInt8 = 2
  static let booleanKind: UInt8 = 3
  static let stringKind: UInt8 = 4
}

private struct CanonicalReader {
  let bytes: [UInt8]
  var offset = 0

  mutating func header(kind: UInt8, minimumBytes: Int) throws {
    guard bytes.count >= minimumBytes,
          try take(4).elementsEqual([CanonicalRecord.version, kind, 0, 0]),
          try uint32() == UInt32(bytes.count)
    else { throw RefloatConfigurationCodecError.invalidMetadata }
  }

  mutating func take(_ count: Int) throws -> ArraySlice<UInt8> {
    guard count >= 0, count <= bytes.count - offset else {
      throw RefloatConfigurationCodecError.invalidMetadata
    }
    let start = offset
    offset += count
    return bytes[start ..< offset]
  }

  mutating func uint8() throws -> UInt8 {
    try take(1).first!
  }

  mutating func uint16() throws -> UInt16 {
    try take(2).reduce(0) { ($0 << 8) | UInt16($1) }
  }

  mutating func uint32() throws -> UInt32 {
    try take(4).reduce(0) { ($0 << 8) | UInt32($1) }
  }

  mutating func uint64() throws -> UInt64 {
    try take(8).reduce(0) { ($0 << 8) | UInt64($1) }
  }

  mutating func int32() throws -> Int32 {
    try Int32(bitPattern: uint32())
  }

  mutating func int64() throws -> Int64 {
    try Int64(bitPattern: uint64())
  }

  mutating func double() throws -> Double {
    let value = try Double(bitPattern: uint64())
    guard value.isFinite else { throw RefloatConfigurationCodecError.invalidNumber }
    return value
  }

  mutating func string(maximum: Int, allowEmpty: Bool) throws -> String {
    let count = try Int(uint32())
    guard count <= maximum, allowEmpty || count > 0 else {
      throw RefloatConfigurationCodecError.invalidMetadata
    }
    let contents = try take(count)
    guard !contents.contains(0), let value = String(bytes: contents, encoding: .utf8) else {
      throw RefloatConfigurationCodecError.invalidMetadata
    }
    return value
  }

  mutating func value(kind: UInt8) throws -> RefloatConfigurationValue {
    switch kind {
    case CanonicalRecord.decimalKind: return try .double(double())
    case CanonicalRecord.integerKind: return try .integer(int64())
    case CanonicalRecord.booleanKind:
      switch try uint8() {
      case 0: return .boolean(false)
      case 1: return .boolean(true)
      default: throw RefloatConfigurationCodecError.invalidValue
      }
    case CanonicalRecord.stringKind:
      return try .string(string(maximum: RefloatConfigurationLimits.stringValueBytes, allowEmpty: true))
    default: throw RefloatConfigurationCodecError.invalidValue
    }
  }

  func finished() throws {
    guard offset == bytes.count else { throw RefloatConfigurationCodecError.remainder }
  }
}

public enum RefloatConfigurationSchemaCodec {
  static func requireCompatibleABI(_ actual: UInt32) throws {
    guard actual == UInt32(FLOAT_VIEW_REFLOAT_CONFIGURATION_CODEC_ABI_VERSION) else {
      throw RefloatConfigurationCodecError.invalidMetadata
    }
  }

  public static func decodeQtCompressed(_ input: Data) throws -> RefloatConfigurationSchema {
    try requireABI()
    var record = Data(count: Int(FLOAT_VIEW_REFLOAT_CONFIGURATION_CODEC_MAX_SCHEMA_RECORD_BYTES))
    let capacity = record.count
    var length = 0
    let status = input.withUnsafeBytes { inputBytes in
      record.withUnsafeMutableBytes { recordBytes in
        float_view_refloat_configuration_codec_decode_schema(
          inputBytes.bindMemory(to: UInt8.self).baseAddress, input.count,
          recordBytes.bindMemory(to: UInt8.self).baseAddress, capacity, &length
        )
      }
    }
    try check(status)
    guard length <= record.count else { throw RefloatConfigurationCodecError.invalidMetadata }
    record.removeSubrange(length ..< record.count)
    return try schema(from: record)
  }

  private static func schema(from record: Data) throws -> RefloatConfigurationSchema {
    var reader = CanonicalReader(bytes: Array(record))
    try reader.header(kind: CanonicalRecord.schemaKind, minimumBytes: CanonicalRecord.schemaHeaderBytes)
    let fieldCount = try Int(reader.uint16())
    let groupCount = try Int(reader.uint16())
    guard fieldCount <= RefloatConfigurationLimits.fields, groupCount <= RefloatConfigurationLimits.groups else {
      throw RefloatConfigurationCodecError.structuralLimit
    }
    let signature = try reader.uint32()
    let fingerprint = try reader.take(32).map { String(format: "%02x", $0) }.joined()
    var fields: [RefloatConfigurationField] = []
    for _ in 0 ..< fieldCount {
      let name = try reader.string(maximum: RefloatConfigurationLimits.nameBytes, allowEmpty: false)
      let longName = try reader.string(maximum: RefloatConfigurationLimits.descriptionBytes, allowEmpty: true)
      guard let type = try RefloatConfigurationFieldType(rawValue: Int(reader.uint8())) else {
        throw RefloatConfigurationCodecError.unsupportedType
      }
      let rawWire = try Int(reader.uint8())
      let wireType: RefloatConfigurationWireType?
      if rawWire == 0 { wireType = nil }
      else {
        guard let parsed = RefloatConfigurationWireType(rawValue: rawWire) else {
          throw RefloatConfigurationCodecError.unsupportedEncoding
        }
        wireType = parsed
      }
      let defaultKind = try reader.uint8()
      let flags = try reader.uint8()
      guard flags & 0xE0 == 0 else { throw RefloatConfigurationCodecError.invalidMetadata }
      let defaultValue = try reader.value(kind: defaultKind)
      let minimum = flags & 1 == 0 ? nil : try reader.double()
      let maximum = flags & 2 == 0 ? nil : try reader.double()
      let step = flags & 4 == 0 ? nil : try reader.double()
      let precision = try flags & 8 == 0 ? nil : Int(reader.int32())
      let scale = flags & 16 == 0 ? nil : try reader.double()
      let suffix = try reader.string(maximum: RefloatConfigurationLimits.descriptionBytes, allowEmpty: true)
      let choiceCount = try Int(reader.uint16())
      guard choiceCount <= RefloatConfigurationLimits.enumChoices, try reader.uint16() == 0 else {
        throw RefloatConfigurationCodecError.structuralLimit
      }
      var choices: [String] = []
      for _ in 0 ..< choiceCount {
        try choices.append(reader.string(maximum: RefloatConfigurationLimits.descriptionBytes, allowEmpty: true))
      }
      let description = try reader.string(
        maximum: RefloatConfigurationLimits.descriptionBytes, allowEmpty: false
      )
      fields.append(.init(
        name: name, longName: longName, type: type, wireType: wireType,
        defaultValue: defaultValue, minimum: minimum, maximum: maximum, step: step,
        precision: precision, scale: scale, suffix: suffix, choices: choices, description: description
      ))
    }
    var groups: [RefloatConfigurationGroup] = []
    var subgroupTotal = 0
    for _ in 0 ..< groupCount {
      let name = try reader.string(maximum: RefloatConfigurationLimits.descriptionBytes, allowEmpty: true)
      let subgroupCount = try Int(reader.uint16())
      guard try reader.uint16() == 0 else { throw RefloatConfigurationCodecError.invalidMetadata }
      subgroupTotal += subgroupCount
      guard subgroupTotal <= RefloatConfigurationLimits.subgroups else {
        throw RefloatConfigurationCodecError.structuralLimit
      }
      var subgroups: [RefloatConfigurationSubgroup] = []
      for _ in 0 ..< subgroupCount {
        let subgroupName = try reader.string(
          maximum: RefloatConfigurationLimits.descriptionBytes, allowEmpty: true
        )
        let referenceCount = try Int(reader.uint16())
        guard referenceCount <= RefloatConfigurationLimits.fields, try reader.uint16() == 0 else {
          throw RefloatConfigurationCodecError.structuralLimit
        }
        var references: [String] = []
        for _ in 0 ..< referenceCount {
          try references.append(reader.string(
            maximum: RefloatConfigurationLimits.descriptionBytes, allowEmpty: true
          ))
        }
        subgroups.append(.init(name: subgroupName, fields: references))
      }
      groups.append(.init(name: name, subgroups: subgroups))
    }
    try reader.finished()
    return .init(
      fingerprint: fingerprint, wireSignature: signature, fields: fields, groups: groups,
      canonicalRecord: record
    )
  }
}

public enum RefloatConfigurationValueCodec {
  public static func decode(
    _ data: Data, schema: RefloatConfigurationSchema
  ) throws -> [String: RefloatConfigurationValue] {
    guard let schemaRecord = schema.canonicalRecord else {
      throw RefloatConfigurationCodecError.invalidMetadata
    }
    try requireABI()
    var record = Data(count: Int(FLOAT_VIEW_REFLOAT_CONFIGURATION_CODEC_MAX_VALUE_RECORD_BYTES))
    let capacity = record.count
    var length = 0
    let status = schemaRecord.withUnsafeBytes { schemaBytes in
      data.withUnsafeBytes { dataBytes in
        record.withUnsafeMutableBytes { recordBytes in
          float_view_refloat_configuration_codec_decode_values(
            schemaBytes.bindMemory(to: UInt8.self).baseAddress, schemaRecord.count,
            dataBytes.bindMemory(to: UInt8.self).baseAddress, data.count,
            recordBytes.bindMemory(to: UInt8.self).baseAddress, capacity, &length
          )
        }
      }
    }
    try check(status)
    guard length <= record.count else { throw RefloatConfigurationCodecError.invalidMetadata }
    record.removeSubrange(length ..< record.count)
    return try values(from: record)
  }

  public static func encode(
    _ values: [String: RefloatConfigurationValue], schema: RefloatConfigurationSchema
  ) throws -> Data {
    guard let schemaRecord = schema.canonicalRecord else {
      throw RefloatConfigurationCodecError.invalidMetadata
    }
    try requireABI()
    let valueRecord = try record(for: values)
    var output = Data(count: Int(FLOAT_VIEW_REFLOAT_CONFIGURATION_CODEC_MAX_VALUE_PAYLOAD_BYTES))
    let capacity = output.count
    var length = 0
    let status = schemaRecord.withUnsafeBytes { schemaBytes in
      valueRecord.withUnsafeBytes { valueBytes in
        output.withUnsafeMutableBytes { outputBytes in
          float_view_refloat_configuration_codec_encode_values(
            schemaBytes.bindMemory(to: UInt8.self).baseAddress, schemaRecord.count,
            valueBytes.bindMemory(to: UInt8.self).baseAddress, valueRecord.count,
            outputBytes.bindMemory(to: UInt8.self).baseAddress, capacity, &length
          )
        }
      }
    }
    try check(status)
    guard length <= output.count else { throw RefloatConfigurationCodecError.invalidMetadata }
    output.removeSubrange(length ..< output.count)
    return output
  }

  private static func values(from record: Data) throws -> [String: RefloatConfigurationValue] {
    var reader = CanonicalReader(bytes: Array(record))
    try reader.header(kind: CanonicalRecord.valuesKind, minimumBytes: CanonicalRecord.valuesHeaderBytes)
    let count = try Int(reader.uint16())
    guard count <= RefloatConfigurationLimits.fields, try reader.uint16() == 0 else {
      throw RefloatConfigurationCodecError.structuralLimit
    }
    var result: [String: RefloatConfigurationValue] = [:]
    for _ in 0 ..< count {
      let name = try reader.string(maximum: RefloatConfigurationLimits.nameBytes, allowEmpty: false)
      let kind = try reader.uint8()
      guard try reader.take(3).allSatisfy({ $0 == 0 }), result[name] == nil else {
        throw RefloatConfigurationCodecError.invalidMetadata
      }
      result[name] = try reader.value(kind: kind)
    }
    try reader.finished()
    return result
  }

  private static func record(for values: [String: RefloatConfigurationValue]) throws -> Data {
    guard values.count <= RefloatConfigurationLimits.fields, let count = UInt16(exactly: values.count) else {
      throw RefloatConfigurationCodecError.structuralLimit
    }
    var record = Data([CanonicalRecord.version, CanonicalRecord.valuesKind, 0, 0, 0, 0, 0, 0])
    append(count, to: &record)
    record.append(contentsOf: [0, 0])
    for name in values.keys.sorted() {
      guard let value = values[name] else { throw RefloatConfigurationCodecError.missingValue }
      try append(name, maximum: RefloatConfigurationLimits.nameBytes, allowEmpty: false, to: &record)
      switch value {
      case let .double(number):
        guard number.isFinite else { throw RefloatConfigurationCodecError.invalidNumber }
        record.append(contentsOf: [CanonicalRecord.decimalKind, 0, 0, 0])
        append(number.bitPattern, to: &record)
      case let .integer(number):
        record.append(contentsOf: [CanonicalRecord.integerKind, 0, 0, 0])
        append(UInt64(bitPattern: number), to: &record)
      case let .boolean(value):
        record.append(contentsOf: [CanonicalRecord.booleanKind, 0, 0, 0, value ? 1 : 0])
      case let .string(value):
        record.append(contentsOf: [CanonicalRecord.stringKind, 0, 0, 0])
        try append(value, maximum: RefloatConfigurationLimits.stringValueBytes, allowEmpty: true, to: &record)
      }
    }
    guard record.count <= Int(FLOAT_VIEW_REFLOAT_CONFIGURATION_CODEC_MAX_VALUE_RECORD_BYTES),
          let length = UInt32(exactly: record.count)
    else { throw RefloatConfigurationCodecError.structuralLimit }
    for (index, byte) in length.bigEndianBytes.enumerated() {
      record[4 + index] = byte
    }
    return record
  }

  private static func append(
    _ value: String, maximum: Int, allowEmpty: Bool, to output: inout Data
  ) throws {
    let bytes = Array(value.utf8)
    guard bytes.count <= maximum, allowEmpty || !bytes.isEmpty, !bytes.contains(0),
          let count = UInt32(exactly: bytes.count)
    else { throw RefloatConfigurationCodecError.invalidValue }
    append(count, to: &output)
    output.append(contentsOf: bytes)
  }

  private static func append<T: FixedWidthInteger>(_ value: T, to output: inout Data) {
    output.append(contentsOf: value.bigEndianBytes)
  }
}

private extension FixedWidthInteger {
  var bigEndianBytes: [UInt8] {
    withUnsafeBytes(of: bigEndian) { Array($0) }
  }
}

private func requireABI() throws {
  try RefloatConfigurationSchemaCodec.requireCompatibleABI(
    float_view_refloat_configuration_codec_abi_version()
  )
}

private func check(_ status: UInt32) throws {
  switch status {
  case UInt32(FLOAT_VIEW_REFLOAT_CONFIGURATION_CODEC_STATUS_OK): return
  case UInt32(FLOAT_VIEW_REFLOAT_CONFIGURATION_CODEC_STATUS_TRUNCATED_ENVELOPE):
    throw RefloatConfigurationCodecError.truncatedEnvelope
  case UInt32(FLOAT_VIEW_REFLOAT_CONFIGURATION_CODEC_STATUS_COMPRESSED_TOO_LARGE):
    throw RefloatConfigurationCodecError.compressedTooLarge
  case UInt32(FLOAT_VIEW_REFLOAT_CONFIGURATION_CODEC_STATUS_EXPANDED_TOO_LARGE):
    throw RefloatConfigurationCodecError.expandedTooLarge
  case UInt32(FLOAT_VIEW_REFLOAT_CONFIGURATION_CODEC_STATUS_EXPANDED_LENGTH_MISMATCH):
    throw RefloatConfigurationCodecError.expandedLengthMismatch
  case UInt32(FLOAT_VIEW_REFLOAT_CONFIGURATION_CODEC_STATUS_DECOMPRESSION_FAILED):
    throw RefloatConfigurationCodecError.decompressionFailed
  case UInt32(FLOAT_VIEW_REFLOAT_CONFIGURATION_CODEC_STATUS_PROHIBITED_XML):
    throw RefloatConfigurationCodecError.prohibitedXML
  case UInt32(FLOAT_VIEW_REFLOAT_CONFIGURATION_CODEC_STATUS_MALFORMED_XML):
    throw RefloatConfigurationCodecError.malformedXML
  case UInt32(FLOAT_VIEW_REFLOAT_CONFIGURATION_CODEC_STATUS_STRUCTURAL_LIMIT):
    throw RefloatConfigurationCodecError.structuralLimit
  case UInt32(FLOAT_VIEW_REFLOAT_CONFIGURATION_CODEC_STATUS_DUPLICATE_FIELD):
    throw RefloatConfigurationCodecError.duplicateField
  case UInt32(FLOAT_VIEW_REFLOAT_CONFIGURATION_CODEC_STATUS_DUPLICATE_ORDER):
    throw RefloatConfigurationCodecError.duplicateOrder
  case UInt32(FLOAT_VIEW_REFLOAT_CONFIGURATION_CODEC_STATUS_MISSING_REFERENCE):
    throw RefloatConfigurationCodecError.missingReference
  case UInt32(FLOAT_VIEW_REFLOAT_CONFIGURATION_CODEC_STATUS_UNSUPPORTED_TYPE):
    throw RefloatConfigurationCodecError.unsupportedType
  case UInt32(FLOAT_VIEW_REFLOAT_CONFIGURATION_CODEC_STATUS_UNSUPPORTED_ENCODING):
    throw RefloatConfigurationCodecError.unsupportedEncoding
  case UInt32(FLOAT_VIEW_REFLOAT_CONFIGURATION_CODEC_STATUS_INVALID_NUMBER):
    throw RefloatConfigurationCodecError.invalidNumber
  case UInt32(FLOAT_VIEW_REFLOAT_CONFIGURATION_CODEC_STATUS_SIGNATURE_MISMATCH):
    throw RefloatConfigurationCodecError.signatureMismatch
  case UInt32(FLOAT_VIEW_REFLOAT_CONFIGURATION_CODEC_STATUS_TRUNCATED_VALUES):
    throw RefloatConfigurationCodecError.truncatedValues
  case UInt32(FLOAT_VIEW_REFLOAT_CONFIGURATION_CODEC_STATUS_REMAINDER):
    throw RefloatConfigurationCodecError.remainder
  case UInt32(FLOAT_VIEW_REFLOAT_CONFIGURATION_CODEC_STATUS_MISSING_VALUE):
    throw RefloatConfigurationCodecError.missingValue
  case UInt32(FLOAT_VIEW_REFLOAT_CONFIGURATION_CODEC_STATUS_EXTRA_VALUE):
    throw RefloatConfigurationCodecError.extraValue
  case UInt32(FLOAT_VIEW_REFLOAT_CONFIGURATION_CODEC_STATUS_INVALID_VALUE):
    throw RefloatConfigurationCodecError.invalidValue
  default: throw RefloatConfigurationCodecError.invalidMetadata
  }
}
