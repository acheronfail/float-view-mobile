import Foundation

public enum WebViewNavigationDecision: Equatable {
  case allowBackend
  case openBrowser(URL)
  case cancel
}

/// The embedded document is pinned to the backend page, allowing hash routes only.
public func isBackendPage(_ url: URL?, backendURL: URL?) -> Bool {
  guard let url, let backendURL,
        var destination = URLComponents(url: url, resolvingAgainstBaseURL: false),
        var backend = URLComponents(url: backendURL, resolvingAgainstBaseURL: false)
  else { return false }
  destination.fragment = nil
  backend.fragment = nil
  if destination.path.isEmpty {
    destination.path = "/"
  }
  if backend.path.isEmpty {
    backend.path = "/"
  }
  return destination == backend
}

/// All document navigation is classified before WebKit can replace the app page.
public func webViewNavigationDecision(
  _ url: URL?, backendURL: URL?, isUserLink: Bool,
  isMainFrame: Bool, opensNewWindow: Bool
) -> WebViewNavigationDecision {
  guard let url, let backendURL else { return .cancel }
  if isBackendPage(url, backendURL: backendURL) {
    return isMainFrame && !opensNewWindow ? .allowBackend : .cancel
  }
  guard isUserLink, isMainFrame || opensNewWindow,
        let scheme = url.scheme?.lowercased(), ["http", "https"].contains(scheme),
        let host = url.host?.lowercased(), !host.isEmpty,
        host != backendURL.host?.lowercased(), host != "localhost",
        !host.hasSuffix(".localhost"), !host.hasPrefix("127."),
        host != "[::1]", host != "::1",
        url.user == nil, url.password == nil
  else { return .cancel }
  return .openBrowser(url)
}
