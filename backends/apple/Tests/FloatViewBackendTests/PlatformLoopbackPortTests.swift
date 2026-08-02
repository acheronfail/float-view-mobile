import FloatViewBackend
import Foundation
import Testing

@Suite("Platform loopback ports")
struct PlatformLoopbackPortTests {
  @Test("ports are stable and distinct")
  func stablePorts() {
    #expect(PlatformLoopbackPort.macOS == 11_010)
    #expect(PlatformLoopbackPort.iOS == 11_011)
    #expect(PlatformLoopbackPort.macOS != PlatformLoopbackPort.iOS)
  }

  @Test("WebSocket control messages remain bounded to 65,536 UTF-8 bytes")
  func webSocketMessageLimit() {
    #expect(LoopbackProtocolLimits.maximumWebSocketMessageBytes == 65_536)
  }

  @Test("bind failure identifies the unavailable address")
  func bindFailure() {
    let message = loopbackStartupFailure(
      port: PlatformLoopbackPort.macOS,
      error: CocoaError(.fileWriteUnknown)
    )
    #expect(message.contains("failed to bind"))
    #expect(message.contains("127.0.0.1:11010"))
  }
}
