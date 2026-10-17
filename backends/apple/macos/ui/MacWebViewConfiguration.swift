import WebKit

@MainActor
enum MacWebViewConfiguration {
  static func make(
    version: String = Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String ?? "development"
  ) -> WKWebViewConfiguration {
    let configuration = WKWebViewConfiguration()
    configuration.applicationNameForUserAgent = "FloatView/\(version)"
    return configuration
  }
}
