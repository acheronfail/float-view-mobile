import AppKit
import Foundation

@main
enum FloatViewMac {
  static func main() throws {
    let configuration = MacLaunchConfiguration()
    let htmlURL = configuration.frontendHTML
      ?? MacResources.url(forResource: "index", withExtension: "html")
    guard let htmlURL else { throw CocoaError(.fileNoSuchFile) }
    let html = try Data(contentsOf: htmlURL)
    #if DEBUG
      if CommandLine.arguments.contains("--browser-host") {
        let application = NSApplication.shared
        application.setActivationPolicy(.accessory)
        let runtime = try MacBackendRuntime(html: html)
        let server = try runtime.makeServer(exportPresenter: MacExportPresenter.schedule)
        try server.start()
        try printReadiness(server: server, runtime: runtime)
        application.run()
        server.stop()
        runtime.shutdown()
        return
      }
      if CommandLine.arguments.contains("--test-host") {
        let runtime = try MacBackendRuntime(html: html)
        let server = try runtime.makeServer()
        try server.start()
        try printReadiness(server: server, runtime: runtime)
        RunLoop.current.run()
        return
      }

    #endif

    let application = NSApplication.shared
    installApplicationIcon(on: application)
    let delegate = MacAppDelegate(html: html)
    application.delegate = delegate
    application.setActivationPolicy(.regular)
    installMainMenu(on: application)
    application.run()
  }
}
