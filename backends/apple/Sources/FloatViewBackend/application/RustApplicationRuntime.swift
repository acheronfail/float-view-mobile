import CFloatViewApplicationRuntime
import Foundation

enum RustApplicationRuntimeError: Error, Equatable {
  case abiMismatch
  case invalidCommand
  case invalidHandle
  case runtimeFailure
  case busy
  case internalFailure
  case invalidState
  case conflict
  case unavailable
  case resultTooLarge
  case copyMismatch
  case malformedResult
}

struct RustApplicationRuntimeResult {
  let generation: UInt64
  let effects: [[String: Any]]
}

final class RustApplicationRuntime: @unchecked Sendable {
  private static let abiVersion: UInt32 = 1
  static let maximumCommandBytes = Int(FLOAT_VIEW_APPLICATION_RUNTIME_MAX_COMMAND_BYTES)
  static let maximumResultBytes = Int(FLOAT_VIEW_APPLICATION_RUNTIME_MAX_RESULT_BYTES)

  private var handle: UInt64 = 0

  init() throws {
    guard float_view_application_runtime_abi_version() == Self.abiVersion else {
      throw RustApplicationRuntimeError.abiMismatch
    }
    guard float_view_application_runtime_create(&handle) == FloatViewApplicationRuntimeStatus(
      FLOAT_VIEW_APPLICATION_RUNTIME_STATUS_OK
    ), handle != 0 else {
      throw RustApplicationRuntimeError.internalFailure
    }
  }

  deinit {
    _ = float_view_application_runtime_destroy(handle)
  }

  func apply(_ command: [String: Any]) throws -> RustApplicationRuntimeResult {
    guard JSONSerialization.isValidJSONObject(command) else {
      throw RustApplicationRuntimeError.invalidCommand
    }
    let input = try JSONSerialization.data(withJSONObject: command, options: [.sortedKeys])
    guard !input.isEmpty, input.count <= Self.maximumCommandBytes else {
      throw RustApplicationRuntimeError.invalidCommand
    }
    var required = 0
    let applyStatus = input.withUnsafeBytes { bytes in
      float_view_application_runtime_apply(
        handle,
        bytes.bindMemory(to: UInt8.self).baseAddress,
        bytes.count,
        &required
      )
    }
    guard applyStatus == FLOAT_VIEW_APPLICATION_RUNTIME_STATUS_OK else { throw Self.error(applyStatus) }
    guard required <= Self.maximumResultBytes else { throw RustApplicationRuntimeError.resultTooLarge }
    var output = Data(count: required)
    var written = 0
    let copyStatus = output.withUnsafeMutableBytes { bytes in
      float_view_application_runtime_copy_result(
        handle,
        bytes.bindMemory(to: UInt8.self).baseAddress,
        bytes.count,
        &written
      )
    }
    guard copyStatus == FLOAT_VIEW_APPLICATION_RUNTIME_STATUS_OK else { throw Self.error(copyStatus) }
    guard written == required else { throw RustApplicationRuntimeError.copyMismatch }
    guard let result = try JSONSerialization.jsonObject(with: output) as? [String: Any],
          (result["version"] as? NSNumber)?.uint32Value == Self.abiVersion,
          let generation = (result["generation"] as? NSNumber)?.uint64Value,
          let effects = result["effects"] as? [[String: Any]]
    else { throw RustApplicationRuntimeError.malformedResult }
    return RustApplicationRuntimeResult(generation: generation, effects: effects)
  }

  func interrupt() {
    _ = float_view_application_runtime_interrupt(handle)
  }

  private static func error(_ status: FloatViewApplicationRuntimeStatus) -> RustApplicationRuntimeError {
    switch status {
    case FloatViewApplicationRuntimeStatus(FLOAT_VIEW_APPLICATION_RUNTIME_STATUS_INVALID_ARGUMENT),
         FloatViewApplicationRuntimeStatus(FLOAT_VIEW_APPLICATION_RUNTIME_STATUS_INVALID_ENVELOPE),
         FloatViewApplicationRuntimeStatus(FLOAT_VIEW_APPLICATION_RUNTIME_STATUS_RESULT_PENDING),
         FloatViewApplicationRuntimeStatus(FLOAT_VIEW_APPLICATION_RUNTIME_STATUS_BUFFER_TOO_SMALL):
      .invalidCommand
    case FloatViewApplicationRuntimeStatus(FLOAT_VIEW_APPLICATION_RUNTIME_STATUS_INVALID_HANDLE):
      .invalidHandle
    case FloatViewApplicationRuntimeStatus(FLOAT_VIEW_APPLICATION_RUNTIME_STATUS_RUNTIME_FAILURE):
      .runtimeFailure
    case FloatViewApplicationRuntimeStatus(FLOAT_VIEW_APPLICATION_RUNTIME_STATUS_BUSY):
      .busy
    case FloatViewApplicationRuntimeStatus(FLOAT_VIEW_APPLICATION_RUNTIME_STATUS_INVALID_STATE):
      .invalidState
    case FloatViewApplicationRuntimeStatus(FLOAT_VIEW_APPLICATION_RUNTIME_STATUS_CONFLICT):
      .conflict
    case FloatViewApplicationRuntimeStatus(FLOAT_VIEW_APPLICATION_RUNTIME_STATUS_UNAVAILABLE):
      .unavailable
    default:
      .internalFailure
    }
  }
}
