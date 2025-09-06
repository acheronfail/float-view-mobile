package fail.acheron.floatview

// TODO: a lot of "toDouble" being called, probably should keep proper types in generated classes

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
                if (v < 10) {
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
            isPaired = buf.readUInt8() != 0
        }

        var betaVersion: Int? = null
        if (buf.size() - buf.index >= 1) {
            betaVersion = buf.readUInt8()
        }

        var hwType: Int? = null
        if (buf.size() - buf.index >= 1) {
            hwType = buf.readUInt8()
        }

        return FirmwareObject.build {
            this.major = major.toDouble()
            this.minor = minor.toDouble()
            this.name = name
            this.uuid = uuid ?: ""
            this.isPaired = isPaired ?: false
            this.betaVersion = betaVersion?.toDouble() ?: 0.0
            this.hwType = hwType?.toDouble() ?: 0.0
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
            this.tachValue = buf.readInt32().toDouble()
            this.tachAbs = buf.readInt32().toDouble()
            this.faultCode = buf.readInt8().toDouble()
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
            this.balanceCurrent = buf.readFloat32().toDouble()
            this.imuBalancePitch = buf.readFloat32().toDouble()
            this.imuRoll = buf.readFloat32().toDouble()

            val (state, satState) = buf.readUint8Pair()
            this.state = state.toDouble()
            this.satState = satState.toDouble()

            val (footpadState, beepReason) = buf.readUint8Pair()
            this.footpadState = footpadState.toDouble()
            this.beepReason = beepReason.toDouble()

            this.adc1 = buf.readFloat32().toDouble()
            this.adc2 = buf.readFloat32().toDouble()

            this.setpoint = buf.readFloat32().toDouble()
            this.setpointAtr = buf.readFloat32().toDouble()
            this.setpointBrakeTilt = buf.readFloat32().toDouble()
            this.setpointTorqueTilt = buf.readFloat32().toDouble()
            this.setpointTurnTilt = buf.readFloat32().toDouble()
            this.setpointRemote = buf.readFloat32().toDouble()

            this.imuPitch = buf.readFloat32().toDouble()
            this.motorFilterCurrent = buf.readFloat32().toDouble()
            this.atrAccelerationDiff = buf.readFloat32().toDouble()

            // TODO: switch on state
            this.boosterCurrent = buf.readFloat32().toDouble()
            this.motorDirCurrent = buf.readFloat32().toDouble()

            this.remoteInput = buf.readFloat32().toDouble()
        }
    }
}
