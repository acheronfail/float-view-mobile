//
//  WebView.swift
//  float-view
//
//  Created on 25/8/2025.
//

import SwiftUI
import WebKit

struct WebView: UIViewRepresentable {
  let url: URL
  let onContentProcessTerminated: () -> Void

  func makeCoordinator() -> Coordinator {
    Coordinator(onContentProcessTerminated: onContentProcessTerminated)
  }

  func makeUIView(context: Context) -> WKWebView {
    let configuration = WKWebViewConfiguration()

    let webView = WKWebView(frame: .zero, configuration: configuration)
    webView.navigationDelegate = context.coordinator

    if #available(iOS 16.4, *) {
      webView.isInspectable = true
    }

    webView.isOpaque = false
    context.coordinator.load(url, in: webView)
    return webView
  }

  func updateUIView(_ webView: WKWebView, context: Context) {
    context.coordinator.load(url, in: webView)
  }

  final class Coordinator: NSObject, WKNavigationDelegate {
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
      DispatchQueue.main.async { [onContentProcessTerminated] in
        onContentProcessTerminated()
      }
    }
  }
}
