import Foundation

private let expectedName = "FloatView Test VESC"
private let primaryCANID: UInt8 = 42

private func emit(_ kind: String, _ fields: [String: Any] = [:]) {
  var event = fields
  event["kind"] = kind
  event["schemaVersion"] = 1
  let data = try! JSONSerialization.data(withJSONObject: event, options: [.sortedKeys])
  print(String(decoding: data, as: UTF8.self))
  fflush(stdout)
}

private final class T2ProductionDriver: @unchecked Sendable {
  private lazy var adapter = CoreBluetoothAdapter(
    eventHandler: { [weak self] event in self?.receive(event) },
    diagnosticHandler: { diagnostic in emit("diagnostic", ["value": diagnostic]) }
  )
  private let protocolCodec = VescProtocolService()
  private var selectedDevice: DeviceID?
  private var completed = false

  func run() -> Never {
    _ = adapter
    DispatchQueue.global().asyncAfter(deadline: .now() + 30) { [weak self] in
      guard let self, !self.completed else { return }
      emit("failed", ["code": "timeout"])
      exit(1)
    }
    RunLoop.main.run()
    fatalError("main run loop returned")
  }

  private func write(_ data: Data) {
    guard let selectedDevice else { return }
    adapter.write(data, to: selectedDevice)
  }

  private func fail(_ code: String, _ detail: String = "") -> Never {
    emit("failed", ["code": code, "detail": detail])
    exit(1)
  }

  private func receive(_ event: BluetoothEvent) {
    do {
      switch event {
      case let .stateChanged(state):
        guard state.radio == .poweredOn else { return }
        adapter.startDiscovery()

      case let .deviceFound(deviceID, name, _):
        guard selectedDevice == nil, name == expectedName else { return }
        selectedDevice = deviceID
        emit("discovered", ["name": name ?? ""])
        adapter.connect(deviceID)

      case let .connectionChanged(deviceID, state):
        guard deviceID == selectedDevice else { return }
        if state == .ready {
          write(try protocolCodec.firmwareRequest())
        } else if state == .disconnected, completed {
          emit("passed", ["disconnected": true])
          exit(0)
        } else if state == .failed {
          fail("connectionFailed")
        }

      case let .notification(deviceID, bytes):
        guard deviceID == selectedDevice else { return }
        for message in try protocolCodec.receive(bytes) {
          try handle(message)
        }

      case let .scanFailed(reason):
        fail("scanFailed", reason)
      }
    } catch {
      fail("protocolFailure", String(describing: error))
    }
  }

  private func handle(_ message: VescProtocolMessage) throws {
    switch message {
    case let .firmware(value) where value.canID == nil:
      guard value.major == 6, value.minor == 5,
            value.hardwareName == "VESC Express T", value.hardwareType == 2
      else { fail("transportIdentity") }
      write(try protocolCodec.canPingRequest())

    case let .canDevices(ids):
      guard ids == [primaryCANID] else { fail("canTopology", String(describing: ids)) }
      write(try protocolCodec.firmwareRequest(canID: primaryCANID))

    case let .firmware(value):
      guard value.canID == primaryCANID, value.major == 6, value.minor == 6,
            value.hardwareName == "THOR300", value.hardwareType == 0
      else { fail("primaryIdentity") }
      write(try protocolCodec.refloatInfoRequest(canID: primaryCANID))

    case let .refloatInfo(version):
      guard version == "1.2.3" else { fail("refloatVersion", version) }
      write(try protocolCodec.refloatTelemetryIDsRequest(canID: primaryCANID))

    case let .refloatTelemetryLayout(fields, runtimeFields):
      guard fields.count == 16, runtimeFields.count == 10 else {
        fail("refloatLayout", "\(fields.count)/\(runtimeFields.count)")
      }
      write(try protocolCodec.refloatTelemetryRequest(canID: primaryCANID))

    case let .refloatTelemetry(value):
      guard value.rideState == .idle, value.footpadState == 0 else {
        fail("refloatTelemetry")
      }
      completed = true
      emit("sequenceComplete", ["rideState": value.rideState.rawValue])
      if let selectedDevice { adapter.disconnect(selectedDevice) }
    }
  }
}

@main
private enum T2ProductionClient {
  static func main() {
    let driver = T2ProductionDriver()
    driver.run()
  }
}
