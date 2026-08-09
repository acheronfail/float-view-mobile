// Capability status: implemented — one-runtime continuity passes simulator and physical gates.
// Concrete owner: shared FloatViewAppleRuntime and FloatViewCoreRuntime.

import Foundation

final class FloatViewRuntimeBridge: @unchecked Sendable {
  private let runtime: FloatViewAppleRuntime

  init(readiness: @escaping FloatViewRuntimeBridgeAdapter.RuntimeReadyHandler) throws {
    let presentation = FloatViewDocuments.makePresenter()
    let host = try FloatViewRuntimeHost.standard(
      backgroundLogging: FloatViewBackgroundExecution.setLoggingActive,
      presentation: presentation
    )
    #if targetEnvironment(simulator)
      let softwareTransport = true
    #else
      let softwareTransport = false
    #endif
    let requestedPort = ProcessInfo.processInfo.environment["FLOAT_VIEW_CONFORMANCE_PORT"]
      .flatMap(UInt16.init) ?? 0
    guard let index = Bundle.main.url(forResource: "index", withExtension: "html") else {
      throw CocoaError(.fileNoSuchFile)
    }
    runtime = try FloatViewAppleRuntime(
      requestedPort: requestedPort,
      softwareTransport: softwareTransport,
      storageRoot: host.durableStorageRoot(),
      frontendIndex: Data(contentsOf: index),
      bluetoothFactory: { handler in
        FloatViewBluetooth.make(
          eventHandler: handler,
          diagnosticHandler: { diagnostic in
            #if DEBUG
              NSLog("[FloatViewCore] adapter %@", diagnostic)
            #endif
          }
        ).port
      },
      location: FloatViewLocation.make(),
      platformHost: host,
      retainedPresentation: presentation,
      backgroundEligibility: { FloatViewBackgroundExecution.effectiveEligibility },
      diagnosticHandler: { stage, code in
        #if DEBUG
          NSLog("[FloatViewApple] stage=%@ code=%@", stage, code)
        #endif
      },
      runtimeReadyHandler: readiness
    )
  }

  func updateLifecycle(foreground: Bool) {
    runtime.updateLifecycle(foreground: foreground)
  }

  func shutdown() {
    runtime.shutdown()
  }
}

func authenticatedLaunchURL(endpoint: URL, token: String) -> URL? {
  guard var components = URLComponents(url: endpoint, resolvingAgainstBaseURL: false) else {
    return nil
  }
  var items = components.queryItems ?? []
  items.removeAll { $0.name == "token" }
  items.append(URLQueryItem(name: "token", value: token))
  components.queryItems = items
  return components.url
}
