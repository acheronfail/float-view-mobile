// Capability status: incomplete — physical reconnect evidence remains.
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
