import Foundation

class VescCommandParser {
  func parse(command: Int, data: Data) {
    switch command {
    case VescMessaging.COMM_FW_VERSION:
      parseFirmware(command, data)
    default:
      print(
        "FloatView_FloatViewApp: Unhandled VESC response - Command: \(command), Data: \(data.map { String(format: "%02X", $0) }.joined(separator: " "))"
      )
    }
  }

  private func parseFirmware(_ command: Int, _ data: Data) {
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
  }
}
