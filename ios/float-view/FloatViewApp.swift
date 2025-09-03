import SwiftUI

func print(_ items: Any..., separator: String = " ", terminator: String = "\n") {
  let output = items.map { "\($0)" }.joined(separator: separator)
  NSLog("%@", output)
}

@main
struct FloatViewApp: App {

  private let vescManager: VescManager

  init() {
    // Start server synchronously right now, before any views are created
    LocalServer.shared.start()

    // Initialize VescManager and listener first
    vescManager = VescManager()

    // Now setup handlers and listeners
    setupWebSocketHandlers()

    // TODO: use messages here
    let msg = WsMessage.with {
      $0.scanDevice = ScanDevice.with {
        $0.address = "AAAC"
        $0.name = "BBBB"
      }
    }

    do {
      let data = try msg.serializedData()
      print("data: \(Array(data))")
    } catch {
      print("Error serializing protobuf: \(error)")
    }
  }

  var body: some Scene {
    WindowGroup {
      ContentView(server: LocalServer.shared).preferredColorScheme(.dark)
    }
  }

  // MARK: - WebSocket Handler Setup (similar to MainActivity.kt)

  private func setupWebSocketHandlers() {
    LocalServer.shared.addHandler("scan") { [vescManager] (data: [String: Any]) in
      vescManager.startScan()
    }

    LocalServer.shared.addHandler("connect") { [vescManager] (data: [String: Any]) in
      guard let address = data["address"] as? String else {
        LocalServer.shared.broadcastMessage(
          wsMessage(topic: "connect", event: "error", data: ["message": "no address"])
        )
        return
      }

      vescManager.connect(address: address)
    }

    LocalServer.shared.addHandler("disconnect") { [vescManager] (data: [String: Any]) in
      vescManager.disconnect()
    }

    LocalServer.shared.addHandler("command") { [vescManager] (data: [String: Any]) in
      let command = data["command"] as? Int
      let payload = data["payload"] as? [UInt8] ?? []
      if let cmd = command {
        vescManager.send(command: cmd, payload: payload)
      } else {
        LocalServer.shared.broadcastMessage(
          wsMessage(
            topic: "command",
            event: "error",
            data: ["message": "no command"]
          )
        )
      }
    }

    LocalServer.shared.addHandler("data") { (data: [String: Any]) in
      guard let shouldLog = data["log"] as? Bool else { return }

      if !shouldLog {
        // TODO: stop logging interval
        return
      }

      let intervalMs = data["intervalMs"] as? Double ?? 1000.0
      print("TODO: log with interval: \(intervalMs)ms")

      // Send mock VESC data
      LocalServer.shared.broadcastMessage(
        wsMessage(
          topic: "data", event: "log",
          data: [
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
            "wh": 0,
          ])
      )
    }
  }

}
