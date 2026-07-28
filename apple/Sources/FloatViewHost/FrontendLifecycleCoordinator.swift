import Foundation

public protocol FrontendServerPort: AnyObject {
  var origin: String { get }
  var token: String { get }
  func start() throws
  func stop()
}

extension LoopbackServer: FrontendServerPort {}

public struct FrontendEndpoint: Sendable, Equatable {
  public let generation: UInt64
  public let origin: URL
  public let token: String

  public init(generation: UInt64, origin: URL, token: String) {
    self.generation = generation
    self.origin = origin
    self.token = token
  }
}

/// Owns only the disposable local presentation transport. Bluetooth, protocol,
/// and logging owners must outlive this coordinator's individual server
/// instances.
public final class FrontendLifecycleCoordinator: @unchecked Sendable {
  private let makeServer: @Sendable () -> any FrontendServerPort
  private let lock = NSLock()
  private var server: (any FrontendServerPort)?
  private var generation: UInt64 = 0

  public init(makeServer: @escaping @Sendable () -> any FrontendServerPort) {
    self.makeServer = makeServer
  }

  public func activate() throws -> FrontendEndpoint {
    lock.lock()
    defer { lock.unlock() }
    server?.stop()
    server = nil

    let next = makeServer()
    do {
      try next.start()
      guard let origin = URL(string: next.origin) else {
        next.stop()
        throw FrontendLifecycleError.invalidOrigin
      }
      generation &+= 1
      server = next
      return FrontendEndpoint(generation: generation, origin: origin, token: next.token)
    } catch {
      next.stop()
      throw error
    }
  }

  public func deactivate() {
    lock.lock()
    defer { lock.unlock() }
    server?.stop()
    server = nil
  }

  public var isActive: Bool {
    lock.withLock { server != nil }
  }

  deinit {
    server?.stop()
  }
}

public enum FrontendLifecycleError: Error {
  case invalidOrigin
}
