import FloatViewBackend
import Foundation

#if DEBUG
  func printReadiness(
    server: LoopbackServer,
    runtime _: MacBackendRuntime
  ) throws {
    let readiness: [String: Any] = [
      "webSocketUrl": "ws://127.0.0.1:\(server.port)/api/ws",
      "sessionToken": server.token,
      "origin": server.origin,
    ]
    let data = try JSONSerialization.data(withJSONObject: readiness)
    print(String(decoding: data, as: UTF8.self))
    fflush(stdout)
  }
#endif
