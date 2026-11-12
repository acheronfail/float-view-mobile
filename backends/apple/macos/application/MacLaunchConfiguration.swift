import FloatViewBackend
import Foundation

struct MacLaunchConfiguration {
  let frontendHTML: URL?
  let storageRoot: URL?

  init(environment: [String: String] = ProcessInfo.processInfo.environment) {
    #if DEBUG
      frontendHTML = environment["FLOAT_VIEW_FRONTEND_HTML"].map(URL.init(fileURLWithPath:))
      storageRoot = environment["FLOAT_VIEW_LOG_ROOT"].map {
        URL(fileURLWithPath: $0, isDirectory: true)
      }
    #else
      frontendHTML = nil
      storageRoot = nil
    #endif
  }
}
