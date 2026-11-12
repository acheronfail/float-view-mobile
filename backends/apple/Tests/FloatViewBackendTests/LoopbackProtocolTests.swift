import FloatViewBackend
import Foundation
import Testing

@Suite("Loopback protocol")
struct LoopbackProtocolTests {
  @Test("WebSocket control messages remain bounded to 65,536 UTF-8 bytes")
  func webSocketMessageLimit() {
    #expect(LoopbackProtocolLimits.maximumWebSocketMessageBytes == 65_536)
  }

  @Test("bind failure identifies OS-assigned loopback startup")
  func bindFailure() {
    let message = loopbackStartupFailure(
      error: CocoaError(.fileWriteUnknown)
    )
    #expect(message.contains("failed to bind"))
    #expect(message.contains("OS-assigned loopback port"))
  }
}
