package fail.acheron.floatview.protocol.vesc

import fail.acheron.floatview.application.VescScheduledRequest
import fail.acheron.floatview.protocol.RustVescCustomConfigurationBridge
import fail.acheron.floatview.protocol.VescProtocolException
import fail.acheron.floatview.protocol.verifyVescCustomConfigurationAbiVersion

sealed interface VescCustomConfigurationResponse {
    data class SchemaChunk(val index: Int, val totalLength: Int, val offset: Int, val bytes: ByteArray) : VescCustomConfigurationResponse
    data class Values(val command: VescCommand, val index: Int, val bytes: ByteArray) : VescCustomConfigurationResponse
    data object WriteAcknowledgement : VescCustomConfigurationResponse
}

object VescCustomConfigurationCodec {
    const val INDEX = 0
    const val MAXIMUM_CHUNK_BYTES = 400

    private const val OPERATION_SCHEMA = 1
    private const val OPERATION_CURRENT = 2
    private const val OPERATION_DEFAULT = 3
    private const val OPERATION_WRITE = 4
    private const val RESPONSE_SCHEMA = 1
    private const val RESPONSE_CURRENT = 2
    private const val RESPONSE_DEFAULT = 3
    private const val RESPONSE_WRITE_ACKNOWLEDGEMENT = 4
    private const val STATUS_OK = 0
    private const val STATUS_UNSUPPORTED_COMMAND = 3
    private const val DECODE_RESULT_INTS = 8
    private val linkedAbiVersion = RustVescCustomConfigurationBridge.abiVersion()

    fun schemaRequest(index: Int = INDEX, length: Int, offset: Int): ByteArray =
        encode(OPERATION_SCHEMA, index, length.toLong(), offset.toLong(), ByteArray(0), 10)

    fun valuesRequest(index: Int = INDEX, defaults: Boolean): ByteArray =
        encode(
            if (defaults) OPERATION_DEFAULT else OPERATION_CURRENT,
            index,
            0,
            0,
            ByteArray(0),
            2,
        )

    fun writeRequest(index: Int = INDEX, values: ByteArray): ByteArray =
        encode(
            OPERATION_WRITE,
            index,
            0,
            0,
            values,
            minOf(values.size.toLong() + 2, VescScheduledRequest.MAX_PAYLOAD_BYTES.toLong()).toInt(),
        )

    fun decode(payload: ByteArray): VescCustomConfigurationResponse {
        verifyVescCustomConfigurationAbiVersion(linkedAbiVersion)
        val result = IntArray(DECODE_RESULT_INTS)
        val status = RustVescCustomConfigurationBridge.decode(payload, result)
        return decodeBridgeResult(status, payload, result)
    }

    internal fun decodeBridgeResult(
        status: Int,
        payload: ByteArray,
        result: IntArray,
    ): VescCustomConfigurationResponse {
        if (result.size != DECODE_RESULT_INTS) fail()
        val kind = result[0]
        val command = result[1]
        val index = result[2]
        val total = result[3]
        val offset = result[4]
        val dataOffset = result[5]
        val dataLength = result[6]
        val unsupported = result[7]
        if (status == STATUS_UNSUPPORTED_COMMAND) {
            if (result.copyOfRange(0, 7).any { it != 0 } || unsupported !in 0..255) fail()
            throw VescProtocolException.UnsupportedCommand(unsupported)
        }
        if (status != STATUS_OK || unsupported != 0 || command !in 0..255 || index !in 0..255) fail()
        val dataEnd = dataOffset.toLong() + dataLength
        if (dataOffset < 0 || dataLength < 0 || dataEnd > payload.size.toLong()) fail()
        val bytes = payload.copyOfRange(dataOffset, dataEnd.toInt())
        return when (kind) {
            RESPONSE_SCHEMA -> {
                if (
                    command != VescCommand.GET_CUSTOM_CONFIGURATION_XML.id ||
                    total <= 0 ||
                    offset < 0 ||
                    dataOffset != 10 ||
                    dataLength > MAXIMUM_CHUNK_BYTES ||
                    offset > total ||
                    dataLength > total - offset ||
                    dataEnd != payload.size.toLong()
                ) fail()
                VescCustomConfigurationResponse.SchemaChunk(index, total, offset, bytes)
            }
            RESPONSE_CURRENT, RESPONSE_DEFAULT -> {
                val expected = if (kind == RESPONSE_CURRENT) {
                    VescCommand.GET_CUSTOM_CONFIGURATION
                } else {
                    VescCommand.GET_CUSTOM_CONFIGURATION_DEFAULT
                }
                if (
                    command != expected.id ||
                    total != 0 ||
                    offset != 0 ||
                    dataOffset != 2 ||
                    dataLength < 4 ||
                    dataEnd != payload.size.toLong()
                ) fail()
                VescCustomConfigurationResponse.Values(expected, index, bytes)
            }
            RESPONSE_WRITE_ACKNOWLEDGEMENT -> {
                if (
                    command != VescCommand.SET_CUSTOM_CONFIGURATION.id ||
                    index != 0 ||
                    total != 0 ||
                    offset != 0 ||
                    dataOffset != 0 ||
                    dataLength != 0 ||
                    payload.size != 1
                ) fail()
                VescCustomConfigurationResponse.WriteAcknowledgement
            }
            else -> fail()
        }
    }

    private fun encode(
        operation: Int,
        index: Int,
        length: Long,
        offset: Long,
        values: ByteArray,
        outputCapacity: Int,
    ): ByteArray {
        verifyVescCustomConfigurationAbiVersion(linkedAbiVersion)
        val output = ByteArray(outputCapacity)
        val result = IntArray(1)
        val status = RustVescCustomConfigurationBridge.encode(
            operation,
            index,
            length,
            offset,
            values,
            output,
            result,
        )
        if (status != STATUS_OK || result[0] !in 1..output.size) fail()
        return output.copyOf(result[0])
    }

    private fun fail(): Nothing = throw VescProtocolException.TruncatedPayload
}
