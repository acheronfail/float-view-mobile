import Foundation

public struct RefloatConfigurationSummary: Equatable, Sendable {
  public let availability: RefloatConfigurationAvailability
  public let writable: Bool
  public let packageVersion: String?
  public let schemaFingerprint: String?
  public let wireSignature: UInt32?
  public let currentLoaded: Bool
  public let defaultLoaded: Bool
  public let pendingOperation: RefloatConfigurationOperation?
}

public enum RefloatConfigurationServiceError: String, Error, Sendable {
  case unavailable
  case invalidState
  case staleResponse
  case unauditedSchema
  case malformedResponse
  case conflict
}

public struct RefloatSchemaCacheIdentity: Hashable, Codable, Sendable {
  public let firmwareVersion: String
  public let hardwareName: String
  public let packageVersion: String
  public let customIndex: UInt8
  public let hardwareConfigurationCRC: UInt32
}

public protocol RefloatSchemaCachePort: AnyObject, Sendable {
  func cachedRefloatSchema(identity: RefloatSchemaCacheIdentity) -> Data?
  func storeRefloatSchema(_ data: Data, identity: RefloatSchemaCacheIdentity)
  func removeCachedRefloatSchema(identity: RefloatSchemaCacheIdentity)
}

public extension RefloatSchemaCachePort {
  func cachedRefloatSchema(identity _: RefloatSchemaCacheIdentity) -> Data? {
    nil
  }

  func storeRefloatSchema(_: Data, identity _: RefloatSchemaCacheIdentity) {}
  func removeCachedRefloatSchema(identity _: RefloatSchemaCacheIdentity) {}
}

public enum RefloatConfigurationEffect: Equatable, Sendable {
  case none
  case schemaChunk(route: ControllerRoute, length: Int, offset: Int, operation: UInt64)
  case values(route: ControllerRoute, defaults: Bool, operation: UInt64)
  case write(route: ControllerRoute, payload: Data, operation: UInt64)
  case verification(route: ControllerRoute, operation: UInt64)
}

public struct RefloatConfigurationCatalogEntry: Equatable, Sendable {
  public let version: String
  public let fingerprint: String
  public let wireSignature: UInt32
}

public enum RefloatConfigurationCatalog {
  public static let entries: [RefloatConfigurationCatalogEntry] = [
    .init(
      version: "1.0.0",
      fingerprint: "324e7f5ef1ea1b2038478af7e6f43c91d63ec06a6caf2907c5b7e6c812382ad9",
      wireSignature: 4_090_797_430
    ),
    .init(
      version: "1.0.1",
      fingerprint: "887e7b03c03304a2f8455e71b369b89830e8d6446a73aa598ab067ac81944526",
      wireSignature: 4_090_797_430
    ),
    .init(
      version: "1.1.0",
      fingerprint: "b5849e38371a65721bb48182048023d1c707012228554878ce263b1c49ee168f",
      wireSignature: 15_843_867
    ),
    .init(
      version: "1.1.1",
      fingerprint: "b5849e38371a65721bb48182048023d1c707012228554878ce263b1c49ee168f",
      wireSignature: 15_843_867
    ),
    .init(
      version: "1.1.2",
      fingerprint: "b5849e38371a65721bb48182048023d1c707012228554878ce263b1c49ee168f",
      wireSignature: 15_843_867
    ),
    .init(
      version: "1.2.0",
      fingerprint: "2561e00a5b6694de0ac272bae077e06050cfc00cf4ba371bb1d5d9616e137f7e",
      wireSignature: 2_427_955_642
    ),
    .init(
      version: "1.2.1",
      fingerprint: "9b8979a36759810a7fa9c19d01534ed52ec9ed8ac4a2d3ab3aefc594ba3b93cc",
      wireSignature: 2_427_955_642
    ),
    .init(
      version: "1.2.2",
      fingerprint: "d7ca6a2c06ce5701858eebfaaf224e97e08a212d1dab60b664d268a90763eb39",
      wireSignature: 2_427_955_642
    ),
    .init(
      version: "1.2.3",
      fingerprint: "fa40717969d1d86b429d7c78b59144b8387b4850be83a72fd3c7cf15ffecd01c",
      wireSignature: 2_427_955_642
    ),
  ]

  public static func resolve(reportedVersion: String,
                             schema: RefloatConfigurationSchema) -> RefloatConfigurationCatalogEntry?
  {
    entries.first {
      ($0.version == reportedVersion || reportedVersion == "1.0" && $0.version.hasPrefix("1.0."))
        && $0.fingerprint == schema.fingerprint && $0.wireSignature == schema.wireSignature
    }
  }
}

/// Connection-scoped, bounded lifecycle owner. Scheduler submission remains in ApplicationService.
public final class RefloatConfigurationService: @unchecked Sendable {
  public static let index: UInt8 = 0
  public static let requestTimeoutMs: Int64 = 3_000
  private let lock = NSLock()
  private weak var cache: (any RefloatSchemaCachePort)?
  private var route: ControllerRoute?
  private var generation = VescRequestGeneration(rawValue: 0)
  private var reportedVersion: String?
  private var canWrite = false
  private var flywheel = false
  private var nextOperation: UInt64 = 0
  private var operation: UInt64?
  private var assembly = Data()
  private var totalLength: Int?
  private var requestedOffset = 0
  private var schema: RefloatConfigurationSchema?
  private var identity: RefloatConfigurationSchemaIdentity?
  private var current: RefloatConfigurationValues?
  private var defaults: RefloatConfigurationValues?
  private var expectedReadback: [String: RefloatConfigurationValue]?
  private var revision: UInt64 = 0
  private var availability: RefloatConfigurationAvailability = .unavailable
  private var pending: RefloatConfigurationOperation?
  private var cacheIdentity: RefloatSchemaCacheIdentity?

  public init(cache: (any RefloatSchemaCachePort)? = nil) {
    self.cache = cache
  }

  public func begin(
    route: ControllerRoute, generation: VescRequestGeneration, firmware: FirmwareRecord,
    reportedVersion: String, writable: Bool, flywheel: Bool
  ) -> RefloatConfigurationEffect {
    lock.withLock {
      resetLocked()
      self.route = route
      self.generation = generation
      self.reportedVersion = reportedVersion
      canWrite = writable
      self.flywheel = flywheel
      guard firmware.customConfigurationCount > Self.index else {
        availability = .unavailable
        return .none
      }
      availability = .loading
      if let crc = firmware.hardwareConfigurationCRC, crc != 0 {
        let cacheIdentity = RefloatSchemaCacheIdentity(
          firmwareVersion: "\(firmware.major).\(firmware.minor)",
          hardwareName: firmware.hardwareName ?? "", packageVersion: reportedVersion,
          customIndex: Self.index, hardwareConfigurationCRC: crc
        )
        self.cacheIdentity = cacheIdentity
        if let cached = cache?.cachedRefloatSchema(identity: cacheIdentity) {
          do {
            let decoded = try RefloatConfigurationSchemaCodec.decodeQtCompressed(cached)
            if let entry = RefloatConfigurationCatalog.resolve(reportedVersion: reportedVersion, schema: decoded) {
              schema = decoded
              identity = .init(
                packageName: "refloat", packageVersion: entry.version,
                fingerprint: entry.fingerprint, wireSignature: entry.wireSignature
              )
              availability = .ready
              guard !flywheel else { return .none }
              pending = .readCurrent
              return .values(route: route, defaults: false, operation: startOperationLocked())
            }
          } catch {}
          cache?.removeCachedRefloatSchema(identity: cacheIdentity)
        }
      }
      pending = .schema
      let operation = startOperationLocked()
      return .schemaChunk(
        route: route,
        length: VescCustomConfigurationCodec.maximumChunkBytes,
        offset: 0,
        operation: operation
      )
    }
  }

  public func reset() {
    lock.withLock { resetLocked() }
  }

  public func summary() -> RefloatConfigurationSummary {
    lock.withLock {
      .init(
        availability: availability,
        writable: availability == .ready && canWrite && !flywheel && current != nil,
        packageVersion: identity?.packageVersion,
        schemaFingerprint: identity?.fingerprint,
        wireSignature: identity?.wireSignature,
        currentLoaded: current != nil && !flywheel,
        defaultLoaded: defaults != nil,
        pendingOperation: pending
      )
    }
  }

  public func loadedSchema() -> RefloatConfigurationSchema? {
    lock.withLock { schema }
  }

  public func loadedValues(defaults: Bool) -> RefloatConfigurationValues? {
    lock.withLock { defaults ? self.defaults : current }
  }

  @discardableResult public func setFlywheel(_ active: Bool) -> RefloatConfigurationEffect {
    lock.withLock {
      let wasActive = flywheel
      flywheel = active
      if active {
        current = nil
        if pending == .readCurrent {
          pending = nil
          operation = nil
        }
        return .none
      }
      guard wasActive, schema != nil, pending == nil, let route else { return .none }
      pending = .readCurrent
      return .values(route: route, defaults: false, operation: startOperationLocked())
    }
  }

  public func invalidateCurrent() {
    lock.withLock { current = nil }
  }

  public func requestValues(defaults: Bool) throws -> RefloatConfigurationEffect {
    try lock.withLock {
      guard let route, schema != nil, pending == nil else { throw RefloatConfigurationServiceError.invalidState }
      guard defaults || !flywheel else { throw RefloatConfigurationServiceError.invalidState }
      pending = defaults ? .readDefault : .readCurrent
      return .values(route: route, defaults: defaults, operation: startOperationLocked())
    }
  }

  public func requestWrite(
    packageVersion: String, fingerprint: String, wireSignature: UInt32,
    baseRevision: UInt64, values: [String: RefloatConfigurationValue]
  ) throws -> RefloatConfigurationEffect {
    try lock.withLock {
      guard let route, let schema, let identity, let current,
            availability == .ready, canWrite, !flywheel, pending == nil,
            identity.packageVersion == packageVersion,
            identity.fingerprint == fingerprint,
            identity.wireSignature == wireSignature,
            current.revision == baseRevision
      else { throw RefloatConfigurationServiceError.invalidState }
      let payload = try RefloatConfigurationValueCodec.encode(values, schema: schema)
      expectedReadback = normalized(values, version: identity.packageVersion)
      pending = .write
      return .write(route: route, payload: payload, operation: startOperationLocked())
    }
  }

  public func accept(
    _ response: VescCustomConfigurationResponse, context: VescRequestContext,
    generation responseGeneration: VescRequestGeneration
  ) throws -> RefloatConfigurationEffect {
    try lock.withLock {
      guard responseGeneration == generation, let route, let operation else {
        throw RefloatConfigurationServiceError.staleResponse
      }
      switch (response, context) {
      case let (.schemaChunk(index, total, offset, bytes),
                .refloatConfigurationSchema(responseRoute, expectedIndex, expectedOffset, expectedOperation)):
        guard pending == .schema, responseRoute == .init(route), index == expectedIndex,
              index == Self.index, offset == expectedOffset, offset == requestedOffset,
              expectedOperation == operation, !bytes.isEmpty,
              total <= RefloatConfigurationLimits.compressedBytes,
              totalLength == nil || totalLength == total
        else { throw RefloatConfigurationServiceError.staleResponse }
        totalLength = total
        assembly.append(bytes)
        guard assembly.count <= total else { throw RefloatConfigurationServiceError.malformedResponse }
        requestedOffset = assembly.count
        if assembly.count < total {
          return .schemaChunk(
            route: route, length: min(VescCustomConfigurationCodec.maximumChunkBytes, total - assembly.count),
            offset: assembly.count, operation: operation
          )
        }
        let decoded = try RefloatConfigurationSchemaCodec.decodeQtCompressed(assembly)
        guard let reportedVersion,
              let entry = RefloatConfigurationCatalog.resolve(reportedVersion: reportedVersion, schema: decoded)
        else {
          failLocked()
          throw RefloatConfigurationServiceError.unauditedSchema
        }
        schema = decoded
        identity = .init(
          packageName: "refloat", packageVersion: entry.version,
          fingerprint: entry.fingerprint, wireSignature: entry.wireSignature
        )
        pending = nil
        self.operation = nil
        availability = .ready
        if let cacheIdentity { cache?.storeRefloatSchema(assembly, identity: cacheIdentity) }
        guard !flywheel else { return .none }
        pending = .readCurrent
        return .values(route: route, defaults: false, operation: startOperationLocked())
      case let (.values(command, index, bytes),
                .refloatConfigurationValues(responseRoute, expectedIndex, expectedDefaults, expectedOperation)):
        guard responseRoute == .init(route), index == expectedIndex, index == Self.index,
              expectedOperation == operation,
              command == (expectedDefaults ? .getCustomConfigurationDefault : .getCustomConfiguration),
              pending == (expectedDefaults ? .readDefault : .readCurrent), let schema
        else { throw RefloatConfigurationServiceError.staleResponse }
        guard expectedDefaults || !flywheel else { throw RefloatConfigurationServiceError.invalidState }
        let decoded = try RefloatConfigurationValueCodec.decode(bytes, schema: schema)
        revision &+= 1
        let values = RefloatConfigurationValues(
          revision: revision, source: expectedDefaults ? "default" : "current", values: decoded
        )
        if expectedDefaults { defaults = values } else { current = values }
        pending = nil
        self.operation = nil
        return .none
      case let (.writeAcknowledgement,
                .refloatConfigurationWrite(responseRoute, expectedIndex, expectedOperation)):
        guard responseRoute == .init(route), expectedIndex == Self.index,
              expectedOperation == operation, pending == .write, expectedReadback != nil
        else { throw RefloatConfigurationServiceError.staleResponse }
        return .verification(route: route, operation: operation)
      case let (.values(command, index, bytes),
                .refloatConfigurationVerification(responseRoute, expectedIndex, expectedOperation)):
        guard responseRoute == .init(route), index == expectedIndex, index == Self.index,
              expectedOperation == operation, pending == .write,
              command == .getCustomConfiguration, !flywheel,
              let schema, let expectedReadback
        else { throw RefloatConfigurationServiceError.staleResponse }
        let decoded = try RefloatConfigurationValueCodec.decode(bytes, schema: schema)
        revision &+= 1
        current = .init(revision: revision, source: "current", values: decoded)
        pending = nil
        self.operation = nil
        self.expectedReadback = nil
        guard decoded == expectedReadback else { throw RefloatConfigurationServiceError.conflict }
        return .none
      default:
        throw RefloatConfigurationServiceError.staleResponse
      }
    }
  }

  public func requestFailed(context: VescRequestContext, generation: VescRequestGeneration) {
    lock.withLock {
      guard generation == self.generation else { return }
      let matches: Bool = switch context {
      case let .refloatConfigurationSchema(_, _, _, operation),
           let .refloatConfigurationValues(_, _, _, operation),
           let .refloatConfigurationWrite(_, _, operation),
           let .refloatConfigurationVerification(_, _, operation): operation == self.operation
      default: false
      }
      guard matches else { return }
      failLocked()
    }
  }

  public func abortActiveOperation(generation: VescRequestGeneration) {
    lock.withLock {
      guard generation == self.generation, pending != nil else { return }
      failLocked()
    }
  }

  private func startOperationLocked() -> UInt64 {
    nextOperation &+= 1
    operation = nextOperation
    return nextOperation
  }

  private func failLocked() {
    pending = nil
    cacheIdentity = nil
    operation = nil
    current = nil
    expectedReadback = nil
    availability = schema == nil ? .unavailable : .ready
  }

  private func resetLocked() {
    route = nil
    reportedVersion = nil
    canWrite = false
    flywheel = false
    operation = nil
    assembly.removeAll(keepingCapacity: false)
    totalLength = nil
    requestedOffset = 0
    schema = nil
    identity = nil
    current = nil
    defaults = nil
    expectedReadback = nil
    revision = 0
    availability = .unavailable
    pending = nil
    cacheIdentity = nil
  }

  private func normalized(
    _ values: [String: RefloatConfigurationValue], version: String
  ) -> [String: RefloatConfigurationValue] {
    guard !version.hasPrefix("1.0.") else { return values }
    var result = values
    if result["meta.is_default"] != nil { result["meta.is_default"] = .boolean(false) }
    return result
  }
}
