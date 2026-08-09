import AppKit
import Foundation

@main
enum FloatViewMac {
  static func main() throws {
    let environment = ProcessInfo.processInfo.environment
    let htmlURL = environment["FLOAT_VIEW_FRONTEND_HTML"].map(URL.init(fileURLWithPath:))
      ?? Bundle.module.url(forResource: "index", withExtension: "html", subdirectory: "Resources")
    guard let htmlURL else { throw CocoaError(.fileNoSuchFile) }
    let html = try Data(contentsOf: htmlURL)
    if CommandLine.arguments.contains("--browser-host") {
      let application = NSApplication.shared
      application.setActivationPolicy(.accessory)
      let runtime = MacBackendRuntime(html: html)
      let server = runtime.makeServer(exportPresenter: MacExportPresenter.schedule)
      try server.start()
      try printReadiness(server: server, runtime: runtime)
      application.run()
      server.stop()
      runtime.shutdown()
      return
    }
    if CommandLine.arguments.contains("--test-host") {
      let runtime = MacBackendRuntime(html: html)
      let server = runtime.makeServer()
      try server.start()
      try printReadiness(server: server, runtime: runtime)
      RunLoop.current.run()
      return
    }

    let application = NSApplication.shared
    installApplicationIcon(on: application)
    let delegate = MacAppDelegate(html: html)
    application.delegate = delegate
    application.setActivationPolicy(.regular)
    installMainMenu(on: application)
    application.run()
  }
}
