import Foundation

class VescCommandLayer {
  private let address: String

  init(_ address: String) {
    self.address = address
  }

  func parse(command: Int, data: Data) {
    switch command {
    case VescBluetoothLayer.COMM_FW_VERSION:
      LocalServer.shared.broadcastMessage(
        BackendMessage.commandFirmware(address: address, firmware: parseFirmware(command, data))
      )
    default:
      LocalServer.shared.broadcastMessage(
        BackendMessage.commandError(
          message: "No parser for command: \(command)"
        )
      )
    }
  }

  private func parseFirmware(_ command: Int, _ data: Data) -> FirmwareObject {
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

    var isPaired = false
    if parser.size - parser.index >= 1 {
      isPaired = parser.readUInt8() != 0
    }

    var betaVersion = 0
    if parser.size - parser.index >= 1 {
      betaVersion = Int(parser.readUInt8())
    }

    var hwType = 0
    if parser.size - parser.index >= 1 {
      hwType = Int(parser.readUInt8())
    }

    return FirmwareObject.build { builder in
      builder.major = Double(major)
      builder.minor = Double(minor)
      builder.name = name
      builder.uuid = uuid ?? ""
      builder.isPaired = isPaired
      builder.betaVersion = Double(betaVersion)
      builder.hwType = Double(hwType)
    }
  }
}
