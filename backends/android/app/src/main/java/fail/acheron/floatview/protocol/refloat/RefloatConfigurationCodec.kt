package fail.acheron.floatview.protocol.refloat

import java.io.ByteArrayOutputStream
import java.io.File
import java.nio.ByteBuffer
import java.nio.charset.CodingErrorAction

object RefloatConfigurationLimits {
    const val COMPRESSED_BYTES = 32 * 1024
    const val EXPANDED_BYTES = 384 * 1024
    const val FIELDS = 512
    const val GROUPS = 16
    const val SUBGROUPS = 64
    const val ENUM_CHOICES = 128
    const val NAME_BYTES = 128
    const val DESCRIPTION_BYTES = 4096
    const val STRING_VALUE_BYTES = 4096
    const val XML_DEPTH = 32
    const val XML_ELEMENTS = 12_000
    const val SCHEMA_RECORD_BYTES = 1024 * 1024
    const val VALUE_RECORD_BYTES = 3 * 1024 * 1024
    const val VALUE_PAYLOAD_BYTES = 4 + FIELDS * (STRING_VALUE_BYTES + 1)
}

enum class RefloatConfigurationCodecError {
    TRUNCATED_ENVELOPE,
    COMPRESSED_TOO_LARGE,
    EXPANDED_TOO_LARGE,
    EXPANDED_LENGTH_MISMATCH,
    DECOMPRESSION_FAILED,
    PROHIBITED_XML,
    MALFORMED_XML,
    STRUCTURAL_LIMIT,
    DUPLICATE_FIELD,
    DUPLICATE_ORDER,
    MISSING_REFERENCE,
    UNSUPPORTED_TYPE,
    UNSUPPORTED_ENCODING,
    INVALID_METADATA,
    INVALID_NUMBER,
    SIGNATURE_MISMATCH,
    TRUNCATED_VALUES,
    REMAINDER,
    MISSING_VALUE,
    EXTRA_VALUE,
    INVALID_VALUE,
}

class RefloatConfigurationCodecException(val reason: RefloatConfigurationCodecError) : Exception(reason.name)

enum class RefloatConfigurationFieldType(val wireValue: Int) {
    DOUBLE(1), INTEGER(2), STRING(3), ENUMERATION(4), BOOLEAN(5), BITFIELD(6);

    companion object {
        fun from(value: Int) = entries.firstOrNull { it.wireValue == value }
    }
}

enum class RefloatConfigurationWireType(val wireValue: Int) {
    UINT8(1), INT8(2), UINT16(3), INT16(4), UINT32(5), INT32(6), DOUBLE16(7), DOUBLE32(8), DOUBLE32_AUTO(9);

    companion object {
        fun from(value: Int) = entries.firstOrNull { it.wireValue == value }
    }
}

sealed interface RefloatConfigurationValue {
    data class Decimal(val value: Double) : RefloatConfigurationValue
    data class Integer(val value: Long) : RefloatConfigurationValue
    data class BooleanValue(val value: Boolean) : RefloatConfigurationValue
    data class StringValue(val value: String) : RefloatConfigurationValue
}

data class RefloatConfigurationField(
    val name: String,
    val longName: String,
    val type: RefloatConfigurationFieldType,
    val wireType: RefloatConfigurationWireType?,
    val defaultValue: RefloatConfigurationValue,
    val minimum: Double?,
    val maximum: Double?,
    val step: Double?,
    val precision: Int?,
    val scale: Double?,
    val suffix: String,
    val choices: List<String>,
    val description: String,
)

data class RefloatConfigurationSubgroup(val name: String, val fields: List<String>)
data class RefloatConfigurationGroup(val name: String, val subgroups: List<RefloatConfigurationSubgroup>)
data class RefloatConfigurationSchema(
    val fingerprint: String,
    val wireSignature: Long,
    val fields: List<RefloatConfigurationField>,
    val groups: List<RefloatConfigurationGroup>,
) {
    internal var canonicalRecord: ByteArray? = null
}
data class RefloatConfigurationSchemaIdentity(
    val packageName: String,
    val packageVersion: String,
    val fingerprint: String,
    val wireSignature: Long,
)
data class RefloatConfigurationPage(
    val identity: RefloatConfigurationSchemaIdentity,
    val group: String,
    val subgroup: String,
    val fields: List<RefloatConfigurationField>,
    val nextCursor: String?,
)
data class RefloatConfigurationValues(
    val revision: Long,
    val source: String,
    val values: Map<String, RefloatConfigurationValue>,
)
enum class RefloatConfigurationOperation { SCHEMA, READ_CURRENT, READ_DEFAULT, WRITE, APPLY_TUNE }
enum class RefloatConfigurationAvailability { LOADING, READY, UNAVAILABLE }

internal const val REFLOAT_CONFIGURATION_CODEC_ABI_VERSION = 1

internal object RustRefloatConfigurationCodecBridge {
    private const val HOST_LIBRARY_PROPERTY = "floatViewRefloatConfigurationCodecJniLibrary"
    private const val LIBRARY_NAME = "float_view_refloat_configuration_codec_android_jni"

    init {
        val explicitHostLibrary = System.getProperty(HOST_LIBRARY_PROPERTY)
        if (explicitHostLibrary == null) {
            System.loadLibrary(LIBRARY_NAME)
        } else {
            val library = File(explicitHostLibrary)
            require(library.isAbsolute) { "$HOST_LIBRARY_PROPERTY must be an absolute path" }
            System.load(library.canonicalPath)
        }
    }

    external fun abiVersion(): Int
    external fun decodeSchema(compressed: ByteArray?, output: ByteArray?, result: IntArray?): Int
    external fun decodeValues(schemaRecord: ByteArray?, payload: ByteArray?, output: ByteArray?, result: IntArray?): Int
    external fun encodeValues(schemaRecord: ByteArray?, valueRecord: ByteArray?, output: ByteArray?, result: IntArray?): Int
}

private object BridgeStatus {
    const val OK = 0
    const val INVALID_ARGUMENT = 1
    const val INVALID_SCHEMA_RECORD = 2
    const val INVALID_VALUE_RECORD = 3
    const val OUTPUT_TOO_SMALL = 4
    const val INTERNAL_FAILURE = 5
    const val TRUNCATED_ENVELOPE = 10
    const val COMPRESSED_TOO_LARGE = 11
    const val EXPANDED_TOO_LARGE = 12
    const val EXPANDED_LENGTH_MISMATCH = 13
    const val DECOMPRESSION_FAILED = 14
    const val PROHIBITED_XML = 15
    const val MALFORMED_XML = 16
    const val STRUCTURAL_LIMIT = 17
    const val DUPLICATE_FIELD = 18
    const val DUPLICATE_ORDER = 19
    const val MISSING_REFERENCE = 20
    const val UNSUPPORTED_TYPE = 21
    const val UNSUPPORTED_ENCODING = 22
    const val INVALID_METADATA = 23
    const val INVALID_NUMBER = 24
    const val SIGNATURE_MISMATCH = 25
    const val TRUNCATED_VALUES = 26
    const val REMAINDER = 27
    const val MISSING_VALUE = 28
    const val EXTRA_VALUE = 29
    const val INVALID_VALUE = 30
}

private object CanonicalRecord {
    const val VERSION = 1
    const val SCHEMA_KIND = 1
    const val VALUES_KIND = 2
    const val SCHEMA_HEADER_BYTES = 48
    const val VALUES_HEADER_BYTES = 12
    const val DECIMAL_KIND = 1
    const val INTEGER_KIND = 2
    const val BOOLEAN_KIND = 3
    const val STRING_KIND = 4
}

private class CanonicalReader(private val bytes: ByteArray) {
    private var offset = 0

    fun header(kind: Int, minimumBytes: Int) {
        if (bytes.size < minimumBytes || !take(4).contentEquals(byteArrayOf(1, kind.toByte(), 0, 0))) fail()
        if (uint32() != bytes.size.toLong()) fail()
    }

    fun take(count: Int): ByteArray {
        if (count < 0 || count > bytes.size - offset) fail()
        return bytes.copyOfRange(offset, offset + count).also { offset += count }
    }

    fun uint8(): Int = take(1)[0].toInt() and 0xff
    fun uint16(): Int = (uint8() shl 8) or uint8()
    fun uint32(): Long = (0 until 4).fold(0L) { value, _ -> (value shl 8) or uint8().toLong() }
    fun int32(): Int = uint32().toInt()
    fun int64(): Long = (0 until 8).fold(0L) { value, _ -> (value shl 8) or uint8().toLong() }

    fun double(): Double = Double.fromBits(int64()).also {
        if (!it.isFinite()) throw RefloatConfigurationCodecException(RefloatConfigurationCodecError.INVALID_NUMBER)
    }

    fun string(maximum: Int, allowEmpty: Boolean): String {
        val count = uint32()
        if (count > maximum.toLong() || count > Int.MAX_VALUE || (!allowEmpty && count == 0L)) fail()
        val contents = take(count.toInt())
        if (contents.any { it == 0.toByte() }) fail()
        val decoder = Charsets.UTF_8.newDecoder()
            .onMalformedInput(CodingErrorAction.REPORT)
            .onUnmappableCharacter(CodingErrorAction.REPORT)
        return try {
            decoder.decode(ByteBuffer.wrap(contents)).toString()
        } catch (_: Exception) {
            fail()
        }
    }

    fun value(kind: Int): RefloatConfigurationValue = when (kind) {
        CanonicalRecord.DECIMAL_KIND -> RefloatConfigurationValue.Decimal(double())
        CanonicalRecord.INTEGER_KIND -> RefloatConfigurationValue.Integer(int64())
        CanonicalRecord.BOOLEAN_KIND -> when (uint8()) {
            0 -> RefloatConfigurationValue.BooleanValue(false)
            1 -> RefloatConfigurationValue.BooleanValue(true)
            else -> throw RefloatConfigurationCodecException(RefloatConfigurationCodecError.INVALID_VALUE)
        }
        CanonicalRecord.STRING_KIND -> RefloatConfigurationValue.StringValue(
            string(RefloatConfigurationLimits.STRING_VALUE_BYTES, true),
        )
        else -> throw RefloatConfigurationCodecException(RefloatConfigurationCodecError.INVALID_VALUE)
    }

    fun finished() {
        if (offset != bytes.size) throw RefloatConfigurationCodecException(RefloatConfigurationCodecError.REMAINDER)
    }

    private fun fail(): Nothing = throw RefloatConfigurationCodecException(RefloatConfigurationCodecError.INVALID_METADATA)
}

private class CanonicalWriter {
    private val output = ByteArrayOutputStream()

    fun uint8(value: Int) {
        output.write(value and 0xff)
    }

    fun uint16(value: Int) {
        uint8(value ushr 8)
        uint8(value)
    }

    fun uint32(value: Long) {
        for (shift in 24 downTo 0 step 8) uint8((value ushr shift).toInt())
    }

    fun int64(value: Long) {
        for (shift in 56 downTo 0 step 8) uint8((value ushr shift).toInt())
    }

    fun string(value: String, maximum: Int, allowEmpty: Boolean) {
        val bytes = value.toByteArray(Charsets.UTF_8)
        if (bytes.size > maximum || (!allowEmpty && bytes.isEmpty()) || bytes.any { it == 0.toByte() }) {
            throw RefloatConfigurationCodecException(RefloatConfigurationCodecError.INVALID_VALUE)
        }
        uint32(bytes.size.toLong())
        output.write(bytes)
    }

    fun value(value: RefloatConfigurationValue) {
        when (value) {
            is RefloatConfigurationValue.Decimal -> {
                if (!value.value.isFinite()) {
                    throw RefloatConfigurationCodecException(RefloatConfigurationCodecError.INVALID_NUMBER)
                }
                uint8(CanonicalRecord.DECIMAL_KIND)
                repeat(3) { uint8(0) }
                int64(value.value.toBits())
            }
            is RefloatConfigurationValue.Integer -> {
                uint8(CanonicalRecord.INTEGER_KIND)
                repeat(3) { uint8(0) }
                int64(value.value)
            }
            is RefloatConfigurationValue.BooleanValue -> {
                uint8(CanonicalRecord.BOOLEAN_KIND)
                repeat(3) { uint8(0) }
                uint8(if (value.value) 1 else 0)
            }
            is RefloatConfigurationValue.StringValue -> {
                uint8(CanonicalRecord.STRING_KIND)
                repeat(3) { uint8(0) }
                string(value.value, RefloatConfigurationLimits.STRING_VALUE_BYTES, true)
            }
        }
    }

    fun bytes(): ByteArray = output.toByteArray()
}

object RefloatConfigurationSchemaCodec {
    internal fun requireCompatibleABI(actual: Int) {
        if (actual != REFLOAT_CONFIGURATION_CODEC_ABI_VERSION) fail(RefloatConfigurationCodecError.INVALID_METADATA)
    }

    fun decodeQtCompressed(input: ByteArray): RefloatConfigurationSchema {
        requireABI()
        val record = exactOutput(RefloatConfigurationLimits.SCHEMA_RECORD_BYTES) { output, result ->
            RustRefloatConfigurationCodecBridge.decodeSchema(input, output, result)
        }
        return schema(record)
    }

    internal fun schema(record: ByteArray): RefloatConfigurationSchema {
        val reader = CanonicalReader(record)
        reader.header(CanonicalRecord.SCHEMA_KIND, CanonicalRecord.SCHEMA_HEADER_BYTES)
        val fieldCount = reader.uint16()
        val groupCount = reader.uint16()
        if (fieldCount > RefloatConfigurationLimits.FIELDS || groupCount > RefloatConfigurationLimits.GROUPS) {
            fail(RefloatConfigurationCodecError.STRUCTURAL_LIMIT)
        }
        val signature = reader.uint32()
        val fingerprint = reader.take(32).joinToString("") { "%02x".format(it.toInt() and 0xff) }
        val fields = ArrayList<RefloatConfigurationField>(fieldCount)
        val fieldNames = HashSet<String>(fieldCount)
        repeat(fieldCount) {
            val name = reader.string(RefloatConfigurationLimits.NAME_BYTES, false)
            if (!fieldNames.add(name)) fail(RefloatConfigurationCodecError.DUPLICATE_FIELD)
            val longName = reader.string(RefloatConfigurationLimits.DESCRIPTION_BYTES, true)
            val type = RefloatConfigurationFieldType.from(reader.uint8())
                ?: fail(RefloatConfigurationCodecError.UNSUPPORTED_TYPE)
            val rawWire = reader.uint8()
            val wireType = if (rawWire == 0) null else RefloatConfigurationWireType.from(rawWire)
                ?: fail(RefloatConfigurationCodecError.UNSUPPORTED_ENCODING)
            val defaultKind = reader.uint8()
            val flags = reader.uint8()
            if (flags and 0xe0 != 0) fail(RefloatConfigurationCodecError.INVALID_METADATA)
            val defaultValue = reader.value(defaultKind)
            if (!valueMatches(defaultValue, type)) fail(RefloatConfigurationCodecError.INVALID_METADATA)
            val minimum = if (flags and 1 == 0) null else reader.double()
            val maximum = if (flags and 2 == 0) null else reader.double()
            val step = if (flags and 4 == 0) null else reader.double()
            val precision = if (flags and 8 == 0) null else reader.int32()
            val scale = if (flags and 16 == 0) null else reader.double()
            if (minimum != null && maximum != null && minimum > maximum) fail(RefloatConfigurationCodecError.INVALID_METADATA)
            when (type) {
                RefloatConfigurationFieldType.DOUBLE -> if (
                    wireType !in setOf(RefloatConfigurationWireType.DOUBLE16, RefloatConfigurationWireType.DOUBLE32) ||
                    scale == null || scale <= 0.0
                ) fail(RefloatConfigurationCodecError.UNSUPPORTED_ENCODING)
                RefloatConfigurationFieldType.INTEGER -> if (wireType !in setOf(
                        RefloatConfigurationWireType.UINT8,
                        RefloatConfigurationWireType.INT8,
                        RefloatConfigurationWireType.UINT16,
                        RefloatConfigurationWireType.INT16,
                        RefloatConfigurationWireType.UINT32,
                        RefloatConfigurationWireType.INT32,
                    )) fail(RefloatConfigurationCodecError.UNSUPPORTED_ENCODING)
                else -> Unit
            }
            val suffix = reader.string(RefloatConfigurationLimits.DESCRIPTION_BYTES, true)
            val choiceCount = reader.uint16()
            if (choiceCount > RefloatConfigurationLimits.ENUM_CHOICES || reader.uint16() != 0) {
                fail(RefloatConfigurationCodecError.STRUCTURAL_LIMIT)
            }
            val choices = List(choiceCount) { reader.string(RefloatConfigurationLimits.DESCRIPTION_BYTES, true) }
            val description = reader.string(RefloatConfigurationLimits.DESCRIPTION_BYTES, false)
            fields += RefloatConfigurationField(
                name,
                longName,
                type,
                wireType,
                defaultValue,
                minimum,
                maximum,
                step,
                precision,
                scale,
                suffix,
                choices,
                description,
            )
        }
        val groups = ArrayList<RefloatConfigurationGroup>(groupCount)
        var subgroupTotal = 0
        repeat(groupCount) {
            val name = reader.string(RefloatConfigurationLimits.DESCRIPTION_BYTES, true)
            val subgroupCount = reader.uint16()
            if (reader.uint16() != 0) fail(RefloatConfigurationCodecError.INVALID_METADATA)
            subgroupTotal += subgroupCount
            if (subgroupTotal > RefloatConfigurationLimits.SUBGROUPS) fail(RefloatConfigurationCodecError.STRUCTURAL_LIMIT)
            val subgroups = ArrayList<RefloatConfigurationSubgroup>(subgroupCount)
            repeat(subgroupCount) {
                val subgroupName = reader.string(RefloatConfigurationLimits.DESCRIPTION_BYTES, true)
                val referenceCount = reader.uint16()
                if (referenceCount > RefloatConfigurationLimits.FIELDS || reader.uint16() != 0) {
                    fail(RefloatConfigurationCodecError.STRUCTURAL_LIMIT)
                }
                val references = List(referenceCount) {
                    reader.string(RefloatConfigurationLimits.DESCRIPTION_BYTES, true)
                }
                subgroups += RefloatConfigurationSubgroup(subgroupName, references)
            }
            groups += RefloatConfigurationGroup(name, subgroups)
        }
        reader.finished()
        return RefloatConfigurationSchema(fingerprint, signature, fields, groups).also {
            it.canonicalRecord = record.copyOf()
        }
    }
}

object RefloatConfigurationValueCodec {
    fun decode(data: ByteArray, schema: RefloatConfigurationSchema): Map<String, RefloatConfigurationValue> {
        val schemaRecord = schema.canonicalRecord ?: fail(RefloatConfigurationCodecError.INVALID_METADATA)
        requireABI()
        val record = exactOutput(RefloatConfigurationLimits.VALUE_RECORD_BYTES) { output, result ->
            RustRefloatConfigurationCodecBridge.decodeValues(schemaRecord, data, output, result)
        }
        return values(record, schema)
    }

    fun encode(values: Map<String, RefloatConfigurationValue>, schema: RefloatConfigurationSchema): ByteArray {
        val schemaRecord = schema.canonicalRecord ?: fail(RefloatConfigurationCodecError.INVALID_METADATA)
        requireABI()
        val valueRecord = record(values)
        return exactOutput(RefloatConfigurationLimits.VALUE_PAYLOAD_BYTES) { output, result ->
            RustRefloatConfigurationCodecBridge.encodeValues(schemaRecord, valueRecord, output, result)
        }
    }

    internal fun values(record: ByteArray, schema: RefloatConfigurationSchema): Map<String, RefloatConfigurationValue> {
        val reader = CanonicalReader(record)
        reader.header(CanonicalRecord.VALUES_KIND, CanonicalRecord.VALUES_HEADER_BYTES)
        val count = reader.uint16()
        if (count > RefloatConfigurationLimits.FIELDS || reader.uint16() != 0) {
            fail(RefloatConfigurationCodecError.STRUCTURAL_LIMIT)
        }
        val expected = schema.fields.associateBy { it.name }
        val result = linkedMapOf<String, RefloatConfigurationValue>()
        repeat(count) {
            val name = reader.string(RefloatConfigurationLimits.NAME_BYTES, false)
            val field = expected[name] ?: fail(RefloatConfigurationCodecError.EXTRA_VALUE)
            val kind = reader.uint8()
            if (reader.take(3).any { it != 0.toByte() } || result.containsKey(name)) {
                fail(RefloatConfigurationCodecError.INVALID_METADATA)
            }
            val value = reader.value(kind)
            if (!valueMatches(value, field.type)) fail(RefloatConfigurationCodecError.INVALID_VALUE)
            result[name] = value
        }
        reader.finished()
        if (!result.keys.containsAll(expected.keys)) fail(RefloatConfigurationCodecError.MISSING_VALUE)
        if (!expected.keys.containsAll(result.keys)) fail(RefloatConfigurationCodecError.EXTRA_VALUE)
        return result
    }

    internal fun record(values: Map<String, RefloatConfigurationValue>): ByteArray {
        if (values.size > RefloatConfigurationLimits.FIELDS || values.size > 0xffff) {
            fail(RefloatConfigurationCodecError.STRUCTURAL_LIMIT)
        }
        val writer = CanonicalWriter()
        writer.uint8(CanonicalRecord.VERSION)
        writer.uint8(CanonicalRecord.VALUES_KIND)
        writer.uint16(0)
        writer.uint32(0)
        writer.uint16(values.size)
        writer.uint16(0)
        values.toSortedMap().forEach { (name, value) ->
            writer.string(name, RefloatConfigurationLimits.NAME_BYTES, false)
            writer.value(value)
        }
        val result = writer.bytes()
        if (result.size > RefloatConfigurationLimits.VALUE_RECORD_BYTES) {
            fail(RefloatConfigurationCodecError.STRUCTURAL_LIMIT)
        }
        val length = result.size.toLong()
        for (index in 0 until 4) result[4 + index] = (length ushr (24 - index * 8)).toByte()
        return result
    }
}

private fun valueMatches(value: RefloatConfigurationValue, type: RefloatConfigurationFieldType): Boolean = when (type) {
    RefloatConfigurationFieldType.DOUBLE -> value is RefloatConfigurationValue.Decimal
    RefloatConfigurationFieldType.INTEGER,
    RefloatConfigurationFieldType.ENUMERATION,
    RefloatConfigurationFieldType.BITFIELD,
    -> value is RefloatConfigurationValue.Integer
    RefloatConfigurationFieldType.BOOLEAN -> value is RefloatConfigurationValue.BooleanValue
    RefloatConfigurationFieldType.STRING -> value is RefloatConfigurationValue.StringValue
}

private fun requireABI() {
    RefloatConfigurationSchemaCodec.requireCompatibleABI(RustRefloatConfigurationCodecBridge.abiVersion())
}

private inline fun exactOutput(maximum: Int, operation: (ByteArray, IntArray) -> Int): ByteArray {
    val result = IntArray(1)
    val firstStatus = operation(ByteArray(0), result)
    if (firstStatus != BridgeStatus.OUTPUT_TOO_SMALL) checkStatus(firstStatus)
    val required = result[0]
    if (required <= 0 || required > maximum) fail(RefloatConfigurationCodecError.INVALID_METADATA)
    val output = ByteArray(required)
    result[0] = 0
    checkStatus(operation(output, result))
    if (result[0] != output.size) fail(RefloatConfigurationCodecError.INVALID_METADATA)
    return output
}

internal fun checkRefloatConfigurationBridgeStatus(status: Int) = checkStatus(status)

private fun checkStatus(status: Int) {
    val error = when (status) {
        BridgeStatus.OK -> return
        BridgeStatus.TRUNCATED_ENVELOPE -> RefloatConfigurationCodecError.TRUNCATED_ENVELOPE
        BridgeStatus.COMPRESSED_TOO_LARGE -> RefloatConfigurationCodecError.COMPRESSED_TOO_LARGE
        BridgeStatus.EXPANDED_TOO_LARGE -> RefloatConfigurationCodecError.EXPANDED_TOO_LARGE
        BridgeStatus.EXPANDED_LENGTH_MISMATCH -> RefloatConfigurationCodecError.EXPANDED_LENGTH_MISMATCH
        BridgeStatus.DECOMPRESSION_FAILED -> RefloatConfigurationCodecError.DECOMPRESSION_FAILED
        BridgeStatus.PROHIBITED_XML -> RefloatConfigurationCodecError.PROHIBITED_XML
        BridgeStatus.MALFORMED_XML -> RefloatConfigurationCodecError.MALFORMED_XML
        BridgeStatus.STRUCTURAL_LIMIT -> RefloatConfigurationCodecError.STRUCTURAL_LIMIT
        BridgeStatus.DUPLICATE_FIELD -> RefloatConfigurationCodecError.DUPLICATE_FIELD
        BridgeStatus.DUPLICATE_ORDER -> RefloatConfigurationCodecError.DUPLICATE_ORDER
        BridgeStatus.MISSING_REFERENCE -> RefloatConfigurationCodecError.MISSING_REFERENCE
        BridgeStatus.UNSUPPORTED_TYPE -> RefloatConfigurationCodecError.UNSUPPORTED_TYPE
        BridgeStatus.UNSUPPORTED_ENCODING -> RefloatConfigurationCodecError.UNSUPPORTED_ENCODING
        BridgeStatus.INVALID_NUMBER -> RefloatConfigurationCodecError.INVALID_NUMBER
        BridgeStatus.SIGNATURE_MISMATCH -> RefloatConfigurationCodecError.SIGNATURE_MISMATCH
        BridgeStatus.TRUNCATED_VALUES -> RefloatConfigurationCodecError.TRUNCATED_VALUES
        BridgeStatus.REMAINDER -> RefloatConfigurationCodecError.REMAINDER
        BridgeStatus.MISSING_VALUE -> RefloatConfigurationCodecError.MISSING_VALUE
        BridgeStatus.EXTRA_VALUE -> RefloatConfigurationCodecError.EXTRA_VALUE
        BridgeStatus.INVALID_VALUE -> RefloatConfigurationCodecError.INVALID_VALUE
        BridgeStatus.INVALID_ARGUMENT,
        BridgeStatus.INVALID_SCHEMA_RECORD,
        BridgeStatus.INVALID_VALUE_RECORD,
        BridgeStatus.OUTPUT_TOO_SMALL,
        BridgeStatus.INTERNAL_FAILURE,
        BridgeStatus.INVALID_METADATA,
        -> RefloatConfigurationCodecError.INVALID_METADATA
        else -> RefloatConfigurationCodecError.INVALID_METADATA
    }
    fail(error)
}

private fun fail(error: RefloatConfigurationCodecError): Nothing = throw RefloatConfigurationCodecException(error)
