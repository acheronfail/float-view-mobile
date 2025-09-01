import CoreBluetooth
import Foundation

/// High-level manager for VESC device operations. Handles all BLE connection, command sending, and
/// response parsing.
class VescManager: NSObject, ObservableObject {

  // MARK: - Connection Status

  enum ConnectionStatus {
    case connecting
    case connected
    case ready
    case disconnected
  }

  // MARK: - Properties

  private let vescBle = VescBle()
  private var vescCommands: VescCommands?

  // MARK: - Initialization

  override init() {
    super.init()
    vescBle.delegate = self
  }

  // MARK: - Public Methods

  /** Start scanning for VESC devices */
  func startScan(timeoutMs: Double = 3000.0) {
    vescBle.startScan(serviceUUIDs: [VescBle.vescServiceUUID], timeoutMs: timeoutMs)
  }

  /** Connect to a VESC device and get firmware version */
  func connect(address: String) {
    if let peripheral = vescBle.connect(to: address) {
      vescCommands = VescCommands(peripheral: peripheral)
    }
  }

  /** Disconnect from the current device */
  func disconnect() {
    vescCommands?.cleanup()
    vescBle.disconnect()
    vescCommands = nil

    LocalServer.shared.webSocket.broadcastMessage(
      wsMessage(topic: "connect", event: "disconnect", data: ["reason": "user requested"])
    )
  }

  /** Check if permissions are granted */
  func hasPermissions() -> Bool {
    return vescBle.hasPermissions()
  }

  /** Request permissions if needed */
  func requestPermissions() -> Bool {
    return vescBle.requestPermissionsIfNeeded()
  }

  func command(_ command: Int) {
    // FIXME: conflicting callbacks here - just have a single place where we pass them back
    // TODO: error if no vescCommand
    vescCommands?.sendCommand(
      command: command,
      data: []
    ) { result in
      switch result {
      case .response(let command, let data):
        switch command {
        case VescCommands.COMM_FW_VERSION:
          let parser = VescBuffer(data: data)

          let major = parser.readUInt8()
          let minor = parser.readUInt8()
          var name = parser.readString()
          if name.isEmpty { name = "Unknown" }

          var uuid: String?
          if parser.size - parser.index >= 12 {
            var uuidBuilder = ""
            for _ in 0..<12 {
              let v = parser.readUInt8()
              if v < 10 {
                uuidBuilder += "0\(String(v, radix: 16))"
              } else {
                uuidBuilder += String(v, radix: 16).uppercased()
              }
            }
            uuid = uuidBuilder
          }

          var isPaired: Bool?
          if parser.size - parser.index >= 1 {
            isPaired = parser.readUInt8() != 0
          }

          var betaVersion: Int?
          if parser.size - parser.index >= 1 {
            betaVersion = Int(parser.readUInt8())
          }

          var hwType: Int?
          if parser.size - parser.index >= 1 {
            hwType = Int(parser.readUInt8())
          }

          LocalServer.shared.webSocket.broadcastMessage(
            wsMessage(
              topic: "command",
              event: "result",
              data: [
                "command": command,
                "data": [
                  "major": major,
                  "minor": minor,
                  "name": name,
                  "uuid": uuid as Any,
                  "isPaired": isPaired as Any,
                  "betaVersion": betaVersion as Any,
                  "hwType": hwType as Any,
                ],
              ])
          )

        default:
          print(
            "FloatView_FloatViewApp: Unhandled VESC response - Command: \(command), Data: \(data.map { String(format: "%02X", $0) }.joined(separator: " "))"
          )
        }
      case .error(let command, let message):
        LocalServer.shared.webSocket.broadcastMessage(
          wsMessage(
            topic: "command", event: "error", data: ["command": command, "message": message]
          )
        )
      }
    }
  }
}

// MARK: - VescBle Delegate

extension VescManager: VescBleDelegate {
  func vescBle(_ vescBle: VescBle, didReceiveData data: Data, from characteristic: CBCharacteristic)
  {
    vescCommands?.onCharacteristicChanged(characteristic: characteristic, value: data)
  }
}
