import SwiftUI

func print(_ items: Any..., separator: String = " ", terminator: String = "\n") {
  let output = items.map { "\($0)" }.joined(separator: separator)
  NSLog("%@", output)
}

@main
struct FloatViewApp: App {

  // VESC BLE manager
  private let vescManager: VescManager
  private let vescListener: VescListenerImpl

  init() {
    // Start server synchronously right now, before any views are created
    LocalServer.shared.start()

    // Initialize VescManager and listener first
    vescManager = VescManager()
    vescListener = VescListenerImpl()

    // Now setup handlers and listeners
    setupWebSocketHandlers()
    setupVescListener()
  }

  var body: some Scene {
    WindowGroup {
      ContentView(server: LocalServer.shared).preferredColorScheme(.dark)
    }
  }

  // MARK: - WebSocket Handler Setup (similar to MainActivity.kt)

  private func setupWebSocketHandlers() {
    setupScanHandler()
    setupConnectHandler()
    setupDataHandler()
  }

  private func setupScanHandler() {
    LocalServer.shared.addHandler("scan") { [vescManager] (data: [String: Any]) in
      print("FloatViewApp: Starting BLE scan for VESC devices")

      if !vescManager.startScan() {
        LocalServer.shared.webSocket.broadcastMessage(
          wsMessage(topic: "scan", event: "error", data: ["message": "Failed to start scan"])
        )
      } else {
        LocalServer.shared.webSocket.broadcastMessage(
          wsMessage(topic: "scan", event: "status", data: ["status": "start"])
        )
      }
    }
  }

  private func setupConnectHandler() {
    LocalServer.shared.addHandler("connect") { [vescManager] (data: [String: Any]) in
      guard let address = data["address"] as? String else {
        LocalServer.shared.webSocket.broadcastMessage(
          wsMessage(topic: "connect", event: "error", data: ["message": "no address"])
        )
        return
      }

      print("FloatViewApp: Attempting to connect to \(address)")
      vescManager.connectAndGetFirmware(address: address)
    }

    LocalServer.shared.addHandler("disconnect") { [vescManager] (data: [String: Any]) in
      print("FloatViewApp: Disconnecting from device")
      vescManager.disconnect()

      LocalServer.shared.webSocket.broadcastMessage(
        wsMessage(topic: "connect", event: "disconnect", data: ["reason": "websocket requested"])
      )
    }
  }

  private func setupDataHandler() {
    LocalServer.shared.addHandler("data") { (data: [String: Any]) in
      guard let shouldLog = data["log"] as? Bool else { return }

      if !shouldLog {
        // TODO: stop logging interval
        return
      }

      let intervalMs = data["intervalMs"] as? Double ?? 1000.0
      print("FloatViewApp: Starting data logging with interval: \(intervalMs)ms")

      // Send mock VESC data
      LocalServer.shared.webSocket.broadcastMessage(
        wsMessage(topic: "data", event: "log", data: [
          "adc1": 0,
          "adc2": 0,
          "ah": 0,
          "altitude": 0,
          "currentBattery": 0,
          "currentMotor": 0,
          "distance": 0,
          "duty": 0,
          "gpsAccuracy": 0,
          "gpsLatitude": 0,
          "gpsLongitude": 0,
          "motorFault": 0,
          "pitch": 0,
          "roll": 0,
          "speed": 0,
          "state": "riding",
          "stateRaw": 0,
          "tempMosfet": 0,
          "tempMotor": 0,
          "time": 0,
          "truePitch": 0,
          "voltage": 0,
          "wh": 0
        ])
      )
    }
  }

  private func sendMockFirmwareData() {
    LocalServer.shared.webSocket.broadcastMessage(
      wsMessage(topic: "data", event: "firmware", data: [
        "major": 6,
        "minor": 2,
        "name": "VESC FW",
        "uuid": "123456789ABC",
        "isPaired": true,
        "betaVersion": 0,
        "hwType": 1
      ])
    )
  }

  // TODO: Implement actual VESC manager listener similar to Android
  /*
  private func setupVescListener() {
    vescManager?.setListener(
      onScanDeviceFound: { name, address in
        LocalServer.shared.webSocket.broadcastMessage(
          wsMessage(topic: "scan", event: "device", data: [
            "name": name,
            "address": address
          ])
        )
      },
      onScanComplete: {
        LocalServer.shared.webSocket.broadcastMessage(
          wsMessage(topic: "scan", event: "status", data: ["status": "complete"])
        )
      },
      // ... other listener callbacks
    )
  }
  */

  private func setupVescListener() {
    vescManager.setListener(vescListener)
  }
}

// MARK: - VescManager Listener Implementation

private class VescListenerImpl: VescManager.VescManagerListener {
  func onScanDeviceFound(name: String?, address: String) {
    LocalServer.shared.webSocket.broadcastMessage(
      wsMessage(topic: "scan", event: "device", data: [
        "name": name ?? "Unknown",
        "address": address
      ])
    )
  }

  func onScanComplete() {
    LocalServer.shared.webSocket.broadcastMessage(
      wsMessage(topic: "scan", event: "status", data: ["status": "complete"])
    )
  }

  func onScanError(message: String) {
    LocalServer.shared.webSocket.broadcastMessage(
      wsMessage(topic: "scan", event: "error", data: ["message": message])
    )
  }

  func onConnectionStatusChanged(status: VescManager.ConnectionStatus, address: String?) {
    let statusString: String
    switch status {
    case .connecting:
      statusString = "connecting"
    case .connected:
      statusString = "connected"
    case .ready:
      statusString = "ready"
    case .disconnected:
      statusString = "disconnected"
    }

    LocalServer.shared.webSocket.broadcastMessage(
      wsMessage(topic: "connect", event: "status", data: ["status": statusString])
    )
  }

  func onConnectionError(message: String) {
    LocalServer.shared.webSocket.broadcastMessage(
      wsMessage(topic: "connect", event: "error", data: ["message": message])
    )
  }

  func onVescResponse(command: Int, data: Data) {
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
        wsMessage(topic: "data", event: "firmware", data: [
          "major": major,
          "minor": minor,
          "name": name,
          "uuid": uuid as Any,
          "isPaired": isPaired as Any,
          "betaVersion": betaVersion as Any,
          "hwType": hwType as Any
        ])
      )

    default:
      print("FloatView_FloatViewApp: Unhandled VESC response - Command: \(command), Data: \(data.map { String(format: "%02X", $0) }.joined(separator: " "))")
    }
  }

  func onCommandError(command: Int, message: String) {
    print("FloatView_FloatViewApp: Command error - Command: \(command), Error: \(message)")
    // TODO: Disconnect and send disconnect message
    LocalServer.shared.webSocket.broadcastMessage(
      wsMessage(topic: "connect", event: "disconnect", data: ["reason": "command error"])
    )
  }
}
