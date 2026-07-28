import Foundation
import XCTest

final class T1VescTestDeviceTests: XCTestCase {
  func testProductionAdapterRoundTripsExactBytes() {
    let completed = expectation(description: "production BLE transport completed")
    let driver = T1BluetoothDriver(completed: completed)

    driver.start()
    wait(for: [completed], timeout: 30)

    XCTAssertNil(driver.failure)
    XCTAssertEqual(driver.receivedNotification, T1BluetoothDriver.notificationBytes)
    XCTAssertTrue(driver.sawReady)
    XCTAssertTrue(driver.sawDisconnected)
    withExtendedLifetime(driver) {}
  }
}

private final class T1BluetoothDriver: @unchecked Sendable {
  static let writeBytes = Data([0x10, 0x20, 0x30, 0x40, 0x50, 0x60])
  static let notificationBytes = Data([0xde, 0xad, 0xbe, 0xef, 0x01, 0x02])

  private let lock = NSLock()
  private let completed: XCTestExpectation
  private lazy var adapter = CoreBluetoothAdapter(
    eventHandler: { [weak self] event in self?.receive(event) },
    diagnosticHandler: { _ in }
  )
  private var selectedDevice: DeviceID?
  private var terminal = false
  private(set) var failure: String?
  private(set) var receivedNotification: Data?
  private(set) var sawReady = false
  private(set) var sawDisconnected = false

  init(completed: XCTestExpectation) {
    self.completed = completed
  }

  func start() {
    let state = adapter.currentState()
    if state.radio == .poweredOn {
      adapter.startDiscovery()
    }
  }

  private func receive(_ event: BluetoothEvent) {
    lock.lock()
    defer { lock.unlock() }
    guard !terminal else { return }

    switch event {
    case let .stateChanged(state):
      if state.radio == .poweredOn {
        adapter.startDiscovery()
      } else if state.radio == .unsupported || state.radio == .poweredOff {
        finish(failure: "bluetooth_\(state.radio.rawValue)")
      }

    case let .deviceFound(deviceID, name, _):
      guard selectedDevice == nil, name == "FloatView Test VESC" else { return }
      selectedDevice = deviceID
      adapter.connect(deviceID)

    case let .connectionChanged(deviceID, state):
      guard deviceID == selectedDevice else { return }
      switch state {
      case .ready:
        sawReady = true
        adapter.write(Self.writeBytes, to: deviceID)
      case .disconnected:
        sawDisconnected = true
        if receivedNotification == Self.notificationBytes {
          finish()
        }
      case .failed:
        finish(failure: "connection_failed")
      default:
        break
      }

    case let .notification(deviceID, bytes):
      guard deviceID == selectedDevice else { return }
      receivedNotification = bytes
      if bytes == Self.notificationBytes {
        adapter.disconnect(deviceID)
      } else {
        finish(failure: "notification_bytes_\(bytes.hex)")
      }

    case let .scanFailed(reason):
      finish(failure: "scan_failed_\(reason)")
    }
  }

  private func finish(failure: String? = nil) {
    guard !terminal else { return }
    terminal = true
    self.failure = failure
    completed.fulfill()
  }
}

private extension Data {
  var hex: String {
    map { String(format: "%02x", $0) }.joined()
  }
}
