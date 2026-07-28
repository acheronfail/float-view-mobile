import Foundation

private let expectedName = "FloatView Test VESC"
private let writeBytes = Data([0x10, 0x20, 0x30, 0x40, 0x50, 0x60])
private let expectedNotification = Data([0xde, 0xad, 0xbe, 0xef, 0x01, 0x02])

private func hex(_ data: Data) -> String {
  data.map { String(format: "%02x", $0) }.joined()
}

private func emit(_ kind: String, _ fields: [String: Any] = [:]) {
  var event = fields
  event["kind"] = kind
  event["schemaVersion"] = 1
  let data = try! JSONSerialization.data(withJSONObject: event, options: [.sortedKeys])
  print(String(decoding: data, as: UTF8.self))
  fflush(stdout)
}

private final class ProductionAdapterDriver: @unchecked Sendable {
  private lazy var adapter = CoreBluetoothAdapter(
    eventHandler: { [weak self] event in self?.receive(event) },
    diagnosticHandler: { diagnostic in emit("diagnostic", ["value": diagnostic]) }
  )
  private var selectedDevice: DeviceID?
  private var receivedExpectedNotification = false
  private var finished = false

  func run() -> Never {
    _ = adapter
    DispatchQueue.global().asyncAfter(deadline: .now() + 25) { [weak self] in
      guard let self, !self.finished else { return }
      emit("failed", ["code": "timeout"])
      exit(1)
    }
    RunLoop.main.run()
    fatalError("main run loop returned")
  }

  private func receive(_ event: BluetoothEvent) {
    switch event {
    case let .stateChanged(state):
      guard state.radio == .poweredOn else { return }
      emit("radioReady")
      adapter.startDiscovery()

    case let .deviceFound(deviceID, name, rssi):
      guard selectedDevice == nil, name == expectedName else { return }
      selectedDevice = deviceID
      emit("discovered", ["name": name ?? "", "rssi": rssi ?? 0])
      adapter.connect(deviceID)

    case let .connectionChanged(deviceID, state):
      guard deviceID == selectedDevice else { return }
      emit("connectionState", ["state": state.rawValue])
      if state == .ready {
        adapter.write(writeBytes, to: deviceID)
        emit("writeIssued", ["hex": hex(writeBytes)])
      } else if state == .disconnected, receivedExpectedNotification {
        finished = true
        emit("passed", ["disconnected": true])
        exit(0)
      } else if state == .failed {
        emit("failed", ["code": "connectionFailed"])
        exit(1)
      }

    case let .notification(deviceID, bytes):
      guard deviceID == selectedDevice, bytes == expectedNotification else {
        emit("failed", [
          "code": "notificationBytes",
          "actualHex": hex(bytes),
        ])
        exit(1)
      }
      receivedExpectedNotification = true
      emit("notificationReceived", ["hex": hex(bytes)])

    case let .scanFailed(reason):
      emit("failed", ["code": "scanFailed", "reason": reason])
      exit(1)
    }
  }
}

@main
private enum T1ProductionClient {
  static func main() {
    let driver = ProductionAdapterDriver()
    driver.run()
  }
}
