import Foundation

public enum PlatformLoopbackPort {
  public static let macOS: UInt16 = 11_010
  public static let iOS: UInt16 = 11_011
}

public enum LoopbackProtocolLimits {
  public static let maximumWebSocketMessageBytes = 65_536
  public static let maximumSessionImportBytes = 128 * 1024 * 1024
}

public func loopbackStartupFailure(port: UInt16, error: any Error) -> String {
  "failed to bind Float View backend to 127.0.0.1:\(port): \(error)"
}

/// Process-owned native execution and assets; Rust owns the complete public transport.
public final class LoopbackServer: @unchecked Sendable {
  private let host: Result<RustLoopbackHost, any Error>
  public var port: UInt16 {
    (try? host.get().port) ?? 0
  }

  public var token: String {
    (try? host.get().token) ?? ""
  }

  public var origin: String {
    "http://127.0.0.1:\(port)"
  }

  public init(
    html: Data,
    port: UInt16 = 0,
    service: SharedService,
    platform: String = "macos"
  ) {
    host = Result {
      let expected = platform == "ios" ? PlatformLoopbackPort.iOS : PlatformLoopbackPort.macOS
      guard platform == "ios" || platform == "macos", port == 0 || port == expected else {
        throw CocoaError(.validationMissingMandatoryProperty)
      }
      return try RustLoopbackHost(html: html, service: service, platform: platform)
    }
  }

  public func start() throws {
    try host.get().start()
  }

  public func stop() {
    try? host.get().stop()
  }

  deinit { stop() }
}
