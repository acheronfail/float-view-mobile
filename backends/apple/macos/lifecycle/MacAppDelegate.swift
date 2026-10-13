import AppKit
import FloatViewBackend
import Foundation
import WebKit

final class MacAppDelegate: NSObject, NSApplicationDelegate, NSWindowDelegate,
  WKNavigationDelegate
{
  private let runtime: MacBackendRuntime
  private var server: LoopbackServer?
  private var generation = 0
  private var window: NSWindow?
  private var webView: WKWebView?
  private var loadingView: MacLoadingView?

  init(html: Data) {
    let runtime = MacBackendRuntime(html: html)
    self.runtime = runtime
  }

  func applicationDidFinishLaunching(_: Notification) {
    showFrontend()
  }

  func applicationDidBecomeActive(_: Notification) {
    runtime.setForeground(true)
  }

  func applicationDidResignActive(_: Notification) {
    runtime.setForeground(false)
  }

  func applicationShouldHandleReopen(
    _: NSApplication,
    hasVisibleWindows flag: Bool
  ) -> Bool {
    if !flag {
      showFrontend()
    }
    return true
  }

  func applicationWillTerminate(_: Notification) {
    server?.stop()
    runtime.shutdown()
  }

  func applicationShouldTerminateAfterLastWindowClosed(_: NSApplication) -> Bool {
    false
  }

  func windowWillClose(_: Notification) {
    webView?.navigationDelegate = nil
    webView?.stopLoading()
    server?.stop()
    server = nil
    trace("[FloatViewState] frontend state=stopped reason=window_closed")
    DispatchQueue.main.async { [weak self] in
      self?.webView = nil
      self?.loadingView = nil
      self?.window = nil
    }
  }

  func webViewWebContentProcessDidTerminate(_: WKWebView) {
    guard window != nil else { return }
    closeFrontendWindow()
    showFrontend()
  }

  func webView(_ webView: WKWebView, didFinish _: WKNavigation!) {
    guard webView === self.webView else { return }
    webView.isHidden = false
    loadingView?.isHidden = true
  }

  func webView(_ webView: WKWebView, didFailProvisionalNavigation _: WKNavigation!, withError _: Error) {
    guard webView === self.webView else { return }
    loadingView?.showFailure()
  }

  func webView(_ webView: WKWebView, didFail _: WKNavigation!, withError _: Error) {
    guard webView === self.webView else { return }
    loadingView?.showFailure()
  }

  private func showFrontend() {
    guard window == nil else {
      window?.makeKeyAndOrderFront(nil)
      return
    }
    do {
      let server = runtime.makeServer(exportPresenter: MacExportPresenter.schedule) { preference in
        DispatchQueue.main.async {
          MacTheme.apply(preference, to: NSApplication.shared)
        }
      }
      try server.start()
      self.server = server
      generation += 1
      let webView = WKWebView(frame: .zero)
      webView.underPageBackgroundColor = MacTheme.canvasColor
      webView.navigationDelegate = self
      webView.allowsMagnification = false
      webView.load(URLRequest(url: URL(string: server.origin)!))
      let window = NSWindow(
        contentRect: NSRect(x: 0, y: 0, width: 430, height: 820),
        styleMask: [.titled, .closable, .miniaturizable, .resizable],
        backing: .buffered,
        defer: false
      )
      window.backgroundColor = MacTheme.canvasColor
      window.title = "Float View"
      window.isReleasedWhenClosed = false
      let content = NSView(frame: NSRect(x: 0, y: 0, width: 430, height: 820))
      webView.frame = content.bounds
      webView.autoresizingMask = [.width, .height]
      webView.isHidden = true
      content.addSubview(webView)
      let loading = MacLoadingView(frame: content.bounds)
      loading.autoresizingMask = [.width, .height]
      content.addSubview(loading)
      window.contentView = content
      loadingView = loading
      window.delegate = self
      window.center()
      window.makeKeyAndOrderFront(nil)
      NSApplication.shared.activate(ignoringOtherApps: true)
      self.webView = webView
      self.window = window
      trace("[FloatViewState] frontend state=ready generation=\(generation)")
    } catch {
      let message = loopbackStartupFailure(
        port: PlatformLoopbackPort.macOS,
        error: error
      )
      trace("[FloatViewState] frontend state=failed \(message)")
    }
  }

  private func closeFrontendWindow() {
    window?.delegate = nil
    window?.orderOut(nil)
    webView?.navigationDelegate = nil
    webView?.stopLoading()
    webView = nil
    loadingView = nil
    window = nil
    server?.stop()
    server = nil
  }
}
