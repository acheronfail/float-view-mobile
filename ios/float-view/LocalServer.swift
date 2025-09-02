import CommonCrypto
import Foundation
import Network

class LocalServer: ObservableObject {
  static let shared = LocalServer()

  private let webSocket = LocalWebSocket()
  private var listener: NWListener?
  private var connections: [NWConnection] = []
  @Published var isRunning = false
  @Published var port: UInt16 = 0

  private let queue = DispatchQueue(label: "LocalServer")
  private var routes: [String: (HttpContext) -> Void] = [:]
  private var wsHandlers: [String: WsHandler] = [:]

  func addRoute(_ path: String, handler: @escaping (HttpContext) -> Void) {
    routes[path] = handler
  }

  func broadcastMessage(_ message: [String: Any]) {
    webSocket.broadcastMessage(message)
  }

  func addHandler(_ commandName: String, handler: @escaping WsHandler) {
    wsHandlers[commandName] = handler
  }

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
    webSocket.removeAllClients()

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
    connection.receive(minimumIncompleteLength: 1, maximumLength: 8192) {
      [weak self] data, _, isComplete, error in
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
      sendResponse(
        connection: connection, status: "400 Bad Request", contentType: "text/plain",
        body: "Bad Request - req not utf8")
      return
    }

    let lines = requestString.components(separatedBy: "\r\n")
    guard let requestLine = lines.first else {
      sendResponse(
        connection: connection, status: "400 Bad Request", contentType: "text/plain",
        body: "Bad Request - failed to parse first line")
      return
    }

    let components = requestLine.components(separatedBy: " ")
    guard components.count >= 2 else {
      sendResponse(
        connection: connection, status: "400 Bad Request", contentType: "text/plain",
        body: "Bad Request")
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
    if webSocket.isWebSocketUpgrade(path: path, headers: headers) {
      webSocket.handleWebSocketUpgrade(connection: connection, headers: headers, handlers: wsHandlers)
      return
    }

    // Create HTTP context
    let ctx = HttpContext(connection: connection, method: method, path: path, headers: headers)

    // Check for custom route handler
    if let handler = routes[path] {
      handler(ctx)
      return
    }

    // Handle default routes
    switch path {
    case "/":
      ctx.sendResponse(status: "200 OK", contentType: "text/html", body: loadHtmlFile(path: "index.html"))
    case "/api/status":
      let json = """
        {"status":"ok","message":"Local server is running","port":\(port)}
        """
      ctx.sendResponse(status: "200 OK", contentType: "application/json", body: json)
    default:
      if let fileContent = loadWebFile(path: path) {
        let contentType = getContentType(for: path)
        ctx.sendResponse(status: "200 OK", contentType: contentType, body: fileContent)
      } else {
        ctx.sendResponse(status: "404 Not Found", contentType: "text/html", body: fallbackHtml(path: path))
      }
    }
  }



  private func sendResponse(
    connection: NWConnection, status: String, contentType: String, body: String
  ) {
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

    connection.send(
      content: responseData,
      completion: .contentProcessed { error in
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
    guard !fileName.isEmpty, !fileName.contains("..") else {  // Basic security check
      return nil
    }

    if let bundleUrl = Bundle.main.url(
      forResource: fileName, withExtension: nil, subdirectory: "build")
    {
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

  private func loadHtmlFile(path: String) -> String {
    if let text = loadWebFile(path: path) {
      return text.replacingOccurrences(of: "{{PORT}}", with: "\(port)")
    }

    print("⚠️ Could not find HTML file \(path)!")
    return fallbackHtml(path: path)
  }

  private func fallbackHtml(path: String) -> String {
    return "<html><head><meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\"></head><body style=\"color: white\">💩️Could not find: \(path)</body></html>"
  }
}
