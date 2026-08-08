// Capability status: incomplete — physical/background evidence remains.
// Concrete OS owner: CoreBluetoothAdapter.

import Foundation

enum FloatViewBluetooth {
  struct Selection {
    let port: any BluetoothPort
    let usesRustSoftwareTransport: Bool
  }

  static func make(
    eventHandler: @escaping @Sendable (BluetoothEvent) -> Void,
    diagnosticHandler: @escaping @Sendable (String) -> Void
  ) -> Selection {
    #if targetEnvironment(simulator)
      Selection(port: InactiveBluetoothPort(), usesRustSoftwareTransport: true)
    #else
      Selection(
        port: CoreBluetoothAdapter(
          eventHandler: eventHandler,
          diagnosticHandler: diagnosticHandler,
          restorationIdentifier: "fail.acheron.float-view.rust-bluetooth-central"
        ),
        usesRustSoftwareTransport: false
      )
    #endif
  }
}
