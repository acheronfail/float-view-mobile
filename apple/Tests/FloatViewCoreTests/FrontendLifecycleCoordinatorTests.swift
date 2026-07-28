import Foundation
import Testing
@testable import FloatViewHost

@Test
func foregroundActivationAlwaysCreatesAFreshServerTokenAndGeneration() throws {
  let factory = FakeFrontendServerFactory()
  let lifecycle = FrontendLifecycleCoordinator { factory.make() }

  let first = try lifecycle.activate()
  #expect(lifecycle.isActive)
  let second = try lifecycle.activate()

  #expect(first.generation == 1)
  #expect(second.generation == 2)
  #expect(first.token != second.token)
  #expect(factory.servers.count == 2)
  #expect(factory.servers[0].stopCount == 1)
  #expect(factory.servers[1].startCount == 1)

  lifecycle.deactivate()
  #expect(!lifecycle.isActive)
  #expect(factory.servers[1].stopCount == 1)
}

private final class FakeFrontendServerFactory: @unchecked Sendable {
  private let lock = NSLock()
  private(set) var servers: [FakeFrontendServer] = []

  func make() -> FakeFrontendServer {
    lock.withLock {
      let server = FakeFrontendServer(index: servers.count + 1)
      servers.append(server)
      return server
    }
  }
}

private final class FakeFrontendServer: FrontendServerPort, @unchecked Sendable {
  let origin: String
  let token: String
  private(set) var startCount = 0
  private(set) var stopCount = 0

  init(index: Int) {
    origin = "http://127.0.0.1:\(10_000 + index)"
    token = "token-\(index)"
  }

  func start() throws {
    startCount += 1
  }

  func stop() {
    stopCount += 1
  }
}
