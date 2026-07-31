public enum RefloatCommand: UInt8, Sendable {
  case info = 0
  case getAllData = 10
  case realtimeDataInternal = 31
  case realtimeDataInternalIDs = 32
}

public enum RefloatWire {
  public static let packageID: UInt8 = 101
  public static let infoProtocolVersion: UInt8 = 2
  public static let infoFlags: UInt8 = 0
  public static let legacyTelemetryDetailLevel: UInt8 = 4

  public static func request(
    _ command: RefloatCommand,
    parameters: [UInt8] = []
  ) -> [UInt8] {
    [
      VescCommand.customAppData.rawValue,
      packageID,
      command.rawValue,
    ] + parameters
  }

  public static func command(in payload: [UInt8]) -> RefloatCommand? {
    guard payload.count >= 3,
          payload[0] == VescCommand.customAppData.rawValue,
          payload[1] == packageID
    else { return nil }
    return RefloatCommand(rawValue: payload[2])
  }
}
