public enum VescCommand: UInt8, Sendable {
  case getFirmwareVersion = 0
  case getRealtimeValues = 4
  case getMotorConfiguration = 14
  case forwardCAN = 34
  case customAppData = 36
  case pingCAN = 62
  case getCustomConfigurationXML = 92
  case getCustomConfiguration = 93
  case getCustomConfigurationDefault = 94
  case setCustomConfiguration = 95
  case bmsGetValues = 96
  case shutdown = 156
}
