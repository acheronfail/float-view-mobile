import CFloatViewApplicationRuntime
import Foundation

/// Caller-owned buffers for the existing application-runtime binding family.
final class ServerExchange: @unchecked Sendable {
  let handle: UInt64
  init(platform: String, html: Data) throws {
    var handle: UInt64 = 0
    let status = html.withUnsafeBytes { bytes in
      float_view_application_runtime_server_create(
        platform == "ios" ? 1 : 0,
        bytes.bindMemory(to: UInt8.self).baseAddress,
        bytes.count,
        &handle
      )
    }
    guard status == 0, handle != 0 else { throw ServerFactError.invalid }
    self.handle = handle
  }

  deinit { float_view_application_runtime_server_destroy(handle) }
  func command(_ name: String, _ parameters: [String: Any]? = nil) throws -> [String: Any] {
    var action: [String: Any] = ["command": name]
    if let parameters {
      action["parameters"] = parameters
    }
    let input = try JSONSerialization.data(withJSONObject: ["version": 1, "action": action])
    guard input.count <= 1_048_576 else { throw ServerFactError.invalid }
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
          let result = envelope["data"] as? [String: Any] else { throw ServerFactError.invalid }
    return result
  }

  func read(generation: UInt64, transfer: UInt64) throws -> Data {
    var output = [UInt8](repeating: 0, count: 65_536)
    var count = 0
    let status = output.withUnsafeMutableBufferPointer { buffer in
      float_view_application_runtime_server_read_transfer(
        handle,
        generation,
        transfer,
        buffer.baseAddress,
        buffer.count,
        &count
      )
    }
    guard status == 0 else { throw ServerFactError.invalid }
    return Data(output.prefix(count))
  }

  func write(generation: UInt64, transfer: UInt64, offset: UInt64, data: Data) throws {
    let status = data.withUnsafeBytes { bytes in
      float_view_application_runtime_server_write_transfer(
        handle,
        generation,
        transfer,
        offset,
        bytes.bindMemory(to: UInt8.self).baseAddress,
        bytes.count
      )
    }
    guard status == 0 else { throw ServerFactError.invalid }
  }

  func complete(id: [String: Any], outcome: [String: Any]) throws {
    let bytes = try JSONSerialization.data(withJSONObject: outcome)
    guard bytes.count <= 8 * 1024 * 1024 else { throw ServerFactError.invalid }
    if bytes.count < 900_000 {
      _ = try command("complete", ["id": id, "outcome": outcome]); return
    }
    var offset = 0
    while offset < bytes.count {
      var end = min(offset + 65_536, bytes.count)
      while end < bytes.count, bytes[end] & 0xC0 == 0x80 {
        end -= 1
      }
      guard let text = String(data: bytes[offset ..< end], encoding: .utf8) else { throw ServerFactError.invalid }
      _ = try command("complete_fragment", ["id": id, "offset": offset, "text": text, "last": end == bytes.count])
      offset = end
    }
  }
}
