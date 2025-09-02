package fail.acheron.floatview

import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put

class VescCommandLayer {
    fun parse(command: Int, response: ByteArray): JsonObject? {
        val parser = VescBuffer(response)
        when (command) {
            COMM_FW_VERSION -> return parseFirmware(parser)
            else -> {
                printError(
                        "Unhandled command $command: ${response.joinToString(", ") { "%02X".format(it) }}"
                )
                return null
            }
        }
    }

    private fun parseFirmware(parser: VescBuffer): JsonObject {
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

        return buildJsonObject {
            put("major", major)
            put("minor", minor)
            put("name", name)
            put("uuid", uuid)
            put("isPaired", isPaired)
            put("betaVersion", betaVersion)
            put("hwType", hwType)
        }
    }
}
