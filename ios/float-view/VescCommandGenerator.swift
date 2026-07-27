
class VescCommandGenerator {
  private var canId: UInt8? = nil

  func setCanId(_ id: UInt8?) {
    canId = id
  }

  private func getPayload(_ values: UInt8...) -> [UInt8] {
    var payload = [UInt8]()
    if let id = canId {
      payload.append(UInt8(CommPacketId.COMM_FORWARD_CAN.rawValue))
      payload.append(id)
    }

    payload.append(contentsOf: values)
    return payload
  }

  /*
   * Vesc commands
   */

  func firmware() -> [UInt8] {
      return getPayload(UInt8(CommPacketId.COMM_FW_VERSION.rawValue))
  }

  func commGetValues() -> [UInt8] {
      return getPayload(UInt8(CommPacketId.COMM_GET_VALUES.rawValue))
  }

  func commGetValuesSetup() -> [UInt8] {
      return getPayload(UInt8(CommPacketId.COMM_GET_VALUES_SETUP.rawValue))
  }

  /*
   * Refloat commands
   */

  private func getRefloatPayload(_ values: UInt8...) -> [UInt8] {
    var payload = getPayload(UInt8(CommPacketId.COMM_CUSTOM_APP_DATA.rawValue), 101)
    payload.append(contentsOf: values)
    return payload
  }

  func refloatInfo(version: Int) -> [UInt8] {
      return getRefloatPayload(UInt8(RefloatCommPacketId.COMMAND_INFO.rawValue), UInt8(version))
  }

  func refloatRtdata() -> [UInt8] {
      return getRefloatPayload(UInt8(RefloatCommPacketId.COMMAND_GET_RTDATA.rawValue))
  }

  func refloatGetAlldata(_ mode: Int8) -> [UInt8] {
      return getRefloatPayload(UInt8(RefloatCommPacketId.COMMAND_GET_ALLDATA.rawValue), UInt8(mode))
  }
}
