import FloatViewBackend
import Foundation

struct MacLaunchConfiguration {
  let frontendHTML: URL?
  let storageRoot: URL?
  let port: UInt16

  init(environment: [String: String] = ProcessInfo.processInfo.environment) {
    #if DEBUG
      frontendHTML = environment["FLOAT_VIEW_FRONTEND_HTML"].map(URL.init(fileURLWithPath:))
      storageRoot = environment["FLOAT_VIEW_LOG_ROOT"].map {
        URL(fileURLWithPath: $0, isDirectory: true)
      }
      port = environment["FLOAT_VIEW_CONFORMANCE_PORT"].flatMap(UInt16.init)
        ?? PlatformLoopbackPort.macOS
    #else
      frontendHTML = nil
      storageRoot = nil
      port = PlatformLoopbackPort.macOS
    #endif
  }
}
