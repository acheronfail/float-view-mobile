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
    configuration.applicationNameForUserAgent = "FloatView/\(Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String ?? "development")"
    configuration.userContentController.add(context.coordinator, name: "floatViewBackNavigation")

    #if DEBUG
      configuration.userContentController.add(context.coordinator, name: "floatViewPerformance")
    #endif

    let webView = HostLayoutWebView(frame: .zero, configuration: configuration)
    webView.navigationDelegate = context.coordinator
    webView.uiDelegate = context.coordinator
    webView.allowsLinkPreview = false
    context.coordinator.installEdgeBack(in: webView)

    #if DEBUG
      webView.isInspectable = true
    #endif

    applyCanvasColor(to: webView)
    webView.scrollView.pinchGestureRecognizer?.isEnabled = false
    context.coordinator.load(url, in: webView)
    return webView
  }

  func updateUIView(_ webView: WKWebView, context: Context) {
    applyCanvasColor(to: webView)
    context.coordinator.load(url, in: webView)
  }

  static func dismantleUIView(_ webView: WKWebView, coordinator: Coordinator) {
    coordinator.stopEdgeBack()
    webView.configuration.userContentController.removeScriptMessageHandler(forName: "floatViewBackNavigation")
    #if DEBUG
      webView.configuration.userContentController.removeScriptMessageHandler(forName: "floatViewPerformance")
    #endif
    webView.navigationDelegate = nil
    webView.uiDelegate = nil
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

  final class Coordinator: NSObject, WKNavigationDelegate, WKUIDelegate, WKScriptMessageHandler,
    UIGestureRecognizerDelegate
  {
    private let onContentProcessTerminated: () -> Void
    private let onLoadComplete: () -> Void
    private let onLoadFailure: () -> Void
    private var loadedURL: URL?
    private weak var edgeBack: UIScreenEdgePanGestureRecognizer?
    private enum EdgePhase { case idle, dragging, waitingForSnapshot, navigating, settling }
    private var edgePhase = EdgePhase.idle
    private var captureDeadline: DispatchWorkItem?
    private var edgePresentation: EdgeBackPresentation?
    private var edgeGeneration = 0
    private var snapshotPending = false
    private var edgeOffset: CGFloat = 0

    init(
      onContentProcessTerminated: @escaping () -> Void,
      onLoadComplete: @escaping () -> Void,
      onLoadFailure: @escaping () -> Void
    ) {
      self.onContentProcessTerminated = onContentProcessTerminated
      self.onLoadComplete = onLoadComplete
      self.onLoadFailure = onLoadFailure
    }

    func webView(_ webView: WKWebView, didFinish _: WKNavigation!) {
      onLoadComplete()
      (webView as? HostLayoutWebView)?.documentLoaded()
    }

    func installEdgeBack(in webView: WKWebView) {
      let gesture = UIScreenEdgePanGestureRecognizer(target: self, action: #selector(handleEdgeBack(_:)))
      gesture.edges = .left
      gesture.maximumNumberOfTouches = 1
      gesture.delegate = self
      gesture.isEnabled = false
      webView.addGestureRecognizer(gesture)
      // A vertical edge gesture fails the direction gate promptly, releasing ordinary scrolling.
      webView.scrollView.panGestureRecognizer.require(toFail: gesture)
      edgeBack = gesture
      NotificationCenter.default.addObserver(self, selector: #selector(resetEdgePresentation),
                                             name: UIApplication.willResignActiveNotification, object: nil)
      NotificationCenter.default.addObserver(self, selector: #selector(resetEdgePresentation),
                                             name: UIDevice.orientationDidChangeNotification, object: nil)
    }

    func gestureRecognizerShouldBegin(_ gestureRecognizer: UIGestureRecognizer) -> Bool {
      guard let gesture = gestureRecognizer as? UIScreenEdgePanGestureRecognizer, edgePhase == .idle
      else {
        return false
      }
      let velocity = gesture.velocity(in: gesture.view)
      return velocity.x > 0 && velocity.x > abs(velocity.y)
    }

    func gestureRecognizer(_: UIGestureRecognizer,
                           shouldRecognizeSimultaneouslyWith _: UIGestureRecognizer) -> Bool
    {
      // WebKit's content-touch recognizers must coexist with the native edge recognizer.
      // The explicit failure rule above still arbitrates document scrolling.
      true
    }

    @objc private func handleEdgeBack(_ gesture: UIScreenEdgePanGestureRecognizer) {
      guard let webView = gesture.view as? WKWebView else { return }
      let translation = gesture.translation(in: webView)
      edgeOffset = translation.x
      switch gesture.state {
      case .began:
        edgePhase = .dragging
        edgeGeneration += 1
        // An older cancelled capture may still be in flight. Keep capture bounded to one request.
        guard !snapshotPending else { return }
        let generation = edgeGeneration
        snapshotPending = true
        let configuration = WKSnapshotConfiguration()
        configuration.rect = webView.bounds
        webView.takeSnapshot(with: configuration) { [weak self, weak webView] image, _ in
          let accept = { [weak self, weak webView] in
            guard let self else { return }
            self.snapshotPending = false
            guard self.edgeGeneration == generation,
                  self.edgePhase == .dragging || self.edgePhase == .waitingForSnapshot,
                  let webView else { return }
            if let image {
              self.edgePresentation = EdgeBackPresentation(webView: webView, image: image)
              self.edgePresentation?.track(self.edgeOffset)
            }
            if self.edgePhase == .waitingForSnapshot {
              if self.edgePresentation != nil {
                self.recordEdgeTest("capture-after-release")
              }
              self.dispatchEdgeBack(in: webView)
            }
          }
          #if DEBUG && targetEnvironment(simulator)
            // Explicit simulator-only fault injection; no identifiers, images or telemetry are logged.
            if let value = ProcessInfo.processInfo.environment["FLOAT_VIEW_EDGE_SNAPSHOT_DELAY_MS"],
               let milliseconds = Double(value), milliseconds > 0, milliseconds <= 1000
            {
              DispatchQueue.main.asyncAfter(deadline: .now() + milliseconds / 1000, execute: accept)
              return
            }
          #endif
          accept()
        }
      case .changed:
        edgePresentation?.track(edgeOffset)
      case .ended:
        guard edgePhase == .dragging else { return }
        let velocity = gesture.velocity(in: webView)
        let distance = max(64, webView.bounds.width * 0.2)
        let completed = translation.x > abs(translation.y) && velocity.x >= 0 &&
          (translation.x >= distance || (translation.x >= 24 && velocity.x >= 600))
        guard completed else {
          settleEdgePresentation(completed: false)
          return
        }
        edgePhase = .waitingForSnapshot
        guard snapshotPending else { dispatchEdgeBack(in: webView); return }
        let generation = edgeGeneration
        let deadline = DispatchWorkItem { [weak self, weak webView] in
          guard let self, let webView, self.edgeGeneration == generation,
                self.edgePhase == .waitingForSnapshot else { return }
          self.dispatchEdgeBack(in: webView)
        }
        captureDeadline = deadline
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.15, execute: deadline)
      case .cancelled, .failed:
        settleEdgePresentation(completed: false)
      default:
        break
      }
    }

    private func dispatchEdgeBack(in webView: WKWebView) {
      captureDeadline?.cancel()
      captureDeadline = nil
      edgePhase = .navigating
      let generation = edgeGeneration
      let interactive = edgePresentation != nil
      if !interactive {
        recordEdgeTest("capture-fallback")
      }
      // The callback distinguishes accepted/restored navigation from consumed save or dismissal guards.
      let script = """
      return await new Promise(resolve => {
        let settled = false;
        const finish = accepted => {
          if (settled) return;
          settled = true;
          clearTimeout(timer);
          resolve(accepted === true);
        };
        const timer = setTimeout(() => finish(false), 10000);
        const event = new CustomEvent('floatview:back', {
          cancelable: true, detail: {source: 'edge', interactive, complete: finish}
        });
        if (window.dispatchEvent(event)) finish(false);
      });
      """
      webView
        .callAsyncJavaScript(script, arguments: ["interactive": interactive], in: nil,
                             in: .page)
        { [weak self] result in
          guard let self, self.edgeGeneration == generation else { return }
          let accepted: Bool
          switch result {
          case let .success(value): accepted = value as? Bool == true
          case .failure: accepted = false
          }
          self.settleEdgePresentation(completed: accepted)
        }
    }

    private func settleEdgePresentation(completed: Bool) {
      captureDeadline?.cancel()
      captureDeadline = nil
      edgeGeneration += 1
      guard let presentation = edgePresentation else {
        edgePhase = .idle
        return
      }
      edgePhase = .settling
      presentation.settle(completed: completed) { [weak self] in
        guard let self, self.edgePresentation === presentation else { return }
        self.edgePresentation = nil
        self.edgePhase = .idle
      }
    }

    @objc private func resetEdgePresentation() {
      edgeGeneration += 1
      captureDeadline?.cancel()
      captureDeadline = nil
      edgePresentation?.remove()
      edgePresentation = nil
      edgePhase = .idle
    }

    private func recordEdgeTest(_ outcome: String) {
      #if DEBUG && targetEnvironment(simulator)
        if ProcessInfo.processInfo.environment["FLOAT_VIEW_EDGE_SNAPSHOT_DELAY_MS"] != nil {
          print("[FloatView edge test] \(outcome)")
        }
      #endif
    }

    func stopEdgeBack() {
      resetEdgePresentation()
      NotificationCenter.default.removeObserver(self)
      if let gesture = edgeBack {
        gesture.view?.removeGestureRecognizer(gesture)
      }
      edgeBack = nil
    }

    func webView(_: WKWebView, didStartProvisionalNavigation _: WKNavigation!) {
      edgeBack?.isEnabled = false
      resetEdgePresentation()
    }

    func webView(_: WKWebView, decidePolicyFor action: WKNavigationAction,
                 decisionHandler: @escaping (WKNavigationActionPolicy) -> Void)
    {
      // A new-window request must reach WKUIDelegate. That delegate opens
      // eligible links externally and always returns nil, never another WebView.
      if action.targetFrame == nil {
        decisionHandler(.allow)
        return
      }
      switch navigationDecision(for: action) {
      case .allowBackend:
        decisionHandler(.allow)
      case let .openBrowser(url):
        decisionHandler(.cancel)
        UIApplication.shared.open(url)
      case .cancel:
        decisionHandler(.cancel)
      }
    }

    func webView(_: WKWebView, decidePolicyFor response: WKNavigationResponse,
                 decisionHandler: @escaping (WKNavigationResponsePolicy) -> Void)
    {
      decisionHandler(response.isForMainFrame && isBackendPage(response.response.url, backendURL: loadedURL)
        ? .allow : .cancel)
    }

    func webView(_: WKWebView, createWebViewWith _: WKWebViewConfiguration,
                 for action: WKNavigationAction, windowFeatures _: WKWindowFeatures) -> WKWebView?
    {
      if case let .openBrowser(url) = navigationDecision(for: action) {
        UIApplication.shared.open(url)
      }
      return nil
    }

    private func navigationDecision(for action: WKNavigationAction) -> WebViewNavigationDecision {
      webViewNavigationDecision(
        action.request.url, backendURL: loadedURL,
        isUserLink: action.navigationType == .linkActivated,
        isMainFrame: action.targetFrame?.isMainFrame == true,
        opensNewWindow: action.targetFrame == nil
      )
    }

    func webView(_: WKWebView, didFailProvisionalNavigation _: WKNavigation!, withError error: Error) {
      guard (error as NSError).code != NSURLErrorCancelled else { return }
      edgeBack?.isEnabled = false
      resetEdgePresentation()
      onLoadFailure()
    }

    func webView(_: WKWebView, didFail _: WKNavigation!, withError error: Error) {
      guard (error as NSError).code != NSURLErrorCancelled else { return }
      edgeBack?.isEnabled = false
      resetEdgePresentation()
      onLoadFailure()
    }

    func load(_ url: URL, in webView: WKWebView) {
      guard loadedURL != url else { return }
      edgeBack?.isEnabled = false
      resetEdgePresentation()
      loadedURL = url
      #if DEBUG && targetEnvironment(simulator)
        if ProcessInfo.processInfo.environment["FLOAT_VIEW_LAYOUT_CAPTURE"] == "1" {
          var captureURL = URLComponents(url: url, resolvingAgainstBaseURL: false)!
          captureURL.fragment = "/sessions"
          webView.load(URLRequest(url: captureURL.url!))
          return
        }
      #endif
      webView.load(URLRequest(url: url))
    }

    func webViewWebContentProcessDidTerminate(_: WKWebView) {
      edgeBack?.isEnabled = false
      resetEdgePresentation()
      #if DEBUG
        print("[FloatView WebView] web content process terminated; recovering frontend")
      #endif
      DispatchQueue.main.async { [onContentProcessTerminated] in
        onContentProcessTerminated()
      }
    }

    func userContentController(_: WKUserContentController, didReceive message: WKScriptMessage) {
      if message.name == "floatViewBackNavigation" {
        guard message.frameInfo.isMainFrame,
              isBackendPage(message.frameInfo.request.url, backendURL: loadedURL),
              let available = message.body as? Bool
        else { return }
        edgeBack?.isEnabled = available
        return
      }
      #if DEBUG
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
      #endif
    }
  }
}

/// Supplies closed OS layout facts to the shared document, without exposing native commands.
private final class HostLayoutWebView: WKWebView {
  private var documentReady = false
  private var lastLayoutJSON: String?

  override func updateConstraints() {
    super.updateConstraints()
    publishLayout()
  }

  override func layoutSubviews() {
    super.layoutSubviews()
    publishLayout()
  }

  override func safeAreaInsetsDidChange() {
    super.safeAreaInsetsDidChange()
    setNeedsUpdateConstraints()
    publishLayout()
  }

  func documentLoaded() {
    documentReady = true
    lastLayoutJSON = nil
    setNeedsUpdateConstraints()
    publishLayout()
    #if DEBUG && targetEnvironment(simulator)
      if ProcessInfo.processInfo.environment["FLOAT_VIEW_LAYOUT_CAPTURE"] == "1" {
        captureLayout()
      }
    #endif
  }

  private func publishLayout() {
    guard documentReady, window != nil, bounds.width > 0, bounds.height > 0 else { return }
    var edge: Any = NSNull()
    var occlusions: [[String: CGFloat]] = []
    #if compiler(>=6.4)
      if #available(iOS 27.1, *) {
        let rtl = effectiveUserInterfaceLayoutDirection == .rightToLeft
        switch traitCollection.verticalBarEdge {
        case .leading: edge = rtl ? "right" : "left"
        case .trailing: edge = rtl ? "left" : "right"
        default: break
        }
        // Reading regions during updateConstraints participates in UIKit's region observation.
        let regions = reservedRegions(kind: .occlusion).filter(\.isActive)
        if regions.count > 16 {
          edge = NSNull()
        }
        occlusions = regions.prefix(16).map {
          ["x": $0.frame.minX - bounds.minX, "y": $0.frame.minY - bounds.minY,
           "width": $0.frame.width, "height": $0.frame.height]
        }
      }
    #endif
    let value: [String: Any] = [
      "version": 1, "edge": edge,
      "bounds": ["width": bounds.width, "height": bounds.height],
      "safeArea": ["top": safeAreaInsets.top, "right": safeAreaInsets.right,
                   "bottom": safeAreaInsets.bottom, "left": safeAreaInsets.left],
      "occlusions": occlusions,
    ]
    guard let data = try? JSONSerialization.data(withJSONObject: value, options: .sortedKeys),
          data.count <= 16 * 1024, let json = String(data: data, encoding: .utf8), json != lastLayoutJSON
    else { return }
    lastLayoutJSON = json
    evaluateJavaScript("window.floatViewHostLayout = \(json); window.dispatchEvent(new Event('floatview:layout'));")
  }

  #if DEBUG && targetEnvironment(simulator)
    /// Explicit opt-in diagnostic; the document never gets a native command channel.
    private var captureFinished = false

    func captureLayout(attempt: Int = 0) {
      guard !captureFinished, attempt < 45 else { return }
      DispatchQueue.main.asyncAfter(deadline: .now() + 1) { [weak self] in
        guard let self, !self.captureFinished else { return }
        let script = """
        (() => {
          const agree = Array.from(document.querySelectorAll('button')).find(b => b.textContent.trim() === 'I agree');
          if (agree) { agree.click(); return null; }
          if (!document.querySelector('[data-app-navigation]')) return null;
          const rect = el => { const r = el.getBoundingClientRect(); return {x:r.x,y:r.y,width:r.width,height:r.height}; };
          const probe = document.createElement('div');
          probe.style.cssText = 'position:fixed;pointer-events:none;padding:env(safe-area-inset-top) env(safe-area-inset-right) env(safe-area-inset-bottom) env(safe-area-inset-left)';
          document.body.append(probe);
          const style = getComputedStyle(probe);
          const safeArea = Object.fromEntries(['top','right','bottom','left'].map(s => [s, parseFloat(style.getPropertyValue('padding-'+s))]));
          probe.remove();
          return {viewport:{width:innerWidth,height:innerHeight,scale:devicePixelRatio},safeArea,
              hostLayout:window.floatViewHostLayout ?? null,controlsEdge:document.documentElement.dataset.controlsEdge ?? null,
            visualViewport:visualViewport ? {width:visualViewport.width,height:visualViewport.height,offsetLeft:visualViewport.offsetLeft,offsetTop:visualViewport.offsetTop,scale:visualViewport.scale} : null,
            navigation:Array.from(document.querySelectorAll('[data-app-navigation], [data-app-navigation] a')).map(rect),
            header:Array.from(document.querySelectorAll('header, header a, header button')).slice(0,20).map(rect)};
        })()
        """
        self.evaluateJavaScript(script) { [weak self] value, error in
          guard let self else { return }
          guard error == nil, let web = value as? [String: Any], let window = self.window else {
            self.captureLayout(attempt: attempt + 1)
            return
          }
          func rect(_ value: CGRect) -> [String: CGFloat] {
            ["x": value.minX, "y": value.minY, "width": value.width, "height": value.height]
          }
          func insets(_ value: UIEdgeInsets) -> [String: CGFloat] {
            ["top": value.top, "right": value.right, "bottom": value.bottom, "left": value.left]
          }
          var native: [String: Any] = [
            "window": rect(window.bounds), "webView": rect(self.bounds),
            "webViewInWindow": rect(self.convert(self.bounds, to: window)),
            "windowSafeArea": insets(window.safeAreaInsets), "webViewSafeArea": insets(self.safeAreaInsets),
            "adjustedContentInset": insets(self.scrollView.adjustedContentInset),
            "statusBar": rect(window.windowScene?.statusBarManager?.statusBarFrame ?? .zero),
          ]
          #if compiler(>=6.4)
            if #available(iOS 27.1, *) {
              native["verticalBarEdge"] = String(describing: self.traitCollection.verticalBarEdge)
              native["occlusions"] = self.reservedRegions(kind: .occlusion).prefix(16).map { rect($0.frame) }
              native["divisions"] = self.reservedRegions(kind: .division).prefix(16).map { rect($0.frame) }
            }
          #endif
          let report: [String: Any] = ["schema": 1, "web": web, "native": native]
          do {
            let data = try JSONSerialization.data(withJSONObject: report, options: [.sortedKeys, .prettyPrinted])
            guard data.count <= 64 * 1024 else { return }
            let documents = try FileManager.default.url(for: .documentDirectory, in: .userDomainMask,
                                                        appropriateFor: nil, create: true)
            try data.write(to: documents.appendingPathComponent("layout-capture.json"), options: .atomic)
            self.captureFinished = true
            print("[FloatView layout] capture ready")
          } catch {
            print("[FloatView layout] capture failed")
          }
        }
      }
    }
  #endif
}
