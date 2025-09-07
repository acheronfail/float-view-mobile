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

    var betaVersion = 0
    if buf.size - buf.index >= 1 {
      betaVersion = Int(buf.readUInt8())
    }

    var hwType = 0
    if buf.size - buf.index >= 1 {
      hwType = Int(buf.readUInt8())
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
      builder.tachValue = Double(buf.readInt32())
      builder.tachAbs = Double(buf.readInt32())
      builder.faultCode = Double(buf.readInt8())
    }
  }

  /*
   * Refloat commands
   */

  func refloatRtData(_ buf: VescBuffer) -> RefloatRtDataObject {
    return RefloatRtDataObject.build { builder in
      builder.balanceCurrent = Double(buf.readFloat32())
      builder.imuBalancePitch = Double(buf.readFloat32())
      builder.imuRoll = Double(buf.readFloat32())

      let (state, satState) = buf.readUInt8Pair()
      builder.state = Double(state)
      builder.satState = Double(satState)

      let (footpadState, beepReason) = buf.readUInt8Pair()
      builder.footpadState = Double(footpadState)
      builder.beepReason = Double(beepReason)

      builder.adc1 = Double(buf.readFloat32())
      builder.adc2 = Double(buf.readFloat32())

      builder.setpoint = Double(buf.readFloat32())
      builder.setpointAtr = Double(buf.readFloat32())
      builder.setpointBrakeTilt = Double(buf.readFloat32())
      builder.setpointTorqueTilt = Double(buf.readFloat32())
      builder.setpointTurnTilt = Double(buf.readFloat32())
      builder.setpointRemote = Double(buf.readFloat32())

      builder.imuPitch = Double(buf.readFloat32())
      builder.motorFilterCurrent = Double(buf.readFloat32())
      builder.atrAccelerationDiff = Double(buf.readFloat32())

      // TODO: switch on state
      builder.boosterCurrent = Double(buf.readFloat32())
      builder.motorDirCurrent = Double(buf.readFloat32())

      builder.remoteInput = Double(buf.readFloat32())
    }
  }
}
