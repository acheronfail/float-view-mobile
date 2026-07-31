package fail.acheron.floatview.protocol.refloat

object RefloatInfoDecoder {
    fun decodeVersion(payload: ByteArray): String {
        val responseVersion = payload[3].toInt() and 0xff
        if (responseVersion == 10) return "1.0"
        if (responseVersion > 10) return "${responseVersion / 10}.${responseVersion % 10}"
        require(responseVersion == RefloatWire.INFO_PROTOCOL_VERSION && payload.size >= 61) {
            "Truncated Refloat INFO v2 response"
        }
        return "${payload[25].toInt() and 0xff}.${payload[26].toInt() and 0xff}." +
            "${payload[27].toInt() and 0xff}"
    }
}
