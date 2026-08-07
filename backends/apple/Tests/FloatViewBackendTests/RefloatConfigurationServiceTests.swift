@testable import FloatViewBackend
import Foundation
import Testing

private final class MemoryRefloatSchemaCache: RefloatSchemaCachePort, @unchecked Sendable {
  var values: [RefloatSchemaCacheIdentity: Data] = [:]
  var removals = 0
  func cachedRefloatSchema(identity: RefloatSchemaCacheIdentity) -> Data? {
    values[identity]
  }

  func storeRefloatSchema(_ data: Data, identity: RefloatSchemaCacheIdentity) {
    values[identity] = data
  }

  func removeCachedRefloatSchema(identity: RefloatSchemaCacheIdentity) {
    values.removeValue(forKey: identity); removals += 1
  }
}

@Test func configurationServiceAssemblesAuditedSchemaThenLoadsCurrentValues() throws {
  var root = URL(fileURLWithPath: #filePath)
  for _ in 0 ..< 4 {
    root.deleteLastPathComponent()
  }
  root.append(path: "fixtures/refloat-configuration")
  let compressed = try Data(contentsOf: root.appending(path: "schemas/1.2.3.qcompress"))
  let current = try Data(contentsOf: root.appending(path: "values/1.2.3-current.bin"))
  let service = RefloatConfigurationService()
  let generation = VescRequestGeneration(rawValue: 9)
  let firmware = FirmwareRecord(
    major: 6, minor: 6, hardwareName: "THOR300", firmwareName: nil,
    customConfigurationCount: 1, hardwareConfigurationCRC: 123
  )
  var effect = service.begin(
    route: .can(42), generation: generation, firmware: firmware,
    reportedVersion: "1.2.3", writable: true, flywheel: false
  )
  while case let .schemaChunk(route, length, offset, operation) = effect {
    let end = min(offset + length, compressed.count)
    let response = VescCustomConfigurationResponse.schemaChunk(
      index: 0, totalLength: compressed.count, offset: offset,
      bytes: compressed.subdata(in: offset ..< end)
    )
    effect = try service.accept(
      response,
      context: .refloatConfigurationSchema(
        route: .init(route), index: 0, offset: offset, operation: operation
      ),
      generation: generation
    )
  }
  guard case let .values(route, defaults, operation) = effect else {
    Issue.record("Expected automatic current-value read after schema")
    return
  }
  #expect(route == .can(42))
  #expect(!defaults)
  _ = try service.accept(
    .values(command: .getCustomConfiguration, index: 0, bytes: current),
    context: .refloatConfigurationValues(
      route: .can(42), index: 0, defaults: false, operation: operation
    ),
    generation: generation
  )
  let summary = service.summary()
  #expect(summary.availability == .ready)
  #expect(summary.packageVersion == "1.2.3")
  #expect(summary.currentLoaded)
  #expect(summary.writable)
  #expect(service.loadedValues(defaults: false)?.values.count == 167)

  let writeBytes = try Data(contentsOf: root.appending(path: "values/1.2.3-write.bin"))
  let readbackBytes = try Data(contentsOf: root.appending(path: "values/1.2.3-readback.bin"))
  let schema = try #require(service.loadedSchema())
  let draft = try RefloatConfigurationValueCodec.decode(writeBytes, schema: schema)
  let baseRevision = try #require(service.loadedValues(defaults: false)?.revision)
  for (version, fingerprint, signature, revision) in [
    ("1.2.2", schema.fingerprint, schema.wireSignature, baseRevision),
    ("1.2.3", "0" + schema.fingerprint.dropFirst(), schema.wireSignature, baseRevision),
    ("1.2.3", schema.fingerprint, schema.wireSignature &+ 1, baseRevision),
    ("1.2.3", schema.fingerprint, schema.wireSignature, baseRevision &+ 1),
  ] {
    #expect(throws: RefloatConfigurationServiceError.invalidState) {
      try service.requestWrite(
        packageVersion: String(version), fingerprint: String(fingerprint), wireSignature: signature,
        baseRevision: revision, values: draft
      )
    }
  }
  let write = try service.requestWrite(
    packageVersion: "1.2.3", fingerprint: schema.fingerprint,
    wireSignature: schema.wireSignature, baseRevision: baseRevision, values: draft
  )
  guard case let .write(writeRoute, payload, writeOperation) = write else {
    Issue.record("Expected complete configuration write")
    return
  }
  #expect(writeRoute == .can(42))
  #expect(payload == writeBytes)
  let verification = try service.accept(
    .writeAcknowledgement,
    context: .refloatConfigurationWrite(
      route: .can(42), index: 0, operation: writeOperation
    ),
    generation: generation
  )
  #expect(verification == .verification(route: .can(42), operation: writeOperation))
  _ = try service.accept(
    .values(command: .getCustomConfiguration, index: 0, bytes: readbackBytes),
    context: .refloatConfigurationVerification(
      route: .can(42), index: 0, operation: writeOperation
    ),
    generation: generation
  )
  #expect(service.loadedValues(defaults: false)?.revision == baseRevision + 1)
  let mismatchBase = try #require(service.loadedValues(defaults: false)?.revision)
  guard case let .write(_, _, mismatchOperation) = try service.requestWrite(
    packageVersion: "1.2.3", fingerprint: schema.fingerprint,
    wireSignature: schema.wireSignature, baseRevision: mismatchBase, values: draft
  ) else { return }
  _ = try service.accept(
    .writeAcknowledgement,
    context: .refloatConfigurationWrite(route: .can(42), index: 0, operation: mismatchOperation),
    generation: generation
  )
  #expect(throws: RefloatConfigurationServiceError.conflict) {
    try service.accept(
      .values(command: .getCustomConfiguration, index: 0, bytes: current),
      context: .refloatConfigurationVerification(
        route: .can(42), index: 0, operation: mismatchOperation
      ),
      generation: generation
    )
  }
  #expect(service.loadedValues(defaults: false)?.revision == mismatchBase + 1)

  service.setFlywheel(true)
  #expect(!service.summary().currentLoaded)
  #expect(!service.summary().writable)
  #expect(throws: RefloatConfigurationServiceError.invalidState) {
    try service.requestValues(defaults: false)
  }
  guard case let .values(_, postFlywheelDefaults, postFlywheelOperation) = service.setFlywheel(false) else {
    Issue.record("Expected fresh current read after flywheel exit")
    return
  }
  #expect(!postFlywheelDefaults)
  #expect(!service.summary().currentLoaded)
  service.requestFailed(
    context: .refloatConfigurationValues(
      route: .can(42), index: 0, defaults: false,
      operation: postFlywheelOperation
    ),
    generation: generation
  )
  #expect(service.summary().pendingOperation == nil)

  guard case let .schemaChunk(_, _, newOffset, newOperation) = service.begin(
    route: .direct, generation: .init(rawValue: 10), firmware: firmware,
    reportedVersion: "1.2.3", writable: false, flywheel: false
  ) else { return }
  #expect(service.summary().availability == .loading)
  #expect(throws: RefloatConfigurationServiceError.staleResponse) {
    try service.accept(
      .schemaChunk(index: 0, totalLength: compressed.count, offset: newOffset, bytes: compressed),
      context: .refloatConfigurationSchema(
        route: .direct, index: 0, offset: newOffset, operation: newOperation
      ), generation: generation
    )
  }
}

@Test func configurationCatalogResolvesLegacyPatchOnlyByFullFingerprint() throws {
  var root = URL(fileURLWithPath: #filePath)
  for _ in 0 ..< 4 {
    root.deleteLastPathComponent()
  }
  root.append(path: "fixtures/refloat-configuration/schemas")
  let zero = try RefloatConfigurationSchemaCodec.decodeQtCompressed(
    Data(contentsOf: root.appending(path: "1.0.0.qcompress"))
  )
  let one = try RefloatConfigurationSchemaCodec.decodeQtCompressed(
    Data(contentsOf: root.appending(path: "1.0.1.qcompress"))
  )
  #expect(RefloatConfigurationCatalog.resolve(reportedVersion: "1.0", schema: zero)?.version == "1.0.0")
  #expect(RefloatConfigurationCatalog.resolve(reportedVersion: "1.0", schema: one)?.version == "1.0.1")
  #expect(zero.wireSignature == one.wireSignature)
}

@Test func configurationServiceUsesOnlyValidatedExactSchemaCacheEntries() throws {
  var root = URL(fileURLWithPath: #filePath)
  for _ in 0 ..< 4 {
    root.deleteLastPathComponent()
  }
  let compressed = try Data(contentsOf: root.appending(path: "fixtures/refloat-configuration/schemas/1.2.3.qcompress"))
  let identity = RefloatSchemaCacheIdentity(
    firmwareVersion: "6.6", hardwareName: "THOR300", packageVersion: "1.2.3",
    customIndex: 0, hardwareConfigurationCRC: 123
  )
  let firmware = FirmwareRecord(
    major: 6, minor: 6, hardwareName: "THOR300", firmwareName: nil,
    customConfigurationCount: 1, hardwareConfigurationCRC: 123
  )
  let cache = MemoryRefloatSchemaCache()
  cache.values[identity] = compressed
  let service = RefloatConfigurationService(cache: cache)
  #expect(service.begin(
    route: .direct, generation: .init(rawValue: 1), firmware: firmware,
    reportedVersion: "1.2.3", writable: true, flywheel: false
  ) == .values(route: .direct, defaults: false, operation: 1))
  #expect(service.summary().availability == .ready)

  cache.values[identity] = Data("corrupt".utf8)
  let replacement = RefloatConfigurationService(cache: cache)
  guard case .schemaChunk = replacement.begin(
    route: .direct, generation: .init(rawValue: 2), firmware: firmware,
    reportedVersion: "1.2.3", writable: true, flywheel: false
  ) else { Issue.record("Expected device fetch after corrupt cache"); return }
  #expect(cache.removals == 1)
}
