import FloatViewBackend
import Foundation

func printReadiness(
  server: LoopbackServer,
  runtime: MacBackendRuntime
) throws {
  let readiness: [String: Any] = [
    "webSocketUrl": "ws://127.0.0.1:\(server.port)/api/ws",
    "sessionToken": server.token,
    "origin": server.origin,
    "savedDeviceId": runtime.testSavedDeviceID ?? NSNull(),
  ]
  let data = try JSONSerialization.data(withJSONObject: readiness)
  print(String(decoding: data, as: UTF8.self))
  fflush(stdout)
}
