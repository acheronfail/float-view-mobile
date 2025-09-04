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

    /**
     * Refloat
     */

     fun refloatInfo(buf: VescBuffer) {
      // TODO:
     }

     fun refloatRtData(buf: VescBuffer) {
      // TODO:
     }
}
