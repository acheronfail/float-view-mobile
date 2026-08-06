public enum VescCommand: UInt8, Sendable {
  case getFirmwareVersion = 0
  case getRealtimeValues = 4
  case getMotorConfiguration = 14
  case forwardCAN = 34
  case customAppData = 36
  case pingCAN = 62
  case bmsGetValues = 96
}
