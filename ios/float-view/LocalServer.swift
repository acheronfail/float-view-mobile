//
//  LocalServer.swift
//  float-view
//
//  Created on 25/8/2025.
//

import Foundation
import Network

class LocalServer: ObservableObject {
    static let shared = LocalServer()

    private var listener: NWListener?
    private var connections: [NWConnection] = []
    @Published var isRunning = false
    @Published var port: UInt16 = 0

    private let queue = DispatchQueue(label: "LocalServer")

    func start() {
        guard !isRunning else { return }

        do {
            // Create listener with random available port
            let parameters = NWParameters.tcp
            parameters.allowLocalEndpointReuse = true

            listener = try NWListener(using: parameters, on: .any)
            listener?.newConnectionHandler = { [weak self] connection in
                self?.handleConnection(connection)
            }

            listener?.stateUpdateHandler = { [weak self] state in
                switch state {
                case .ready:
                    if let port = self?.listener?.port {
                        DispatchQueue.main.async {
                            self?.port = port.rawValue
                            self?.isRunning = true
                        }
                        print("Server started on port \(port)")
                    }
                case .failed(let error):
                    print("Server failed: \(error)")
                    DispatchQueue.main.async {
                        self?.isRunning = false
                        self?.port = 0
                    }
                case .cancelled:
                    DispatchQueue.main.async {
                        self?.isRunning = false
                        self?.port = 0
                    }
                default:
                    break
                }
            }

            listener?.start(queue: queue)
        } catch {
            print("Failed to start server: \(error)")
        }
    }

    func stop() {
        listener?.cancel()
        connections.forEach { $0.cancel() }
        connections.removeAll()

        DispatchQueue.main.async {
            self.isRunning = false
            self.port = 0
        }
    }

    private func handleConnection(_ connection: NWConnection) {
        connections.append(connection)

        connection.stateUpdateHandler = { [weak self] state in
            switch state {
            case .ready:
                self?.receiveRequest(from: connection)
            case .cancelled, .failed:
                self?.connections.removeAll { $0 === connection }
            default:
                break
            }
        }

        connection.start(queue: queue)
    }

    private func receiveRequest(from connection: NWConnection) {
        connection.receive(minimumIncompleteLength: 1, maximumLength: 8192) { [weak self] data, _, isComplete, error in
            if let error = error {
                print("Error receiving request: \(error)")
                connection.cancel()
                return
            }

            if let data = data, !data.isEmpty {
                self?.processRequest(data: data, connection: connection)
            } else if isComplete {
                // No data and connection is complete, close it
                connection.cancel()
            } else {
                // Continue receiving if not complete and no error
                self?.receiveRequest(from: connection)
            }
        }
    }

    private func processRequest(data: Data, connection: NWConnection) {
        guard let requestString = String(data: data, encoding: .utf8) else {
            sendResponse(connection: connection, status: "400 Bad Request", contentType: "text/plain", body: "Bad Request - req not utf8")
            return
        }

        let lines = requestString.components(separatedBy: "\r\n")
        guard let requestLine = lines.first else {
            sendResponse(connection: connection, status: "400 Bad Request", contentType: "text/plain", body: "Bad Request - failed to parse first line")
            return
        }

        let components = requestLine.components(separatedBy: " ")
        guard components.count >= 2 else {
            sendResponse(connection: connection, status: "400 Bad Request", contentType: "text/plain", body: "Bad Request")
            return
        }

        let method = components[0]
        let path = components[1]

        print("Handling request: \(method) \(path)")

        // Parse headers
        var headers: [String: String] = [:]
        for line in lines.dropFirst() {
            if line.isEmpty { break }
            let parts = line.components(separatedBy: ": ")
            if parts.count >= 2 {
                headers[parts[0].lowercased()] = parts[1]
            }
        }

        // Handle WebSocket upgrade
        if path == "/ws" &&
           headers["upgrade"]?.lowercased() == "websocket" &&
           headers["connection"]?.lowercased().contains("upgrade") == true {
            handleWebSocketUpgrade(connection: connection, headers: headers)
            return
        }

        // Handle HTTP requests
        switch path {
        case "/":
            sendResponse(connection: connection, status: "200 OK", contentType: "text/html", body: loadHtmlFile(filename: "index.html"))
        case "/api/status":
            let json = """
                {"status":"ok","message":"Local server is running","port":\(port)}
                """
            sendResponse(connection: connection, status: "200 OK", contentType: "application/json", body: json)
        default:
            if let fileContent = loadWebFile(path: path) {
                let contentType = getContentType(for: path)
                sendResponse(connection: connection, status: "200 OK", contentType: contentType, body: fileContent)
            } else {
                sendResponse(connection: connection, status: "404 Not Found", contentType: "text/html", body: loadHtmlFile(filename: "404.html"))
            }
        }
    }

    private func handleWebSocketUpgrade(connection: NWConnection, headers: [String: String]) {
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

        connection.send(content: response.data(using: .utf8)!, completion: .contentProcessed { error in
            if let error = error {
                print("Error sending WebSocket handshake: \(error)")
                connection.cancel()
                return
            }

            print("WebSocket connection established")
            let welcomeMessage = """
                {"type":"welcome","message":"Connected to Float View Mobile","clientId":"client_\(Int(Date().timeIntervalSince1970))"}
                """
            self.sendWebSocketMessage(connection: connection, message: welcomeMessage)
            self.handleWebSocketMessages(connection: connection)
        })
    }

    private func generateWebSocketAcceptKey(_ key: String) -> String {
        let magicString = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11"
        let combined = key + magicString
        let hash = combined.data(using: .utf8)!.sha1
        return hash.base64EncodedString()
    }

    private func handleWebSocketMessages(connection: NWConnection) {
        connection.receive(minimumIncompleteLength: 1, maximumLength: 8192) { [weak self] data, _, isComplete, error in
            if let error = error {
                print("WebSocket error: \(error)")
                return
            }

            if let data = data, !data.isEmpty {
                if let frame = self?.parseWebSocketFrame(data: data) {
                    print("Received WebSocket message: \(frame.payload)")

                    // Echo the message back
                    let echoMessage = """
                        {"type":"echo","original":\(frame.payload),"timestamp":\(Int(Date().timeIntervalSince1970 * 1000)),"from":"server"}
                        """
                    self?.sendWebSocketMessage(connection: connection, message: echoMessage)
                }
            }

            if !isComplete {
                self?.handleWebSocketMessages(connection: connection)
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

    private func sendWebSocketMessage(connection: NWConnection, message: String) {
        let messageData = message.data(using: .utf8)!
        let frame = createWebSocketFrame(payload: messageData)

        connection.send(content: frame, completion: .contentProcessed { error in
            if let error = error {
                print("Error sending WebSocket message: \(error)")
            }
        })
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

    private func sendResponse(connection: NWConnection, status: String, contentType: String, body: String) {
        let bodyData = body.data(using: .utf8) ?? Data()
        let response = """
            HTTP/1.1 \(status)\r
            Content-Type: \(contentType); charset=utf-8\r
            Content-Length: \(bodyData.count)\r
            Connection: close\r
            \r
            \(body)
            """

        guard let responseData = response.data(using: .utf8) else {
            connection.cancel()
            return
        }

        connection.send(content: responseData, completion: .contentProcessed { error in
            if let error = error {
                print("Error sending response: \(error)")
            }
            // Close connection after response is sent
            connection.cancel()
        })
    }

    private func loadWebFile(path: String) -> String? {
        // Remove leading slash and decode URL encoding
        let fileName = path.first == "/" ? String(path.dropFirst()) : path
        guard !fileName.isEmpty,
              !fileName.contains("..") else { // Basic security check
            return nil
        }

        if let bundleUrl = Bundle.main.url(forResource: fileName, withExtension: nil, subdirectory: "web") {
            do {
                let content = try String(contentsOf: bundleUrl, encoding: .utf8)
                print("Successfully loaded web file from app bundle: web/\(fileName)")
                return content
            } catch {
                print("Error reading bundled web file web/\(fileName): \(error)")
            }
        }

        return nil
    }

    private func getContentType(for path: String) -> String {
        let pathExtension = (path as NSString).pathExtension.lowercased()

        switch pathExtension {
        case "html", "htm":
            return "text/html"
        case "css":
            return "text/css"
        case "js":
            return "application/javascript"
        case "json":
            return "application/json"
        case "png":
            return "image/png"
        case "jpg", "jpeg":
            return "image/jpeg"
        case "gif":
            return "image/gif"
        case "svg":
            return "image/svg+xml"
        case "ico":
            return "image/x-icon"
        case "txt":
            return "text/plain"
        default:
            return "application/octet-stream"
        }
    }

    private func loadHtmlFile(filename: String) -> String {
        if let text = loadWebFile(path: filename) {
            return text.replacingOccurrences(of: "{{PORT}}", with: "\(port)")
        }

        print("⚠️ Could not find HTML file \(filename)!")
        return "<html><body>💩️Could not find: \(filename)</body></html>"
    }
}

struct WebSocketFrame {
    let opcode: Int
    let payload: String
}

// Import CommonCrypto for SHA-1
import CommonCrypto

// Extension to add SHA-1 hashing
extension Data {
    var sha1: Data {
        var digest = [UInt8](repeating: 0, count: Int(CC_SHA1_DIGEST_LENGTH))
        _ = self.withUnsafeBytes { bytes in
            CC_SHA1(bytes.bindMemory(to: UInt8.self).baseAddress, CC_LONG(self.count), &digest)
        }
        return Data(digest)
    }
}
