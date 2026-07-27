import CryptoKit
import Foundation
import Network
import Security

public final class LoopbackServer: @unchecked Sendable {
  public let token: String
  public private(set) var port: UInt16 = 0
  public var origin: String { "http://127.0.0.1:\(port)" }
  private let platform: String
  private let webRoot: URL
  private let queue = DispatchQueue(label: "fail.acheron.float-view.loopback")
  private var listener: NWListener?

  public init(webRoot: URL, platform: String) {
    self.webRoot = webRoot
    self.platform = platform
    token = Self.randomToken()
  }

  public func start() throws {
    let parameters = NWParameters.tcp
    parameters.requiredLocalEndpoint = .hostPort(host: .ipv4(.loopback), port: .any)
    let listener = try NWListener(using: parameters)
    listener.newConnectionHandler = { [weak self] in self?.accept($0) }
    listener.stateUpdateHandler = { [weak self] state in
      if case .ready = state, let endpoint = listener.port {
        self?.port = endpoint.rawValue
      }
    }
    self.listener = listener
    listener.start(queue: queue)
    while port == 0 { RunLoop.current.run(until: Date(timeIntervalSinceNow: 0.005)) }
  }

  public func stop() {
    listener?.cancel()
    listener = nil
  }

  private func accept(_ connection: NWConnection) {
    connection.start(queue: queue)
    receiveHTTP(connection, buffer: Data())
  }

  private func receiveHTTP(_ connection: NWConnection, buffer: Data) {
    connection.receive(minimumIncompleteLength: 1, maximumLength: 65_536) {
      [weak self] data, _, complete, error in
      guard let self, error == nil else { connection.cancel(); return }
      let joined = buffer + (data ?? Data())
      if joined.range(of: Data("\r\n\r\n".utf8)) == nil {
        if complete || joined.count > 65_536 { connection.cancel() }
        else { self.receiveHTTP(connection, buffer: joined) }
        return
      }
      self.handleHTTP(connection, request: joined)
    }
  }

  private func handleHTTP(_ connection: NWConnection, request: Data) {
    guard let text = String(data: request, encoding: .utf8) else {
      respond(connection, status: "400 Bad Request", body: Data()); return
    }
    let lines = text.components(separatedBy: "\r\n")
    let parts = (lines.first ?? "").split(separator: " ")
    guard parts.count >= 2 else {
      respond(connection, status: "400 Bad Request", body: Data()); return
    }
    let target = String(parts[1])
    let headers: [String: String] = Dictionary(uniqueKeysWithValues: lines.dropFirst().compactMap { line in
      guard let colon = line.firstIndex(of: ":") else { return nil }
      return (
        String(line[..<colon]).lowercased(),
        String(line[line.index(after: colon)...]).trimmingCharacters(in: .whitespaces)
      )
    })
    if target.hasPrefix("/api/v1/ws") {
      guard query(target, "token") == token,
            headers["origin"] == origin,
            headers["upgrade"]?.lowercased() == "websocket",
            let key = headers["sec-websocket-key"]
      else { respond(connection, status: "403 Forbidden", body: Data()); return }
      let accept = Data(Insecure.SHA1.hash(data: Data((key + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11").utf8))).base64EncodedString()
      let response = "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: \(accept)\r\n\r\n"
      connection.send(content: Data(response.utf8), completion: .contentProcessed { [weak self] _ in
        self?.receiveWebSocket(connection, buffer: Data(), initialized: false)
      })
      return
    }
    if target == "/api/bootstrap" {
      let body = try! JSONSerialization.data(withJSONObject: [
        "apiVersion": ["major": 1, "minor": 0],
        "webSocketUrl": "\(origin.replacingOccurrences(of: "http", with: "ws"))/api/v1/ws",
        "sessionToken": token,
      ])
      respond(connection, status: "200 OK", contentType: "application/json", body: body)
      return
    }
    let requested = target == "/" ? "index.html" : String(target.dropFirst())
    let safe = requested.split(separator: "/").filter { $0 != ".." }.joined(separator: "/")
    let url = webRoot.appending(path: safe)
    guard url.path.hasPrefix(webRoot.path), let body = try? Data(contentsOf: url) else {
      respond(connection, status: "404 Not Found", body: Data()); return
    }
    respond(connection, status: "200 OK", contentType: mime(url.pathExtension), body: body)
  }

  private func receiveWebSocket(_ connection: NWConnection, buffer: Data, initialized: Bool) {
    connection.receive(minimumIncompleteLength: 1, maximumLength: 65_536) {
      [weak self] data, _, complete, error in
      guard let self, error == nil else { connection.cancel(); return }
      var joined = buffer + (data ?? Data())
      var hello = initialized
      while let frame = Self.takeFrame(&joined) {
        switch frame.opcode {
        case 1:
          guard frame.payload.count <= 65_536,
                let object = try? JSONSerialization.jsonObject(with: frame.payload),
                let request = object as? [String: Any]
          else { self.sendClose(connection, code: 1007); return }
          let result = self.handle(request, initialized: &hello)
          self.sendFrame(connection, opcode: 1, payload: result)
        case 8: connection.cancel(); return
        case 9: self.sendFrame(connection, opcode: 10, payload: frame.payload)
        default: break
        }
      }
      if complete {
        connection.cancel()
      } else {
        let remaining = Data(joined)
        let nextInitialized = hello
        self.queue.async {
          self.receiveWebSocket(connection, buffer: remaining, initialized: nextInitialized)
        }
      }
    }
  }

  private func handle(_ request: [String: Any], initialized: inout Bool) -> Data {
    let id = request["id"] ?? NSNull()
    let method = request["method"] as? String ?? ""
    func response(_ result: Any) -> Data {
      try! JSONSerialization.data(withJSONObject: ["jsonrpc": "2.0", "id": id, "result": result])
    }
    func failure(_ code: Int, _ kind: String, _ message: String) -> Data {
      try! JSONSerialization.data(withJSONObject: [
        "jsonrpc": "2.0", "id": id,
        "error": ["code": code, "message": message, "data": ["kind": kind, "recoverable": false]],
      ])
    }
    if method == "system.hello" {
      let params = request["params"] as? [String: Any]
      let api = params?["api"] as? [String: Any]
      guard api?["major"] as? Int == 1, api?["minMinor"] as? Int ?? 1 <= 0 else {
        return failure(-32001, "unsupported", "Unsupported API version")
      }
      initialized = true
      return response([
        "api": ["major": 1, "minor": 0],
        "backend": ["name": "float-view-apple", "version": "0.1.0", "platform": platform],
        "capabilities": [["name": "system.snapshot", "available": true]],
        "limits": ["minTelemetryIntervalMs": 50, "maxMessageBytes": 65_536, "maxReconnectDurationMs": 3_600_000],
        "snapshot": snapshot(),
      ])
    }
    guard initialized else { return failure(-32000, "invalid_state", "system.hello required") }
    if method == "system.getSnapshot" { return response(snapshot()) }
    if method == "savedDevices.list" { return response([Any]()) }
    if method == "sessions.list" {
      return response(["items": [Any](), "nextCursor": NSNull()])
    }
    return failure(-32601, "unsupported", "Unknown method: \(method)")
  }

  private func snapshot() -> [String: Any] {
    [
      "revision": 0,
      "bluetooth": ["radio": "powered_on", "scanPermission": "granted", "connectPermission": "granted"],
      "knownDevices": [], "activeConnection": NSNull(),
      "telemetrySubscriptions": [], "loggingJobs": [],
    ]
  }

  private func respond(
    _ connection: NWConnection,
    status: String,
    contentType: String = "text/plain",
    body: Data
  ) {
    let headers = "HTTP/1.1 \(status)\r\nContent-Type: \(contentType)\r\nContent-Length: \(body.count)\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n"
    connection.send(content: Data(headers.utf8) + body, completion: .contentProcessed { _ in
      connection.cancel()
    })
  }

  private func sendFrame(_ connection: NWConnection, opcode: UInt8, payload: Data) {
    var frame = Data([0x80 | opcode])
    if payload.count < 126 { frame.append(UInt8(payload.count)) }
    else if payload.count <= 65_535 {
      frame.append(126)
      frame.append(UInt8(truncatingIfNeeded: payload.count >> 8))
      frame.append(UInt8(truncatingIfNeeded: payload.count))
    } else { sendClose(connection, code: 1009); return }
    frame.append(payload)
    connection.send(content: frame, completion: .contentProcessed { _ in })
  }

  private func sendClose(_ connection: NWConnection, code: UInt16) {
    sendFrame(
      connection,
      opcode: 8,
      payload: Data([UInt8(truncatingIfNeeded: code >> 8), UInt8(truncatingIfNeeded: code)])
    )
    connection.cancel()
  }

  private static func takeFrame(_ buffer: inout Data) -> (opcode: UInt8, payload: Data)? {
    // Data.removeFirst advances startIndex; normalize before integer indexing
    // because subsequent TCP receives may append to that same slice.
    if buffer.startIndex != 0 { buffer = Data(buffer) }
    guard buffer.count >= 2 else { return nil }
    let opcode = buffer[0] & 0x0f
    let masked = buffer[1] & 0x80 != 0
    var length = Int(buffer[1] & 0x7f)
    var offset = 2
    if length == 126 {
      guard buffer.count >= 4 else { return nil }
      length = Int(buffer[2]) << 8 | Int(buffer[3]); offset = 4
    } else if length == 127 {
      return nil
    }
    let maskCount = masked ? 4 : 0
    guard buffer.count >= offset + maskCount + length else { return nil }
    let mask = masked ? Array(buffer[offset..<(offset + 4)]) : []
    offset += maskCount
    var payload = Data(buffer[offset..<(offset + length)])
    if masked {
      for index in payload.indices { payload[index] ^= mask[index % 4] }
    }
    buffer.removeFirst(offset + length)
    return (opcode, payload)
  }

  private func query(_ target: String, _ name: String) -> String? {
    URLComponents(string: "http://localhost\(target)")?.queryItems?.first { $0.name == name }?.value
  }

  private func mime(_ ext: String) -> String {
    switch ext { case "html": "text/html"; case "js": "text/javascript"; case "css": "text/css"; case "svg": "image/svg+xml"; default: "application/octet-stream" }
  }

  private static func randomToken() -> String {
    var bytes = [UInt8](repeating: 0, count: 32)
    _ = SecRandomCopyBytes(kSecRandomDefault, bytes.count, &bytes)
    return Data(bytes).base64EncodedString().replacingOccurrences(of: "/", with: "_")
      .replacingOccurrences(of: "+", with: "-").replacingOccurrences(of: "=", with: "")
  }
}
