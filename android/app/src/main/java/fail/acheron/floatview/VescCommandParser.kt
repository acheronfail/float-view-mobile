package fail.acheron.floatview

class VescCommandParser() {
    /*
     * Vesc command
     */

    fun firmware(buf: VescBuffer): FirmwareObject {
        val major = buf.readUInt8()
        val minor = buf.readUInt8()
        var name = buf.readString()
        if (name.isEmpty()) name = "Unknown"

        var uuid: String? = null
        if (buf.size() - buf.index >= 12) {
            val sb = StringBuilder()
            for (i in 0 until 12) {
                val v = buf.readUInt8()
                if (v < 10U) {
                    sb.append("0")
                    sb.append(v.toString(16))
                } else {
                    sb.append(v.toString(16).uppercase())
                }
            }
            uuid = sb.toString()
        }

        var isPaired: Boolean? = null
        if (buf.size() - buf.index >= 1) {
            isPaired = buf.readUInt8() != 0U
        }

        var betaVersion: UInt? = null
        if (buf.size() - buf.index >= 1) {
            betaVersion = buf.readUInt8()
        }

        var hwType: UInt? = null
        if (buf.size() - buf.index >= 1) {
            hwType = buf.readUInt8()
        }

        return FirmwareObject.build {
            this.major = major
            this.minor = minor
            this.name = name
            this.uuid = uuid ?: ""
            this.isPaired = isPaired ?: false
            this.betaVersion = betaVersion ?: 0U
            this.hwType = hwType ?: 0U
        }
    }

    fun getValues(buf: VescBuffer): GetValuesObject {
        return GetValuesObject.build {
            this.tempMosfet = buf.readDouble16(10)
            this.tempMotor = buf.readDouble16(10)
            this.currentMotor = buf.readDouble32(100)
            this.currentBattery = buf.readDouble32(100)
            this.id = buf.readDouble32(100)
            this.iq = buf.readDouble32(100)
            this.dutyCycle = buf.readDouble16(1000)
            this.erpm = buf.readDouble32(1)
            this.voltage = buf.readDouble16(10)
            this.ampHoursConsumed = buf.readDouble32(10000)
            this.ampHoursCharged = buf.readDouble32(10000)
            this.wattHoursConsumed = buf.readDouble32(10000)
            this.wattHoursCharged = buf.readDouble32(10000)
            this.tachValue = buf.readInt32()
            this.tachAbs = buf.readInt32()
            this.faultCode = buf.readInt8()
        }
    }

    fun getValuesSetup(buf: VescBuffer): GetValuesObject {
        print(buf)
        return GetValuesObject.build {
            // TODO: implement
        }
    }

    /**
     * Refloat commands NOTE: the `appId` and `appPacketId` are stripped before calling these
     * functions
     */
    fun refloatInfo(buf: VescBuffer) {
        // TODO:
        print(buf)
    }

    fun refloatRtData(buf: VescBuffer): RefloatRtDataObject {
        return RefloatRtDataObject.build {
            this.balanceCurrent = buf.readFloat32()
            this.imuBalancePitch = buf.readFloat32()
            this.imuRoll = buf.readFloat32()

            val (state, satState) = buf.readUint8Pair()
            this.state = state
            this.satState = satState

            val (footpadState, beepReason) = buf.readUint8Pair()
            this.footpadState = footpadState
            this.beepReason = beepReason

            this.adc1 = buf.readFloat32()
            this.adc2 = buf.readFloat32()

            this.setpoint = buf.readFloat32()
            this.setpointAtr = buf.readFloat32()
            this.setpointBrakeTilt = buf.readFloat32()
            this.setpointTorqueTilt = buf.readFloat32()
            this.setpointTurnTilt = buf.readFloat32()
            this.setpointRemote = buf.readFloat32()

            this.imuPitch = buf.readFloat32()
            this.motorFilterCurrent = buf.readFloat32()
            this.atrAccelerationDiff = buf.readFloat32()

            // TODO: switch on state
            this.boosterCurrent = buf.readFloat32()
            this.motorDirCurrent = buf.readFloat32()

            this.remoteInput = buf.readFloat32()
        }
    }
}
