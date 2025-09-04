package fail.acheron.floatview

class VescCommandLayer(private val address: String) {
    fun parse(command: Int, response: ByteArray) {
        val parser = VescBuffer(response)
        when (command) {
            COMM_FW_VERSION ->
                    LocalServer.broadcastMessage(
                            BackendMessage.commandFirmware(address, parseFirmware(parser))
                    )
            else -> {
                LocalServer.broadcastMessage(
                        BackendMessage.commandError(
                                command.toDouble(),
                                "No parser for command: $command"
                        )
                )
            }
        }
    }

    private fun parseFirmware(parser: VescBuffer): FirmwareObject {
        val major = parser.readUInt8()
        val minor = parser.readUInt8()
        var name = parser.readString()
        if (name.isEmpty()) name = "Unknown"

        var uuid: String? = null
        if (parser.size() - parser.index >= 12) {
            val sb = StringBuilder()
            for (i in 0 until 12) {
                val v = parser.readUInt8()
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
        if (parser.size() - parser.index >= 1) {
            isPaired = parser.readUInt8() != 0
        }

        var betaVersion: Int? = null
        if (parser.size() - parser.index >= 1) {
            betaVersion = parser.readUInt8()
        }

        var hwType: Int? = null
        if (parser.size() - parser.index >= 1) {
            hwType = parser.readUInt8()
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
}
