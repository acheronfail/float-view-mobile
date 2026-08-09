//
//  WebView.swift
//  float-view
//
//  Created on 25/8/2025.
//

import SwiftUI
import WebKit

struct WebView: UIViewRepresentable {
  @Environment(\.colorScheme) private var colorScheme

  let url: URL
  let onContentProcessTerminated: () -> Void

  func makeCoordinator() -> Coordinator {
    Coordinator(onContentProcessTerminated: onContentProcessTerminated)
  }

  func makeUIView(context: Context) -> WKWebView {
    let configuration = WKWebViewConfiguration()

    #if DEBUG
      configuration.userContentController.add(context.coordinator, name: "floatViewPerformance")
    #endif

    let webView = WKWebView(frame: .zero, configuration: configuration)
    webView.navigationDelegate = context.coordinator
    webView.allowsLinkPreview = false

    if #available(iOS 16.4, *) {
      webView.isInspectable = true
    }

    applyCanvasColor(to: webView)
    webView.scrollView.pinchGestureRecognizer?.isEnabled = false
    context.coordinator.load(url, in: webView)
    return webView
  }

  func updateUIView(_ webView: WKWebView, context: Context) {
    applyCanvasColor(to: webView)
    context.coordinator.load(url, in: webView)
  }

  private func applyCanvasColor(to webView: WKWebView) {
    let color = colorScheme == .dark
      ? UIColor(red: 2 / 255, green: 6 / 255, blue: 23 / 255, alpha: 1)
      : UIColor(red: 248 / 255, green: 250 / 255, blue: 252 / 255, alpha: 1)
    webView.isOpaque = true
    webView.backgroundColor = color
    webView.scrollView.backgroundColor = color
  }

  final class Coordinator: NSObject, WKNavigationDelegate, WKScriptMessageHandler {
    private let onContentProcessTerminated: () -> Void
    private var loadedURL: URL?

    init(onContentProcessTerminated: @escaping () -> Void) {
      self.onContentProcessTerminated = onContentProcessTerminated
    }

    func load(_ url: URL, in webView: WKWebView) {
      guard loadedURL != url else { return }
      loadedURL = url
      webView.load(URLRequest(url: url))
    }

    func webViewWebContentProcessDidTerminate(_: WKWebView) {
      #if DEBUG
        print("[FloatView WebView] web content process terminated; recovering frontend")
      #endif
      DispatchQueue.main.async { [onContentProcessTerminated] in
        onContentProcessTerminated()
      }
    }

    func userContentController(_: WKUserContentController, didReceive message: WKScriptMessage) {
      guard message.name == "floatViewPerformance" else { return }
      guard
        let body = message.body as? [String: Any],
        let data = try? JSONSerialization.data(withJSONObject: body, options: [.sortedKeys]),
        let text = String(data: data, encoding: .utf8)
      else {
        print("[FloatView frontend performance] \(message.body)")
        return
      }
      print("[FloatView frontend performance] \(text)")
    }
  }
}
