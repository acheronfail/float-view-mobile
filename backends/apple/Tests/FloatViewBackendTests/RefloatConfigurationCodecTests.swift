@testable import FloatViewBackend
import Foundation
import Testing

private struct ConfigurationManifest: Decodable {
  struct Release: Decodable {
    let version: String
    let schemaFingerprintSha256: String
    let wireSignature: UInt32
    let serializedFieldCount: Int
  }

  let releases: [Release]
}

private func configurationFixtureRoot() -> URL {
  var root = URL(fileURLWithPath: #filePath)
  for _ in 0 ..< 4 {
    root.deleteLastPathComponent()
  }
  return root.appending(path: "fixtures/refloat-configuration")
}

@Test func refloatConfigurationSchemasAndValuesMatchEveryAuditedRelease() throws {
  let root = configurationFixtureRoot()
  let manifest = try JSONDecoder().decode(
    ConfigurationManifest.self,
    from: Data(contentsOf: root.appending(path: "manifest.json"))
  )
  #expect(manifest.releases.count == 9)
  for release in manifest.releases {
    let schema = try RefloatConfigurationSchemaCodec.decodeQtCompressed(
      Data(contentsOf: root.appending(path: "schemas/\(release.version).qcompress"))
    )
    #expect(schema.fingerprint == release.schemaFingerprintSha256)
    #expect(schema.wireSignature == release.wireSignature)
    #expect(schema.fields.count == release.serializedFieldCount)
    let defaults = try Data(contentsOf: root.appending(path: "values/\(release.version)-default.bin"))
    let decoded = try RefloatConfigurationValueCodec.decode(defaults, schema: schema)
    #expect(decoded.count == release.serializedFieldCount)
    #expect(try RefloatConfigurationValueCodec.encode(decoded, schema: schema) == defaults)
  }
}

@Test func refloatConfigurationCodecRejectsEnvelopeSignatureAndRemainder() throws {
  #expect(throws: RefloatConfigurationCodecError.truncatedEnvelope) {
    try RefloatConfigurationSchemaCodec.decodeQtCompressed(Data([0, 0]))
  }
  let root = configurationFixtureRoot()
  let schema = try RefloatConfigurationSchemaCodec.decodeQtCompressed(
    Data(contentsOf: root.appending(path: "schemas/1.2.3.qcompress"))
  )
  var mismatch = try Data(contentsOf: root.appending(path: "values/1.2.3-default.bin"))
  mismatch[0] ^= 0xFF
  #expect(throws: RefloatConfigurationCodecError.signatureMismatch) {
    try RefloatConfigurationValueCodec.decode(mismatch, schema: schema)
  }
  var remainder = try Data(contentsOf: root.appending(path: "values/1.2.3-default.bin"))
  remainder.append(0)
  #expect(throws: RefloatConfigurationCodecError.remainder) {
    try RefloatConfigurationValueCodec.decode(remainder, schema: schema)
  }
}

@Test func refloatConfigurationParserRejectsAdversarialSchemasWithoutPartialState() {
  let cases: [(String, RefloatConfigurationCodecError)] = [
    (
      "<?xml version=\"1.0\"?><!DOCTYPE x [<!ENTITY e SYSTEM \"file:///etc/passwd\">]><ConfigParams>&e;</ConfigParams>",
      .prohibitedXML
    ),
    ("<ConfigParams><Params><x/><x/></Params><SerOrder/><Grouping/></ConfigParams>", .duplicateField),
    (
      "<ConfigParams><Params><x><type>5</type></x></Params><SerOrder><ser>x</ser><ser>x</ser></SerOrder><Grouping/></ConfigParams>",
      .duplicateOrder
    ),
    ("<ConfigParams><Params/><SerOrder><ser>x</ser></SerOrder><Grouping/></ConfigParams>", .missingReference),
    (
      "<ConfigParams><Params><x><type>99</type></x></Params><SerOrder><ser>x</ser></SerOrder><Grouping/></ConfigParams>",
      .unsupportedType
    ),
    (
      "<ConfigParams><Params><x><type>1</type><vTx>99</vTx></x></Params><SerOrder><ser>x</ser></SerOrder><Grouping/></ConfigParams>",
      .unsupportedEncoding
    ),
    (
      "<ConfigParams><Params><x><type>1</type><vTx>7</vTx><valDouble>NaN</valDouble></x></Params><SerOrder><ser>x</ser></SerOrder><Grouping/></ConfigParams>",
      .invalidNumber
    ),
  ]
  for (xml, expected) in cases {
    #expect(throws: expected) { try RefloatConfigurationSchemaCodec.parse(Data(xml.utf8)) }
  }
  let nested = "<ConfigParams>" + String(repeating: "<x>", count: 40) +
    String(repeating: "</x>", count: 40) + "</ConfigParams>"
  #expect(throws: RefloatConfigurationCodecError.structuralLimit) {
    try RefloatConfigurationSchemaCodec.parse(Data(nested.utf8))
  }
}
