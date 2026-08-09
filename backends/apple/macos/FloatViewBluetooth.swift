// Capability status: implemented — the physical 16-scenario fixture matrix passes.
// Concrete OS owner: shared CoreBluetoothAdapter.

import FloatViewBackend

enum FloatViewBluetooth {
  static func make(
    softwareTransport: Bool,
    eventHandler: @escaping @Sendable (BluetoothEvent) -> Void,
    diagnosticHandler: @escaping @Sendable (String) -> Void
  ) -> any BluetoothPort {
    if softwareTransport { return InactiveBluetoothPort() }
    return CoreBluetoothAdapter(
      eventHandler: eventHandler,
      diagnosticHandler: diagnosticHandler
    )
  }
}
