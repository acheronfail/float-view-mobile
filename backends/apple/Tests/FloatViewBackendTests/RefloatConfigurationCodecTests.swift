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

@Test func refloatConfigurationAdapterRejectsSchemasWithoutTheSelectedCanonicalRecord() throws {
  let root = configurationFixtureRoot()
  let selected = try RefloatConfigurationSchemaCodec.decodeQtCompressed(
    Data(contentsOf: root.appending(path: "schemas/1.2.3.qcompress"))
  )
  let reconstructed = RefloatConfigurationSchema(
    fingerprint: selected.fingerprint, wireSignature: selected.wireSignature,
    fields: selected.fields, groups: selected.groups
  )
  let values = try Data(contentsOf: root.appending(path: "values/1.2.3-default.bin"))
  #expect(throws: RefloatConfigurationCodecError.invalidMetadata) {
    try RefloatConfigurationValueCodec.decode(values, schema: reconstructed)
  }

  var corruptRecord = try #require(selected.canonicalRecord)
  corruptRecord[2] = 1
  let corrupt = RefloatConfigurationSchema(
    fingerprint: selected.fingerprint, wireSignature: selected.wireSignature,
    fields: selected.fields, groups: selected.groups, canonicalRecord: corruptRecord
  )
  #expect(throws: RefloatConfigurationCodecError.invalidMetadata) {
    try RefloatConfigurationValueCodec.decode(values, schema: corrupt)
  }

  let persisted = try JSONEncoder().encode(selected)
  let decoded = try JSONDecoder().decode(RefloatConfigurationSchema.self, from: persisted)
  #expect(decoded == selected)
  #expect(decoded.canonicalRecord == nil)
  #expect(throws: RefloatConfigurationCodecError.invalidMetadata) {
    try RefloatConfigurationValueCodec.decode(values, schema: decoded)
  }
}

@Test func refloatConfigurationAdapterRejectsABIDrift() {
  #expect(throws: RefloatConfigurationCodecError.invalidMetadata) {
    try RefloatConfigurationSchemaCodec.requireCompatibleABI(2)
  }
}
