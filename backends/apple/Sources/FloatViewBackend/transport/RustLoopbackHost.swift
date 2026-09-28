import Foundation

/// Listener lifetime borrows the existing process service.
final class RustLoopbackHost: @unchecked Sendable {
  private let service: SharedService
  private let exchange: ServerExchange
  private let platform: String
  private let lock = NSLock()
  private var running = false
  private(set) var port: UInt16 = 0
  private(set) var token = ""
  init(html: Data, service: SharedService, platform: String) throws {
    self.service = service
    self.platform = platform
    exchange = try ServerExchange(service: service, platform: platform, html: html)
  }

  func start() throws {
    try lock.withLock {
      guard !running else { return }
      let result = try exchange.command("start", ["platform": platform, "port": platform == "ios" ? 11011 : 11010])
      guard let port = result["port"] as? NSNumber, let token = result["token"] as? String else {
        throw CocoaError(.validationMissingMandatoryProperty)
      }
      self.port = port.uint16Value
      self.token = token
      running = true
    }
  }

  func stop() {
    lock.withLock {
      guard running else { return }
      _ = try? exchange.command("stop")
      running = false
    }
  }

  deinit { stop() }
}
