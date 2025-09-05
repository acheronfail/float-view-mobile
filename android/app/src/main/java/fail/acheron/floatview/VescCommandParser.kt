package fail.acheron.floatview

class VescCommandParser() {
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

    /** Refloat */
    fun refloatInfo(buf: VescBuffer) {
        val appId = buf.readUInt8()
        val appPacketId = buf.readUInt8()

        // TODO:
    }

    fun refloatRtData(buf: VescBuffer) {
        val appId = buf.readUInt8()
        val appPacketId = buf.readUInt8()

        val balanceCurrent = buf.readFloat32()
        val imuBalancePitch = buf.readFloat32()
        val imuRoll = buf.readFloat32()

        val (state, satState) = buf.readUint8Pair()
        val (footpadState, beepReason) = buf.readUint8Pair()
        val adc1 = buf.readFloat32()
        val adc2 = buf.readFloat32()

        val setpoint = buf.readFloat32()
        val setpointAtr = buf.readFloat32()
        val setpointBrakeTilt = buf.readFloat32()
        val setpointTorqueTilt = buf.readFloat32()
        val setpointTurnTilt = buf.readFloat32()
        val setpointRemote = buf.readFloat32()

        val imuPitch = buf.readFloat32()
        val motorFilterCurrent = buf.readFloat32()
        val atrAccelerationDiff = buf.readFloat32()

        // TODO: switch on state
        val boosterCurrent = buf.readFloat32()
        val motorDirCurrent = buf.readFloat32()

        val remoteInput = buf.readFloat32()

        val dataMap =
                mapOf(
                        "appId" to appId,
                        "appPacketId" to appPacketId,
                        "balanceCurrent" to balanceCurrent,
                        "imuBalancePitch" to imuBalancePitch,
                        "imuRoll" to imuRoll,
                        "state" to state,
                        "satState" to satState,
                        "footpadState" to footpadState,
                        "beepReason" to beepReason,
                        "adc1" to adc1,
                        "adc2" to adc2,
                        "setpoint" to setpoint,
                        "setpointAtr" to setpointAtr,
                        "setpointBrakeTilt" to setpointBrakeTilt,
                        "setpointTorqueTilt" to setpointTorqueTilt,
                        "setpointTurnTilt" to setpointTurnTilt,
                        "setpointRemote" to setpointRemote,
                        "imuPitch" to imuPitch,
                        "motorFilterCurrent" to motorFilterCurrent,
                        "atrAccelerationDiff" to atrAccelerationDiff,
                        "boosterCurrent" to boosterCurrent,
                        "motorDirCurrent" to motorDirCurrent,
                        "remoteInput" to remoteInput
                )

        println("Refloat Info Data: $dataMap")
    }
}
