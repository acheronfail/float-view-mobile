import CommonCrypto
import Foundation
import Network

class LocalWebSocket {
  private let logTag = "FloatView_LocalWebSocket"
  private var webSocketClients: [String: WebSocketClient] = [:]
  private let queue = DispatchQueue(label: "LocalWebSocket")

  func isWebSocketUpgrade(path: String, headers: [String: String]) -> Bool {
    return path == "/ws" && headers["upgrade"]?.lowercased() == "websocket"
      && headers["connection"]?.lowercased().contains("upgrade") == true
  }

  func handleWebSocketUpgrade(
    connection: NWConnection,
    headers: [String: String],
    handlers: [String: WsHandler]
  ) {
    guard let webSocketKey = headers["sec-websocket-key"] else {
      print("Missing sec-websocket-key header")
      connection.cancel()
      return
    }

    let acceptKey = generateWebSocketAcceptKey(webSocketKey)
    let response = """
      HTTP/1.1 101 Switching Protocols\r
      Upgrade: websocket\r
      Connection: Upgrade\r
      Sec-WebSocket-Accept: \(acceptKey)\r
      \r

      """

    connection.send(
      content: response.data(using: .utf8)!,
      completion: .contentProcessed { [weak self] error in
        if let error = error {
          print("Error sending WebSocket handshake: \(error)")
          connection.cancel()
          return
        }

        print("WebSocket connection established")
        let clientId = "client_\(Int(Date().timeIntervalSince1970))"
        let client = WebSocketClient(connection: connection, id: clientId)

        // Add handlers to client
        for (commandName, handler) in handlers {
          client.addHandler(commandName, handler: handler)
        }

        self?.webSocketClients[clientId] = client
        self?.sendWebSocketMessage(client: client, message: BackendMessage.socketWelcome())
        self?.handleWebSocketMessages(client: client)
      }
    )
  }

  private func handleWebSocketMessages(client: WebSocketClient) {
    client.connection.receive(minimumIncompleteLength: 1, maximumLength: 8192) {
      [weak self] data, _, isComplete, error in
      if let error = error {
        print(
          "\(self?.logTag ?? "LocalWebSocket"): WebSocket client \(client.id) disconnected: \(error)"
        )
        self?.webSocketClients.removeValue(forKey: client.id)
        client.connection.cancel()
        return
      }

      if let data = data, !data.isEmpty {
        if let frame = self?.parseWebSocketFrame(data: data) {
          do {
            let payload = frame.payload.data(using: .utf8) ?? Data()
            let command =
              try JSONSerialization.jsonObject(with: payload, options: []) as? [String: Any]

              // FIXME: use this object and setup handler - remove "handlers" logic
              let cmd = try JSONDecoder().decode(BackendCommand.self, from: payload)
              
            if let command = command,
              let cmd = command["cmd"] as? String,
              let data = command["data"] as? [String: Any]
            {

              print("ws<: '\(cmd)': \(data)")

              if let handler = client.getHandler(cmd) {
                handler(data)
              } else {
                self?.sendWebSocketMessage(client: client, message: BackendMessage.socketError(message: "Unhandled websocket command: \(cmd)"))
              }
            }
          } catch {
            print("\(self?.logTag ?? "LocalWebSocket"): Error parsing JSON command: \(error)")
          }
        }
      }

      if !isComplete {
        self?.handleWebSocketMessages(client: client)
      }
    }
  }

  private func sendWebSocketMessage(client: WebSocketClient, message: [String: Any]) {
    do {
      let jsonData = try JSONSerialization.data(withJSONObject: message, options: [])
      let frame = createWebSocketFrame(payload: jsonData)

      client.connection.send(
        content: frame,
        completion: .contentProcessed { error in
          if let error = error {
            print("Error sending WebSocket message: \(error)")
          }
        }
      )
    } catch {
      print("\(logTag): Error serializing message: \(error)")
    }
  }

  func broadcastMessage(_ message: [String: Any]) {
    print("ws>: '\(message)'")
    queue.async { [weak self] in
      self?.webSocketClients.values.forEach { client in
        self?.sendWebSocketMessage(client: client, message: message)
      }
    }
  }

  private func parseWebSocketFrame(data: Data) -> WebSocketFrame? {
    guard data.count >= 2 else { return nil }

    let firstByte = data[0]
    let secondByte = data[1]

    let opcode = firstByte & 0x0F

    // Only handle text frames (opcode 1)
    guard opcode == 1 else { return nil }

    let masked = (secondByte & 0x80) != 0
    var payloadLength = Int(secondByte & 0x7F)
    var offset = 2

    // Handle extended payload length
    if payloadLength == 126 {
      guard data.count >= offset + 2 else { return nil }
      payloadLength = Int(data[offset]) << 8 | Int(data[offset + 1])
      offset += 2
    } else if payloadLength == 127 {
      // Don't handle large payloads for simplicity
      return nil
    }

    // Extract masking key
    var maskingKey: [UInt8] = []
    if masked {
      guard data.count >= offset + 4 else { return nil }
      maskingKey = Array(data[offset..<offset + 4])
      offset += 4
    }

    // Extract payload
    guard data.count >= offset + payloadLength else { return nil }
    var payload = Array(data[offset..<offset + payloadLength])

    // Unmask payload
    if masked {
      for i in 0..<payload.count {
        payload[i] ^= maskingKey[i % 4]
      }
    }

    guard let payloadString = String(bytes: payload, encoding: .utf8) else { return nil }

    return WebSocketFrame(opcode: Int(opcode), payload: payloadString)
  }

  private func createWebSocketFrame(payload: Data) -> Data {
    let payloadLength = payload.count
    var frame = Data()

    // First byte: FIN=1, RSV=000, Opcode=0001 (text frame)
    frame.append(0x81)

    // Second byte and payload length (server frames are never masked)
    if payloadLength < 126 {
      frame.append(UInt8(payloadLength))
    } else if payloadLength < 65536 {
      frame.append(126)
      frame.append(UInt8(payloadLength >> 8))
      frame.append(UInt8(payloadLength & 0xFF))
    } else {
      // Don't handle large payloads for simplicity
      frame.append(UInt8(payloadLength))
    }

    // Append payload (no masking for server-to-client frames)
    frame.append(payload)

    return frame
  }

  private func generateWebSocketAcceptKey(_ key: String) -> String {
    let magicString = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11"
    let combined = key + magicString
    let hash = combined.data(using: .utf8)!.sha1
    return hash.base64EncodedString()
  }

  func removeAllClients() {
    webSocketClients.values.forEach { client in
      client.connection.cancel()
    }
    webSocketClients.removeAll()
  }
}
