
class VescCommandGenerator {
  private var canId: Int? = nil

  func setCanId(_ id: Int?) {
    canId = id
  }

  private func getPayload(_ values: Int...) -> [UInt8] {
    let payload = [Int]()
    if let id = canId {
      payload.add(CommPacketId.CommForwardCan)
      payload.add(id)
    }

    payload.add(contentsOf: values)
    return payload
  }

  /*
   * Vesc commands
   */

  func firmware() -> [UInt8] {
    return getPayload(CommPacketId.CommFwVersion)
  }

  func commGetValues() -> [UInt8] {
    return getPayload(CommPacketId.CommGetValues)
  }

  /*
   * Refloat commands
   */

  private func getRefloatPayload(_ values: Int...) -> [UInt8] {
    var payload = [Int]()
    payload.add(CommPacketId.CommCustomAppData)
    payload.add(101)
    payload.add(contentsOf: values)
    return payload
  }

  func refloatInfo(version: Int) -> [UInt8] {
    return getRefloatPayload(RefloatCommPacketId.CommandInfo, version)
  }

  func refloatRtdata() -> [UInt8] {
    return getRefloatPayload(RefloatCommPacketId.CommandGetRtdata)
  }
}
