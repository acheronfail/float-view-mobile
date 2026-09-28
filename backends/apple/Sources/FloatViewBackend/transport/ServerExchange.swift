import CFloatViewApplicationRuntime
import Foundation

/// Caller-owned buffers for the existing application-runtime binding family.
final class ServerExchange: @unchecked Sendable {
  let handle: UInt64
  init(service: SharedService, platform: String, html: Data) throws {
    var handle: UInt64 = 0
    let status = html.withUnsafeBytes { bytes in
      float_view_application_runtime_service_server_create(
        service.handle,
        platform == "ios" ? 1 : 0,
        bytes.bindMemory(to: UInt8.self).baseAddress,
        bytes.count,
        &handle
      )
    }
    guard status == 0, handle != 0 else { throw CocoaError(.validationMissingMandatoryProperty) }
    self.handle = handle
  }

  deinit { float_view_application_runtime_server_destroy(handle) }
  func command(_ name: String, _ parameters: [String: Any]? = nil) throws -> [String: Any] {
    var action: [String: Any] = ["command": name]
    if let parameters {
      action["parameters"] = parameters
    }
    let input = try JSONSerialization.data(withJSONObject: ["version": 1, "action": action])
    guard input.count <= 1_048_576 else { throw CocoaError(.validationMissingMandatoryProperty) }
    var output = [UInt8](repeating: 0, count: 1_048_576)
    var count = 0
    let status = input.withUnsafeBytes { bytes in
      output.withUnsafeMutableBufferPointer { buffer in
        float_view_application_runtime_server_command(
          handle,
          bytes.bindMemory(to: UInt8.self).baseAddress,
          bytes.count,
          buffer.baseAddress,
          buffer.count,
          &count
        )
      }
    }
    guard status == 0,
          let envelope = try JSONSerialization.jsonObject(with: Data(output.prefix(count))) as? [String: Any],
          let result = envelope["data"] as? [String: Any] else { throw CocoaError(.validationMissingMandatoryProperty) }
    return result
  }
}
