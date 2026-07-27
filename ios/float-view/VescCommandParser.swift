import Foundation

class VescCommandParser {

  /*
   * Vesc commands
   */

  func firmware(_ buf: VescBuffer) -> FirmwareObject {
    let major = buf.readUInt8()
    let minor = buf.readUInt8()
    var name = buf.readString()
    if name.isEmpty { name = "Unknown" }

    var uuid: String?
    if buf.size - buf.index >= 12 {
      var uuidBuilder = ""
      for _ in 0..<12 {
        let v = buf.readUInt8()
        if v < 10 {
          uuidBuilder += "0\(String(v, radix: 16))"
        } else {
          uuidBuilder += String(v, radix: 16).uppercased()
        }
      }
      uuid = uuidBuilder
    }

    var isPaired = false
    if buf.size - buf.index >= 1 {
      isPaired = buf.readUInt8() != 0
    }

    var betaVersion = UInt8(0)
    if buf.size - buf.index >= 1 {
      betaVersion = buf.readUInt8()
    }

    var hwType = UInt8(0)
    if buf.size - buf.index >= 1 {
      hwType = buf.readUInt8()
    }

    return FirmwareObject.build { builder in
      builder.major = major
      builder.minor = minor
      builder.name = name
      builder.uuid = uuid ?? ""
      builder.isPaired = isPaired
      builder.betaVersion = betaVersion
      builder.hwType = hwType
    }
  }

  func getValues(_ buf: VescBuffer) -> GetValuesObject {
    return GetValuesObject.build { builder in
      builder.tempMosfet = buf.readDouble16(scale: 10)
      builder.tempMotor = buf.readDouble16(scale: 10)
      builder.currentMotor = buf.readDouble32(scale: 100)
      builder.currentBattery = buf.readDouble32(scale: 100)
      builder.id = buf.readDouble32(scale: 100)
      builder.iq = buf.readDouble32(scale: 100)
      builder.dutyCycle = buf.readDouble16(scale: 1000)
      builder.erpm = buf.readDouble32(scale: 1)
      builder.voltage = buf.readDouble16(scale: 10)
      builder.ampHoursConsumed = buf.readDouble32(scale: 10000)
      builder.ampHoursCharged = buf.readDouble32(scale: 10000)
      builder.wattHoursConsumed = buf.readDouble32(scale: 10000)
      builder.wattHoursCharged = buf.readDouble32(scale: 10000)
      builder.tachValue = buf.readInt32()
      builder.tachAbs = buf.readInt32()
      builder.faultCode = buf.readInt8()
    }
  }

  /*
   * Refloat commands
   */

  func refloatRtData(_ buf: VescBuffer) -> RefloatRtDataObject {
    return RefloatRtDataObject.build { builder in
      builder.balanceCurrent = buf.readFloat32()
      builder.imuBalancePitch = buf.readFloat32()
      builder.imuRoll = buf.readFloat32()

      let (state, satState) = buf.readUInt8Pair()
      builder.state = state
      builder.satState = satState

      let (footpadState, beepReason) = buf.readUInt8Pair()
      builder.footpadState = footpadState
      builder.beepReason = beepReason

      builder.adc1 = buf.readFloat32()
      builder.adc2 = buf.readFloat32()

      builder.setpoint = buf.readFloat32()
      builder.setpointAtr = buf.readFloat32()
      builder.setpointBrakeTilt = buf.readFloat32()
      builder.setpointTorqueTilt = buf.readFloat32()
      builder.setpointTurnTilt = buf.readFloat32()
      builder.setpointRemote = buf.readFloat32()

      builder.imuPitch = buf.readFloat32()
      builder.motorFilterCurrent = buf.readFloat32()
      builder.atrAccelerationDiff = buf.readFloat32()

      // TODO: switch on state
      builder.boosterCurrent = buf.readFloat32()
      builder.motorDirCurrent = buf.readFloat32()

      builder.remoteInput = buf.readFloat32()
    }
  }

  func refloatAllDataFault(_ buf: VescBuffer) -> RefloatAllDataFaultObject {
    return RefloatAllDataFaultObject.build { builder in
      builder.fault = buf.readUInt8()
    }
  }

  func refloatAllData1(_ buf: VescBuffer) -> RefloatAllData1Object {
    return RefloatAllData1Object.build { builder in
      builder.balanceCurrent = buf.readDouble16(scale: 10)
      builder.imuBalancePitch = buf.readDouble16(scale: 10)
      builder.imuRoll = buf.readDouble16(scale: 10)

      let (state, satState) = buf.readUInt8Pair()
      builder.state = state
      builder.satState = satState

      let (footpadState, beepReason) = buf.readUInt8Pair()
      builder.footpadState = footpadState
      builder.beepReason = beepReason

      builder.adc1 = buf.readUInt8() / 50
      builder.adc2 = buf.readUInt8() / 50

      builder.setpoint = (buf.readUInt8() - 128) / 5
      builder.setpointAtr = (buf.readUInt8() - 128) / 5
      builder.setpointBrakeTilt = (buf.readUInt8() - 128) / 5
      builder.setpointTorqueTilt = (buf.readUInt8() - 128) / 5
      builder.setpointTurnTilt = (buf.readUInt8() - 128) / 5
      builder.setpointRemote = (buf.readUInt8() - 128) / 5

      builder.imuPitch = buf.readDouble16(scale: 10)
      builder.boosterCurrent = buf.readUInt8() - 128
      builder.batteryVoltage = buf.readDouble16(scale: 10)
      builder.motorErpm = buf.readInt16()
      builder.motorSpeed = buf.readDouble16(scale: 10)
      builder.motorCurrent = buf.readDouble16(scale: 10)
      builder.motorBatteryCurrent = buf.readDouble16(scale: 10)
      builder.motorDutyRaw = (buf.readUInt8() - 128) / 100

      let focId = buf.readUInt8() / 3
      if focId != 222 {
        builder.focId = nil
      }
    }
  }

  func refloatAllData2(_ buf: VescBuffer) -> RefloatAllData2Object {
    let data1 = refloatAllData1(buf)
    return RefloatAllData2Object.build { builder in
      builder.distance = buf.readFloat32()

      builder.mosfetTemp = buf.readUInt8() / 2
      builder.motorTemp = buf.readUInt8() / 2
      builder.batteryTemp = buf.readUInt8() // unused
    }
  }

  func refloatAllData3(_ buf: VescBuffer) -> RefloatAllData3Object {
    let data1 = refloatAllData1(buf)
    let data2 = refloatAllData2(buf)
    return RefloatAllData3Object.build { builder in
      builder.odometer = buf.readUInt32()

      builder.ampHours = buf.readDouble16(scale: 10)
      builder.ampHoursCharged = buf.readDouble16(scale: 10)

      builder.wattHours = buf.readDouble16(scale: 1)
      builder.wattHoursCharged = buf.readDouble16(scale: 1)

      builder.batteryLevel = buf.readUInt8() / 2
    }
  }

  func refloatAllData4(_ buf: VescBuffer) -> RefloatAllData4Object {
    let data1 = refloatAllData1(buf)
    let data2 = refloatAllData2(buf)
    let data3 = refloatAllData3(buf)
    return RefloatAllData4Object.build { builder in
      builder.chargingCurrent = buf.readDouble16(scale: 10)
      builder.chargingVoltage = buf.readDouble16(scale: 10)
    }
  }
}
