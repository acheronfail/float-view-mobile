public enum VescProtocolError: Error, Equatable {
  case invalidFrame
  case invalidCRC
  case payloadTooLarge
  case unsupportedCommand(UInt8)
  case truncatedPayload
}
