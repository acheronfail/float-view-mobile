import CryptoKit
import Foundation
import zlib

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

public struct RefloatConfigurationSchema: Equatable, Sendable, Codable {
  public let fingerprint: String
  public let wireSignature: UInt32
  public let fields: [RefloatConfigurationField]
  public let groups: [RefloatConfigurationGroup]
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

private struct RawField {
  var values: [String: [String]] = [:]

  func first(_ name: String, default fallback: String = "") -> String {
    values[name]?.first ?? fallback
  }
}

private final class ConfigParamsXMLDelegate: NSObject, XMLParserDelegate {
  var fields: [(String, RawField)] = []
  var order: [String] = []
  var groups: [RefloatConfigurationGroup] = []
  var error: RefloatConfigurationCodecError?

  private var stack: [String] = []
  private var contents = ""
  private var elementCount = 0
  private var currentFieldName: String?
  private var currentField = RawField()
  private var currentGroupName: String?
  private var currentSubgroups: [RefloatConfigurationSubgroup] = []
  private var currentSubgroupName: String?
  private var currentSubgroupFields: [String] = []

  func parser(
    _: XMLParser, didStartElement elementName: String, namespaceURI _: String?,
    qualifiedName _: String?, attributes _: [String: String] = [:]
  ) {
    guard error == nil else { return }
    elementCount += 1
    stack.append(elementName)
    contents = ""
    if stack.count > RefloatConfigurationLimits.xmlDepth || elementCount > RefloatConfigurationLimits.xmlElements {
      error = .structuralLimit
      return
    }
    if stack.count == 3, stack[0] == "ConfigParams", stack[1] == "Params" {
      currentFieldName = elementName
      currentField = RawField()
    } else if stack.suffix(2) == ["Grouping", "group"] {
      currentGroupName = nil
      currentSubgroups = []
    } else if stack.suffix(2) == ["group", "subgroup"] {
      currentSubgroupName = nil
      currentSubgroupFields = []
    }
  }

  func parser(_: XMLParser, foundCharacters string: String) {
    guard contents.utf8.count + string.utf8.count <= RefloatConfigurationLimits.descriptionBytes * 2 else {
      error = .structuralLimit
      return
    }
    contents += string
  }

  func parser(
    _: XMLParser, didEndElement elementName: String, namespaceURI _: String?, qualifiedName _: String?
  ) {
    guard error == nil else { return }
    let value = contents
    if stack.count == 4, stack[0] == "ConfigParams", stack[1] == "Params" {
      currentField.values[elementName, default: []].append(value)
    } else if stack == ["ConfigParams", "SerOrder", "ser"] {
      order.append(value)
    } else if stack.suffix(3) == ["group", "subgroup", "subgroupName"] {
      currentSubgroupName = value
    } else if stack.suffix(2) == ["subgroupParams", "param"] {
      currentSubgroupFields.append(value)
    } else if stack.suffix(2) == ["group", "groupName"] {
      currentGroupName = value
    }

    if stack.count == 3, stack[0] == "ConfigParams", stack[1] == "Params",
       let name = currentFieldName
    {
      fields.append((name, currentField))
      currentFieldName = nil
    } else if stack.suffix(2) == ["group", "subgroup"] {
      currentSubgroups.append(.init(name: currentSubgroupName ?? "", fields: currentSubgroupFields))
    } else if stack.suffix(2) == ["Grouping", "group"] {
      groups.append(.init(name: currentGroupName ?? "", subgroups: currentSubgroups))
    }
    stack.removeLast()
    contents = ""
  }

  func parser(_: XMLParser, parseErrorOccurred _: Error) {
    if error == nil { error = .malformedXML }
  }
}

public enum RefloatConfigurationSchemaCodec {
  public static func decodeQtCompressed(_ input: Data) throws -> RefloatConfigurationSchema {
    guard input.count >= 4 else { throw RefloatConfigurationCodecError.truncatedEnvelope }
    guard input.count <= RefloatConfigurationLimits.compressedBytes else {
      throw RefloatConfigurationCodecError.compressedTooLarge
    }
    let expected = input.prefix(4).reduce(0) { ($0 << 8) | Int($1) }
    guard expected <= RefloatConfigurationLimits.expandedBytes else {
      throw RefloatConfigurationCodecError.expandedTooLarge
    }
    let expanded = try inflate(input.dropFirst(4), expectedBytes: expected)
    return try parse(expanded)
  }

  public static func parse(_ xml: Data) throws -> RefloatConfigurationSchema {
    guard xml.count <= RefloatConfigurationLimits.expandedBytes else {
      throw RefloatConfigurationCodecError.expandedTooLarge
    }
    let lowered = String(decoding: xml, as: UTF8.self).lowercased()
    guard !lowered.contains("<!doctype"), !lowered.contains("<!entity") else {
      throw RefloatConfigurationCodecError.prohibitedXML
    }
    let delegate = ConfigParamsXMLDelegate()
    let parser = XMLParser(data: xml)
    parser.shouldProcessNamespaces = false
    parser.shouldReportNamespacePrefixes = false
    parser.shouldResolveExternalEntities = false
    parser.delegate = delegate
    guard parser.parse(), delegate.error == nil else {
      throw delegate.error ?? RefloatConfigurationCodecError.malformedXML
    }
    guard delegate.fields.count <= RefloatConfigurationLimits.fields,
          delegate.groups.count <= RefloatConfigurationLimits.groups,
          delegate.groups.reduce(0, { $0 + $1.subgroups.count }) <= RefloatConfigurationLimits.subgroups
    else { throw RefloatConfigurationCodecError.structuralLimit }
    var rawByName: [String: RawField] = [:]
    for (name, raw) in delegate.fields {
      guard rawByName.updateValue(raw, forKey: name) == nil else {
        throw RefloatConfigurationCodecError.duplicateField
      }
    }
    guard Set(delegate.order).count == delegate.order.count else {
      throw RefloatConfigurationCodecError.duplicateOrder
    }
    var fields: [RefloatConfigurationField] = []
    for name in delegate.order {
      guard let raw = rawByName[name] else { throw RefloatConfigurationCodecError.missingReference }
      try fields.append(normalizedField(name: name, raw: raw))
    }
    let signature = RefloatConfigurationValueCodec.signature(fields: fields)
    let digest = SHA256.hash(data: xml).map { String(format: "%02x", $0) }.joined()
    return .init(fingerprint: digest, wireSignature: signature, fields: fields, groups: delegate.groups)
  }

  private static func inflate(_ bytes: Data.SubSequence, expectedBytes: Int) throws -> Data {
    var stream = z_stream()
    guard inflateInit_(&stream, ZLIB_VERSION, Int32(MemoryLayout<z_stream>.size)) == Z_OK else {
      throw RefloatConfigurationCodecError.decompressionFailed
    }
    defer { inflateEnd(&stream) }
    var output = Data(count: expectedBytes)
    let result = Data(bytes).withUnsafeBytes { input in
      output.withUnsafeMutableBytes { destination in
        stream.next_in = UnsafeMutablePointer(mutating: input.bindMemory(to: Bytef.self).baseAddress)
        stream.avail_in = uInt(input.count)
        stream.next_out = destination.bindMemory(to: Bytef.self).baseAddress
        stream.avail_out = uInt(destination.count)
        return zlib.inflate(&stream, Z_FINISH)
      }
    }
    guard result == Z_STREAM_END else { throw RefloatConfigurationCodecError.decompressionFailed }
    guard Int(stream.total_out) == expectedBytes, stream.avail_in == 0 else {
      throw RefloatConfigurationCodecError.expandedLengthMismatch
    }
    return output
  }

  private static func normalizedField(name: String, raw: RawField) throws -> RefloatConfigurationField {
    try bounded(name, RefloatConfigurationLimits.nameBytes)
    guard let rawType = Int(raw.first("type")), let type = RefloatConfigurationFieldType(rawValue: rawType) else {
      throw RefloatConfigurationCodecError.unsupportedType
    }
    let wireType = raw.values["vTx"]?.first.flatMap(Int.init).flatMap(RefloatConfigurationWireType.init)
    if type == .double || type == .integer, wireType == nil {
      throw RefloatConfigurationCodecError.unsupportedEncoding
    }
    let choices = raw.values["enumNames"] ?? []
    guard choices.count <= RefloatConfigurationLimits.enumChoices else {
      throw RefloatConfigurationCodecError.structuralLimit
    }
    let value: RefloatConfigurationValue
    switch type {
    case .double:
      guard let number = Double(raw.first("valDouble")), number.isFinite else {
        throw RefloatConfigurationCodecError.invalidNumber
      }
      value = .double(number)
    case .integer, .enumeration, .bitfield:
      guard let number = Int64(raw.first("valInt")) else { throw RefloatConfigurationCodecError.invalidNumber }
      value = .integer(number)
    case .boolean:
      guard let number = Int(raw.first("valInt")), number == 0 || number == 1 else {
        throw RefloatConfigurationCodecError.invalidNumber
      }
      value = .boolean(number == 1)
    case .string:
      value = .string(raw.first("valString"))
    }
    let minimum = try optionalFinite(type == .double ? raw.first("minDouble") : raw.first("minInt"))
    let maximum = try optionalFinite(type == .double ? raw.first("maxDouble") : raw.first("maxInt"))
    let step = try optionalFinite(type == .double ? raw.first("stepDouble") : raw.first("stepInt"))
    let scale = try optionalFinite(raw.first("vTxDoubleScale"))
    let description = sanitize(raw.first("description"))
    try bounded(description, RefloatConfigurationLimits.descriptionBytes)
    return .init(
      name: name, longName: raw.first("longName"), type: type, wireType: wireType,
      defaultValue: value, minimum: minimum, maximum: maximum, step: step,
      precision: Int(raw.first("editorDecimalsDouble")), scale: scale,
      suffix: raw.first("suffix"), choices: choices, description: description
    )
  }

  private static func optionalFinite(_ value: String) throws -> Double? {
    guard !value.isEmpty else { return nil }
    guard let result = Double(value), result.isFinite else { throw RefloatConfigurationCodecError.invalidNumber }
    return result
  }

  private static func bounded(_ value: String, _ maximum: Int) throws {
    guard !value.isEmpty, value.utf8.count <= maximum else {
      throw RefloatConfigurationCodecError.invalidMetadata
    }
  }

  private static func sanitize(_ value: String) -> String {
    var result = ""
    var insideTag = false
    for character in value {
      if character == "<" { insideTag = true; result.append(" ") }
      else if character == ">" { insideTag = false; result.append(" ") }
      else if !insideTag { result.append(character) }
    }
    return result.split(whereSeparator: { $0.isWhitespace }).joined(separator: " ")
  }
}

private struct ConfigurationByteReader {
  let bytes: [UInt8]
  var offset = 0
  var remaining: Int {
    bytes.count - offset
  }

  mutating func take(_ count: Int) throws -> ArraySlice<UInt8> {
    guard count >= 0, offset + count <= bytes.count else { throw RefloatConfigurationCodecError.truncatedValues }
    defer { offset += count }
    return bytes[offset ..< offset + count]
  }

  mutating func unsigned(_ count: Int) throws -> UInt64 {
    try take(count).reduce(0) { ($0 << 8) | UInt64($1) }
  }

  mutating func signed(_ count: Int) throws -> Int64 {
    let value = try unsigned(count)
    let shift = UInt64(64 - count * 8)
    return Int64(bitPattern: value << shift) >> Int64(shift)
  }
}

public enum RefloatConfigurationValueCodec {
  public static func signature(fields: [RefloatConfigurationField]) -> UInt32 {
    var input = ""
    for field in fields {
      input += field.name + String(field.type.rawValue) + String(field.wireType?.rawValue ?? 0)
      input += field.choices.joined()
    }
    return crc32c(Data(input.utf8))
  }

  public static func decode(_ data: Data,
                            schema: RefloatConfigurationSchema) throws -> [String: RefloatConfigurationValue]
  {
    var reader = ConfigurationByteReader(bytes: Array(data))
    guard try reader.unsigned(4) == UInt64(schema.wireSignature) else {
      throw RefloatConfigurationCodecError.signatureMismatch
    }
    var values: [String: RefloatConfigurationValue] = [:]
    for field in schema.fields {
      values[field.name] = try decode(field, reader: &reader)
    }
    guard reader.remaining == 0 else { throw RefloatConfigurationCodecError.remainder }
    return values
  }

  public static func encode(
    _ values: [String: RefloatConfigurationValue], schema: RefloatConfigurationSchema
  ) throws -> Data {
    let expected = Set(schema.fields.map(\.name))
    let provided = Set(values.keys)
    guard expected.isSubset(of: provided) else { throw RefloatConfigurationCodecError.missingValue }
    guard provided.isSubset(of: expected) else { throw RefloatConfigurationCodecError.extraValue }
    var output = Data()
    append(UInt64(schema.wireSignature), bytes: 4, to: &output)
    for field in schema.fields {
      try encode(values[field.name]!, field: field, to: &output)
    }
    return output
  }

  private static func decode(
    _ field: RefloatConfigurationField, reader: inout ConfigurationByteReader
  ) throws -> RefloatConfigurationValue {
    switch field.type {
    case .double:
      guard let scale = field.scale, scale > 0 else { throw RefloatConfigurationCodecError.invalidMetadata }
      let raw: Int64
      switch field.wireType {
      case .double16: raw = try reader.signed(2)
      case .double32: raw = try reader.signed(4)
      default: throw RefloatConfigurationCodecError.unsupportedEncoding
      }
      return .double(Double(raw) / scale)
    case .integer:
      let raw: Int64
      switch field.wireType {
      case .uint8: raw = try Int64(reader.unsigned(1))
      case .int8: raw = try reader.signed(1)
      case .uint16: raw = try Int64(reader.unsigned(2))
      case .int16: raw = try reader.signed(2)
      case .uint32: raw = try Int64(reader.unsigned(4))
      case .int32: raw = try reader.signed(4)
      default: throw RefloatConfigurationCodecError.unsupportedEncoding
      }
      return .integer(raw)
    case .enumeration:
      return try .integer(reader.signed(1))
    case .bitfield:
      return try .integer(Int64(reader.unsigned(1)))
    case .boolean:
      let raw = try reader.signed(1)
      guard raw == 0 || raw == 1 else { throw RefloatConfigurationCodecError.invalidValue }
      return .boolean(raw == 1)
    case .string:
      let start = reader.offset
      guard let end = reader.bytes[start...].firstIndex(of: 0),
            end - start <= RefloatConfigurationLimits.stringValueBytes
      else { throw RefloatConfigurationCodecError.truncatedValues }
      let bytes = try reader.take(end - start + 1).dropLast()
      guard let value = String(bytes: bytes, encoding: .utf8) else {
        throw RefloatConfigurationCodecError.invalidValue
      }
      return .string(value)
    }
  }

  private static func encode(
    _ value: RefloatConfigurationValue, field: RefloatConfigurationField, to output: inout Data
  ) throws {
    switch (field.type, value) {
    case let (.double, .double(number)):
      guard number.isFinite, within(number, field: field), let scale = field.scale, scale > 0 else {
        throw RefloatConfigurationCodecError.invalidValue
      }
      let scaled = number * scale
      guard scaled.rounded() == scaled else { throw RefloatConfigurationCodecError.invalidValue }
      let raw = Int64(scaled)
      switch field.wireType {
      case .double16 where raw >= Int64(Int16.min) && raw <= Int64(Int16.max): append(
          UInt64(bitPattern: raw),
          bytes: 2,
          to: &output
        )
      case .double32 where raw >= Int64(Int32.min) && raw <= Int64(Int32.max): append(
          UInt64(bitPattern: raw),
          bytes: 4,
          to: &output
        )
      default: throw RefloatConfigurationCodecError.invalidValue
      }
    case let (.integer, .integer(number)):
      guard within(Double(number), field: field) else { throw RefloatConfigurationCodecError.invalidValue }
      let bounds: (Int64, Int64, Int)
      switch field.wireType {
      case .uint8: bounds = (0, 255, 1)
      case .int8: bounds = (-128, 127, 1)
      case .uint16: bounds = (0, 65_535, 2)
      case .int16: bounds = (-32_768, 32_767, 2)
      case .uint32: bounds = (0, 4_294_967_295, 4)
      case .int32: bounds = (-2_147_483_648, 2_147_483_647, 4)
      default: throw RefloatConfigurationCodecError.unsupportedEncoding
      }
      guard number >= bounds.0, number <= bounds.1 else { throw RefloatConfigurationCodecError.invalidValue }
      append(UInt64(bitPattern: number), bytes: bounds.2, to: &output)
    case let (.enumeration, .integer(number)):
      guard number >= 0, number < field.choices.count,
            number <= 127 else { throw RefloatConfigurationCodecError.invalidValue }
      append(UInt64(number), bytes: 1, to: &output)
    case let (.bitfield, .integer(number)):
      guard number >= 0, number <= 255 else { throw RefloatConfigurationCodecError.invalidValue }
      append(UInt64(number), bytes: 1, to: &output)
    case let (.boolean, .boolean(boolean)):
      output.append(boolean ? 1 : 0)
    case let (.string, .string(string)):
      guard string.utf8.count <= RefloatConfigurationLimits.stringValueBytes, !string.utf8.contains(0) else {
        throw RefloatConfigurationCodecError.invalidValue
      }
      output.append(contentsOf: string.utf8); output.append(0)
    default:
      throw RefloatConfigurationCodecError.invalidValue
    }
  }

  private static func within(_ value: Double, field: RefloatConfigurationField) -> Bool {
    (field.minimum == nil || value >= field.minimum!) && (field.maximum == nil || value <= field.maximum!)
  }

  private static func append(_ value: UInt64, bytes: Int, to data: inout Data) {
    for shift in stride(from: (bytes - 1) * 8, through: 0, by: -8) {
      data.append(UInt8(truncatingIfNeeded: value >> UInt64(shift)))
    }
  }

  private static func crc32c(_ data: Data) -> UInt32 {
    var crc: UInt32 = 0xFFFFFFFF
    for byte in data {
      crc ^= UInt32(byte)
      for _ in 0 ..< 8 {
        let mask = UInt32(bitPattern: -Int32(crc & 1))
        crc = (crc >> 1) ^ (0x82F63B78 & mask)
      }
    }
    return ~crc
  }
}
