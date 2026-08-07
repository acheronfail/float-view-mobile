import CFloatViewCore
import Foundation

public enum FloatViewCoreError: Error, Equatable {
  case abiMismatch(expected: UInt32, actual: UInt32)
  case buildMismatch
  case native(FloatViewCoreStatus)
  case unknownNative(Int32)
  case invalidCommand
  case payloadTooLarge
}

public enum FloatViewCoreStatus: Int32, Sendable {
  case ok = 0
  case invalidArgument = 1
  case invalidHandle = 2
  case wrongState = 3
  case bufferTooSmall = 4
  case timeout = 5
  case stopped = 6
  case queueFull = 7
  case staleGeneration = 8
  case duplicateOperation = 9
  case panic = 10
  case `internal` = 11
}

public struct FloatViewHostCommand: Sendable, Equatable {
  public let kind: UInt32
  public let flags: UInt32
  public let runtimeGeneration: UInt64
  public let operationID: UInt64
  public let payload: Data
  public let presentationGeneration: UInt64

  public init(
    kind: UInt32,
    flags: UInt32 = 0,
    runtimeGeneration: UInt64,
    operationID: UInt64,
    payload: Data,
    presentationGeneration: UInt64 = 0
  ) {
    self.kind = kind
    self.flags = flags
    self.runtimeGeneration = runtimeGeneration
    self.operationID = operationID
    self.payload = Data(payload)
    self.presentationGeneration = presentationGeneration
  }
}

public protocol FloatViewPlatformRuntime: AnyObject, Sendable {
  var generation: UInt64 { get }
  func submitPlatformEvent(_ payload: Data) throws
  func submitBluetoothBytes(
    connectionGeneration: UInt64,
    adapterOrder: UInt64,
    bytes: Data
  ) throws
  func complete(_ command: FloatViewHostCommand, resultKind: UInt32, payload: Data) throws
}

public final class FloatViewCoreRuntime: @unchecked Sendable {
  public typealias CommandHandler = @Sendable (FloatViewHostCommand) -> Void
  public static let abiVersion = UInt32(FLOAT_VIEW_ABI_VERSION)
  public static let maximumPayloadBytes = Int(FLOAT_VIEW_MAX_COMMAND_PAYLOAD)
  public static let maximumCompletionPayloadBytes = 8 * 1024 * 1024

  private let stateLock = NSLock()
  private let commandQueue = DispatchQueue(label: "fail.acheron.float-view.core-command-pump")
  private let pumpStopped = DispatchSemaphore(value: 0)
  private var handle: OpaquePointer?
  private var runtimeGeneration: UInt64 = 0
  private var presentationGeneration: UInt64 = 0
  private var pumpStarted = false
  private var pumpFailure: FloatViewCoreError?

  public init() throws {
    let actualABI = float_view_abi_version()
    let expectedABI = UInt32(FLOAT_VIEW_ABI_VERSION)
    guard actualABI == expectedABI else {
      throw FloatViewCoreError.abiMismatch(expected: expectedABI, actual: actualABI)
    }

    var required = 0
    var identity = [UInt8](repeating: 0, count: Int(FLOAT_VIEW_BUILD_ID_CAPACITY))
    let identityStatus = identity.withUnsafeMutableBytes { bytes in
      float_view_build_id(bytes.baseAddress, bytes.count, &required)
    }
    guard identityStatus == FLOAT_VIEW_STATUS_OK,
          identity == Array("float-view-0.1.0".utf8)
    else {
      throw FloatViewCoreError.buildMismatch
    }

    var created: OpaquePointer?
    let status = float_view_runtime_create(&created)
    guard status == FLOAT_VIEW_STATUS_OK, let created else {
      throw Self.error(status)
    }
    handle = created
  }

  deinit {
    if handle != nil {
      try? stop()
    }
  }

  public func start(requestedPort: UInt16 = 0, handler: @escaping CommandHandler) throws {
    stateLock.lock()
    defer { stateLock.unlock() }
    guard let handle, !pumpStarted else {
      throw FloatViewCoreError.native(.wrongState)
    }
    var generation: UInt64 = 0
    let status = float_view_runtime_start_on_port(handle, requestedPort, &generation)
    guard status == FLOAT_VIEW_STATUS_OK else {
      throw Self.error(status)
    }
    runtimeGeneration = generation
    pumpStarted = true
    pumpFailure = nil
    commandQueue.async { [self] in
      pump(handler: handler)
    }
  }

  public func replacePresentation(generation: UInt64) {
    stateLock.withLock {
      presentationGeneration = generation
    }
  }

  public func submitTestCommand(kind: UInt32, payload: Data) throws {
    guard payload.count <= Self.maximumPayloadBytes else {
      throw FloatViewCoreError.payloadTooLarge
    }
    let (handle, generation) = try activeRuntime()
    let status = payload.withUnsafeBytes { bytes in
      float_view_runtime_submit_event(handle, generation, kind, bytes.baseAddress, bytes.count)
    }
    guard status == FLOAT_VIEW_STATUS_OK else {
      throw Self.error(status)
    }
  }

  public func complete(
    _ command: FloatViewHostCommand,
    resultKind: UInt32 = 0,
    payload: Data = Data()
  ) throws {
    guard payload.count <= Self.maximumCompletionPayloadBytes else {
      throw FloatViewCoreError.payloadTooLarge
    }
    let (handle, generation) = try activeRuntime()
    guard command.runtimeGeneration == generation else {
      throw FloatViewCoreError.native(.staleGeneration)
    }
    let status = payload.withUnsafeBytes { bytes in
      float_view_runtime_complete_operation(
        handle, generation, command.operationID, resultKind, bytes.baseAddress, bytes.count
      )
    }
    guard status == FLOAT_VIEW_STATUS_OK else {
      throw Self.error(status)
    }
  }

  public func cancel(_ command: FloatViewHostCommand) throws {
    try complete(command, resultKind: 1)
  }

  public var generation: UInt64 {
    stateLock.withLock { runtimeGeneration }
  }

  public func submitPlatformEvent(_ payload: Data) throws {
    guard payload.count <= Self.maximumPayloadBytes else {
      throw FloatViewCoreError.payloadTooLarge
    }
    let (handle, generation) = try activeRuntime()
    let status = payload.withUnsafeBytes { bytes in
      float_view_runtime_submit_event(
        handle, generation, floatViewPlatformEventKind, bytes.baseAddress, bytes.count
      )
    }
    guard status == FLOAT_VIEW_STATUS_OK else {
      throw Self.error(status)
    }
  }

  public func submitBluetoothBytes(
    connectionGeneration: UInt64,
    adapterOrder: UInt64,
    bytes: Data
  ) throws {
    guard !bytes.isEmpty, bytes.count <= 4096 else {
      throw FloatViewCoreError.payloadTooLarge
    }
    let (handle, generation) = try activeRuntime()
    let status = bytes.withUnsafeBytes { buffer in
      float_view_runtime_submit_ble_bytes(
        handle,
        generation,
        connectionGeneration,
        adapterOrder,
        buffer.baseAddress,
        buffer.count
      )
    }
    guard status == FLOAT_VIEW_STATUS_OK else {
      throw Self.error(status)
    }
  }

  public var commandPumpFailure: FloatViewCoreError? {
    stateLock.withLock { pumpFailure }
  }

  public func stop() throws {
    let currentHandle: OpaquePointer? = stateLock.withLock { handle }
    guard let currentHandle else { return }
    let stopStatus = float_view_runtime_stop(currentHandle)
    guard stopStatus == FLOAT_VIEW_STATUS_OK else {
      throw Self.error(stopStatus)
    }
    if stateLock.withLock({ pumpStarted }) {
      pumpStopped.wait()
    }
    let destroyStatus = float_view_runtime_destroy(currentHandle)
    guard destroyStatus == FLOAT_VIEW_STATUS_OK else {
      throw Self.error(destroyStatus)
    }
    stateLock.withLock {
      handle = nil
      pumpStarted = false
    }
  }

  private func activeRuntime() throws -> (OpaquePointer, UInt64) {
    try stateLock.withLock {
      guard let handle, pumpStarted else {
        throw FloatViewCoreError.native(.wrongState)
      }
      return (handle, runtimeGeneration)
    }
  }

  private func pump(handler: @escaping CommandHandler) {
    precondition(!Thread.isMainThread)
    defer { pumpStopped.signal() }
    var payload = [UInt8](repeating: 0, count: 1024)
    while true {
      guard let (handle, generation) = try? activeRuntime() else { return }
      var header = FloatViewCommandHeader()
      var required = 0
      let status = payload.withUnsafeMutableBytes { bytes in
        float_view_runtime_wait_command(
          handle, generation, 100, &header, bytes.baseAddress, bytes.count, &required
        )
      }
      if status == FLOAT_VIEW_STATUS_BUFFER_TOO_SMALL {
        guard required <= Self.maximumPayloadBytes else {
          failPump(.payloadTooLarge)
          return
        }
        payload = [UInt8](repeating: 0, count: required)
        continue
      }
      if status == FLOAT_VIEW_STATUS_TIMEOUT { continue }
      if status == FLOAT_VIEW_STATUS_STOPPED { return }
      guard status == FLOAT_VIEW_STATUS_OK else {
        failPump(Self.error(status))
        return
      }
      guard header.abi_version == UInt32(FLOAT_VIEW_ABI_VERSION),
            header.runtime_generation == generation,
            header.reserved == 0,
            header.payload_len == UInt64(required),
            required <= payload.count,
            required <= Self.maximumPayloadBytes
      else {
        failPump(.invalidCommand)
        return
      }
      let presentation = stateLock.withLock { presentationGeneration }
      let command = FloatViewHostCommand(
        kind: header.kind,
        flags: header.flags,
        runtimeGeneration: header.runtime_generation,
        operationID: header.operation_id,
        payload: Data(payload.prefix(required)),
        presentationGeneration: presentation
      )
      handler(command)
    }
  }

  private func failPump(_ error: FloatViewCoreError) {
    stateLock.withLock {
      pumpFailure = error
    }
  }

  private static func error(_ status: Int32) -> FloatViewCoreError {
    FloatViewCoreStatus(rawValue: status).map(FloatViewCoreError.native)
      ?? .unknownNative(status)
  }
}

extension FloatViewCoreRuntime: FloatViewPlatformRuntime {}

private extension NSLock {
  func withLock<T>(_ body: () throws -> T) rethrows -> T {
    lock()
    defer { unlock() }
    return try body()
  }
}
