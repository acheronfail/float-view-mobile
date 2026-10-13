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
  let onLoadComplete: () -> Void
  let onLoadFailure: () -> Void

  func makeCoordinator() -> Coordinator {
    Coordinator(
      onContentProcessTerminated: onContentProcessTerminated,
      onLoadComplete: onLoadComplete,
      onLoadFailure: onLoadFailure
    )
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
    let traits = UITraitCollection(userInterfaceStyle: colorScheme == .dark ? .dark : .light)
    let color = UIColor(named: "Canvas")!.resolvedColor(with: traits)
    // Let the matching native canvas show through before WebKit paints the document.
    webView.isOpaque = false
    webView.backgroundColor = color
    webView.underPageBackgroundColor = color
    webView.scrollView.backgroundColor = color
  }

  final class Coordinator: NSObject, WKNavigationDelegate, WKScriptMessageHandler {
    private let onContentProcessTerminated: () -> Void
    private let onLoadComplete: () -> Void
    private let onLoadFailure: () -> Void
    private var loadedURL: URL?

    init(
      onContentProcessTerminated: @escaping () -> Void,
      onLoadComplete: @escaping () -> Void,
      onLoadFailure: @escaping () -> Void
    ) {
      self.onContentProcessTerminated = onContentProcessTerminated
      self.onLoadComplete = onLoadComplete
      self.onLoadFailure = onLoadFailure
    }

    func webView(_: WKWebView, didFinish _: WKNavigation!) {
      onLoadComplete()
    }

    func webView(_: WKWebView, didFailProvisionalNavigation _: WKNavigation!, withError _: Error) {
      onLoadFailure()
    }

    func webView(_: WKWebView, didFail _: WKNavigation!, withError _: Error) {
      onLoadFailure()
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
