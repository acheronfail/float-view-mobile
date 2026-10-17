import AppKit
@testable import FloatViewMac
import Foundation
import Network
import WebKit
import XCTest

/// Real WebKit requests to a bounded synthetic loopback image server; no provider traffic.
final class MacWebViewNetworkTests: XCTestCase, @unchecked Sendable {
  @MainActor
  func testHostExposesNavigationPolicyToWebKit() {
    let host = MacAppDelegate(html: Data())
    XCTAssertTrue(host.responds(to: NSSelectorFromString("webView:decidePolicyForNavigationAction:decisionHandler:")))
    XCTAssertTrue(host.responds(to: NSSelectorFromString("webView:decidePolicyForNavigationResponse:decisionHandler:")))
  }

  @MainActor
  func testIdentifiedImageRequestsUseWebKitCache() async throws {
    _ = NSApplication.shared
    let server = try AuditImageServer()
    let started = expectation(description: "Loopback capture listener ready")
    server.start { started.fulfill() }
    await fulfillment(of: [started], timeout: 10)
    defer { server.stop() }
    let url = try XCTUnwrap(server.imageURL)
    let receiver = AuditImageReceiver()
    let config = MacWebViewConfiguration.make(version: "1.0.0")
    config.userContentController.add(receiver, name: "loaded")
    let html = "<img src='\(url.absoluteString)' onload='webkit.messageHandlers.loaded.postMessage(true)' onerror='webkit.messageHandlers.loaded.postMessage(false)'>"
    var view: WKWebView? = WKWebView(frame: CGRect(x: 0, y: 0, width: 1100, height: 760), configuration: config)
    let first = expectation(description: "First image loaded")
    receiver.completion = { loaded in XCTAssertTrue(loaded); first.fulfill() }
    view?.loadHTMLString(html, baseURL: url.deletingLastPathComponent())
    await fulfillment(of: [first], timeout: 15)
    XCTAssertEqual(server.requests.count, 1)
    let headers = try XCTUnwrap(server.requests.first)
    XCTAssertTrue(headers["user-agent", default: ""].contains("FloatView/1.0.0"))
    XCTAssertNil(headers["authorization"])
    XCTAssertNil(headers["cache-control"])
    XCTAssertNil(headers["pragma"])
    view?.stopLoading()
    view = nil
    // Recreate the WebView, as the app can do on foreground return.
    view = WKWebView(frame: CGRect(x: 0, y: 0, width: 1100, height: 760), configuration: config)
    let second = expectation(description: "Cached image loaded in recreated WebView")
    receiver.completion = { loaded in XCTAssertTrue(loaded); second.fulfill() }
    view?.loadHTMLString(html, baseURL: url.deletingLastPathComponent())
    await fulfillment(of: [second], timeout: 15)
    XCTAssertEqual(server.requests.count, 1, "Unexpired image must not be downloaded twice")
    let attachment =
      XCTAttachment(
        string: "First request headers: \(headers)\nRequests after recreated WebView: \(server.requests.count)"
      )
    attachment.lifetime = .keepAlways
    add(attachment)
    config.userContentController.removeScriptMessageHandler(forName: "loaded")
    view?.stopLoading()
  }
}

@MainActor
private final class AuditImageReceiver: NSObject, WKScriptMessageHandler {
  var completion: ((Bool) -> Void)?
  func userContentController(_: WKUserContentController, didReceive message: WKScriptMessage) {
    completion?(message.body as? Bool == true)
  }
}

private final class AuditImageServer: @unchecked Sendable {
  private let listener: NWListener
  private let queue = DispatchQueue(label: "FloatView.MapNetworkAudit")
  private let lock = NSLock()
  private var captured: [[String: String]] = []
  var requests: [[String: String]] {
    lock.lock(); defer { lock.unlock() }; return captured
  }

  var imageURL: URL? {
    listener.port.flatMap { URL(string: "http://127.0.0.1:\($0.rawValue)/tile.png") }
  }

  init() throws {
    let parameters = NWParameters.tcp
    parameters.requiredLocalEndpoint = .hostPort(host: .ipv4(.loopback), port: .any)
    listener = try NWListener(using: parameters)
  }

  func start(ready: @escaping @Sendable () -> Void) {
    listener.stateUpdateHandler = {
      state in if case .ready = state {
        ready()
      }
    }
    listener.newConnectionHandler = { [weak self] connection in
      guard let self else { connection.cancel(); return }
      connection.start(queue: queue)
      receive(connection, bytes: Data())
    }
    listener.start(queue: queue)
  }

  func stop() {
    listener.cancel()
  }

  private func receive(_ connection: NWConnection, bytes: Data) {
    connection.receive(minimumIncompleteLength: 1, maximumLength: 16384) { [weak self] data, _, complete, error in
      guard let self else { connection.cancel(); return }
      var bytes = bytes
      if let data {
        bytes.append(data)
      }
      guard bytes.count <= 16384, error == nil else { connection.cancel(); return }
      if let text = String(data: bytes, encoding: .utf8), text.contains("\r\n\r\n") {
        var headers: [String: String] = [:]
        for line in text.components(separatedBy: "\r\n").dropFirst() {
          guard let separator = line.firstIndex(of: ":") else { continue }
          headers[String(line[..<separator]).lowercased()] = line[line.index(after: separator)...]
            .trimmingCharacters(in: .whitespaces)
        }
        lock.lock(); captured.append(headers); lock.unlock()
        let image =
          Data(
            base64Encoded: "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+jRZkAAAAASUVORK5CYII="
          )!
        let date = DateFormatter()
        date.locale = Locale(identifier: "en_US_POSIX")
        date.timeZone = TimeZone(secondsFromGMT: 0)
        date.dateFormat = "EEE, dd MMM yyyy HH:mm:ss 'GMT'"
        let response = "HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nContent-Length: \(image.count)\r\nCache-Control: public, max-age=3600\r\nETag: \"float-view-audit\"\r\nDate: \(date.string(from: Date()))\r\nConnection: close\r\n\r\n"
        connection.send(
          content: Data(response.utf8) + image,
          completion: .contentProcessed { _ in connection.cancel() }
        )
      } else if complete {
        connection.cancel()
      } else {
        receive(connection, bytes: bytes)
      }
    }
  }
}
