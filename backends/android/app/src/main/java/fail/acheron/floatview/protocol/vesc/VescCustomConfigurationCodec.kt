package fail.acheron.floatview.protocol.vesc

import fail.acheron.floatview.backend.VescScheduledRequest
import fail.acheron.floatview.protocol.VescProtocolException

sealed interface VescCustomConfigurationResponse {
    data class SchemaChunk(val index: Int, val totalLength: Int, val offset: Int, val bytes: ByteArray) : VescCustomConfigurationResponse
    data class Values(val command: VescCommand, val index: Int, val bytes: ByteArray) : VescCustomConfigurationResponse
    data object WriteAcknowledgement : VescCustomConfigurationResponse
}

object VescCustomConfigurationCodec {
    const val INDEX = 0
    const val MAXIMUM_CHUNK_BYTES = 400

    fun schemaRequest(index: Int = INDEX, length: Int, offset: Int): ByteArray {
        if (index !in 0..255 || length !in 1..MAXIMUM_CHUNK_BYTES || offset < 0) fail()
        return byteArrayOf(VescCommand.GET_CUSTOM_CONFIGURATION_XML.byte, index.toByte()) + i32(length) + i32(offset)
    }

    fun valuesRequest(index: Int = INDEX, defaults: Boolean): ByteArray = byteArrayOf(
        if (defaults) VescCommand.GET_CUSTOM_CONFIGURATION_DEFAULT.byte else VescCommand.GET_CUSTOM_CONFIGURATION.byte,
        index.toByte(),
    )

    fun writeRequest(index: Int = INDEX, values: ByteArray): ByteArray {
        if (index !in 0..255 || values.isEmpty() || values.size > VescScheduledRequest.MAX_PAYLOAD_BYTES - 2) fail()
        return byteArrayOf(VescCommand.SET_CUSTOM_CONFIGURATION.byte, index.toByte()) + values
    }

    fun decode(payload: ByteArray): VescCustomConfigurationResponse {
        val command = payload.firstOrNull()?.toInt()?.and(0xff)?.let { raw -> VescCommand.entries.firstOrNull { it.id == raw } } ?: fail()
        return when (command) {
            VescCommand.GET_CUSTOM_CONFIGURATION_XML -> {
                if (payload.size < 10) fail()
                val total = positiveI32(payload, 2)
                val offset = nonnegativeI32(payload, 6)
                val bytes = payload.copyOfRange(10, payload.size)
                if (bytes.size > MAXIMUM_CHUNK_BYTES || offset > total || bytes.size > total - offset) fail()
                VescCustomConfigurationResponse.SchemaChunk(payload[1].toInt() and 0xff, total, offset, bytes)
            }
            VescCommand.GET_CUSTOM_CONFIGURATION, VescCommand.GET_CUSTOM_CONFIGURATION_DEFAULT -> {
                if (payload.size < 6) fail()
                VescCustomConfigurationResponse.Values(command, payload[1].toInt() and 0xff, payload.copyOfRange(2, payload.size))
            }
            VescCommand.SET_CUSTOM_CONFIGURATION -> {
                if (payload.size != 1) fail()
                VescCustomConfigurationResponse.WriteAcknowledgement
            }
            else -> throw VescProtocolException.UnsupportedCommand(command.id)
        }
    }

    private fun i32(value: Int) = byteArrayOf((value ushr 24).toByte(), (value ushr 16).toByte(), (value ushr 8).toByte(), value.toByte())
    private fun positiveI32(bytes: ByteArray, offset: Int) = nonnegativeI32(bytes, offset).also { if (it == 0) fail() }
    private fun nonnegativeI32(bytes: ByteArray, offset: Int): Int {
        if (offset + 4 > bytes.size) fail()
        val value = ((bytes[offset].toLong() and 0xffL) shl 24) or
            ((bytes[offset + 1].toLong() and 0xffL) shl 16) or
            ((bytes[offset + 2].toLong() and 0xffL) shl 8) or
            (bytes[offset + 3].toLong() and 0xffL)
        if (value > Int.MAX_VALUE) fail()
        return value.toInt()
    }
    private fun fail(): Nothing = throw VescProtocolException.TruncatedPayload
}
