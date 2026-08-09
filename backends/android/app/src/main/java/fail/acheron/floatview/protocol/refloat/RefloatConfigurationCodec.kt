package fail.acheron.floatview.protocol.refloat

import org.xml.sax.Attributes
import org.xml.sax.helpers.DefaultHandler
import java.io.ByteArrayInputStream
import java.nio.charset.CodingErrorAction
import java.security.MessageDigest
import java.util.zip.DataFormatException
import java.util.zip.Inflater
import javax.xml.parsers.SAXParserFactory
import kotlin.math.round

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

    companion object { fun from(value: Int) = entries.firstOrNull { it.wireValue == value } }
}

enum class RefloatConfigurationWireType(val wireValue: Int) {
    UINT8(1), INT8(2), UINT16(3), INT16(4), UINT32(5), INT32(6), DOUBLE16(7), DOUBLE32(8), DOUBLE32_AUTO(9);

    companion object { fun from(value: Int) = entries.firstOrNull { it.wireValue == value } }
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
)
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

private data class RawField(val values: MutableMap<String, MutableList<String>> = linkedMapOf()) {
    fun first(name: String, fallback: String = "") = values[name]?.firstOrNull() ?: fallback
}

private class ConfigParamsHandler : DefaultHandler() {
    val fields = mutableListOf<Pair<String, RawField>>()
    val order = mutableListOf<String>()
    val groups = mutableListOf<RefloatConfigurationGroup>()
    private val stack = mutableListOf<String>()
    private var contents = StringBuilder()
    private var elementCount = 0
    private var currentFieldName: String? = null
    private var currentField = RawField()
    private var currentGroupName: String? = null
    private var currentSubgroups = mutableListOf<RefloatConfigurationSubgroup>()
    private var currentSubgroupName: String? = null
    private var currentSubgroupFields = mutableListOf<String>()

    override fun startElement(uri: String?, localName: String?, qName: String, attributes: Attributes?) {
        elementCount += 1
        stack += qName
        contents = StringBuilder()
        if (stack.size > RefloatConfigurationLimits.XML_DEPTH || elementCount > RefloatConfigurationLimits.XML_ELEMENTS) {
            fail(RefloatConfigurationCodecError.STRUCTURAL_LIMIT)
        }
        if (stack.size == 3 && stack[0] == "ConfigParams" && stack[1] == "Params") {
            currentFieldName = qName
            currentField = RawField()
        } else if (stack.takeLast(2) == listOf("Grouping", "group")) {
            currentGroupName = null
            currentSubgroups = mutableListOf()
        } else if (stack.takeLast(2) == listOf("group", "subgroup")) {
            currentSubgroupName = null
            currentSubgroupFields = mutableListOf()
        }
    }

    override fun characters(ch: CharArray, start: Int, length: Int) {
        if (contents.length + length > RefloatConfigurationLimits.DESCRIPTION_BYTES * 2) {
            fail(RefloatConfigurationCodecError.STRUCTURAL_LIMIT)
        }
        contents.append(ch, start, length)
    }

    override fun endElement(uri: String?, localName: String?, qName: String) {
        val value = contents.toString()
        when {
            stack.size == 4 && stack[0] == "ConfigParams" && stack[1] == "Params" ->
                currentField.values.getOrPut(qName) { mutableListOf() } += value
            stack == listOf("ConfigParams", "SerOrder", "ser") -> order += value
            stack.takeLast(3) == listOf("group", "subgroup", "subgroupName") -> currentSubgroupName = value
            stack.takeLast(2) == listOf("subgroupParams", "param") -> currentSubgroupFields += value
            stack.takeLast(2) == listOf("group", "groupName") -> currentGroupName = value
        }
        when {
            stack.size == 3 && stack[0] == "ConfigParams" && stack[1] == "Params" -> {
                fields += (currentFieldName ?: qName) to currentField
                currentFieldName = null
            }
            stack.takeLast(2) == listOf("group", "subgroup") ->
                currentSubgroups += RefloatConfigurationSubgroup(currentSubgroupName ?: "", currentSubgroupFields.toList())
            stack.takeLast(2) == listOf("Grouping", "group") ->
                groups += RefloatConfigurationGroup(currentGroupName ?: "", currentSubgroups.toList())
        }
        stack.removeAt(stack.lastIndex)
        contents = StringBuilder()
    }

    private fun fail(error: RefloatConfigurationCodecError): Nothing = throw RefloatConfigurationCodecException(error)
}

object RefloatConfigurationSchemaCodec {
    fun decodeQtCompressed(input: ByteArray): RefloatConfigurationSchema {
        if (input.size < 4) fail(RefloatConfigurationCodecError.TRUNCATED_ENVELOPE)
        if (input.size > RefloatConfigurationLimits.COMPRESSED_BYTES) fail(RefloatConfigurationCodecError.COMPRESSED_TOO_LARGE)
        val expected = ((input[0].toInt() and 0xff) shl 24) or ((input[1].toInt() and 0xff) shl 16) or
            ((input[2].toInt() and 0xff) shl 8) or (input[3].toInt() and 0xff)
        if (expected < 0 || expected > RefloatConfigurationLimits.EXPANDED_BYTES) fail(RefloatConfigurationCodecError.EXPANDED_TOO_LARGE)
        val output = ByteArray(expected)
        val inflater = Inflater()
        try {
            inflater.setInput(input, 4, input.size - 4)
            val actual = inflater.inflate(output)
            if (!inflater.finished()) fail(RefloatConfigurationCodecError.DECOMPRESSION_FAILED)
            if (actual != expected || inflater.remaining != 0) fail(RefloatConfigurationCodecError.EXPANDED_LENGTH_MISMATCH)
        } catch (_: DataFormatException) {
            fail(RefloatConfigurationCodecError.DECOMPRESSION_FAILED)
        } finally {
            inflater.end()
        }
        return parse(output)
    }

    fun parse(xml: ByteArray): RefloatConfigurationSchema {
        if (xml.size > RefloatConfigurationLimits.EXPANDED_BYTES) fail(RefloatConfigurationCodecError.EXPANDED_TOO_LARGE)
        val lowered = xml.decodeToString().lowercase()
        if ("<!doctype" in lowered || "<!entity" in lowered) fail(RefloatConfigurationCodecError.PROHIBITED_XML)
        val handler = ConfigParamsHandler()
        try {
            val factory = SAXParserFactory.newInstance()
            factory.isNamespaceAware = false
            factory.setFeature("http://apache.org/xml/features/disallow-doctype-decl", true)
            factory.setFeature("http://xml.org/sax/features/external-general-entities", false)
            factory.setFeature("http://xml.org/sax/features/external-parameter-entities", false)
            factory.newSAXParser().parse(ByteArrayInputStream(xml), handler)
        } catch (error: RefloatConfigurationCodecException) {
            throw error
        } catch (error: Exception) {
            var cause: Throwable? = error.cause
            while (cause != null) {
                if (cause is RefloatConfigurationCodecException) throw cause
                cause = cause.cause
            }
            fail(RefloatConfigurationCodecError.MALFORMED_XML)
        }
        if (handler.fields.size > RefloatConfigurationLimits.FIELDS || handler.groups.size > RefloatConfigurationLimits.GROUPS ||
            handler.groups.sumOf { it.subgroups.size } > RefloatConfigurationLimits.SUBGROUPS
        ) fail(RefloatConfigurationCodecError.STRUCTURAL_LIMIT)
        val rawByName = linkedMapOf<String, RawField>()
        handler.fields.forEach { (name, field) ->
            if (rawByName.put(name, field) != null) fail(RefloatConfigurationCodecError.DUPLICATE_FIELD)
        }
        if (handler.order.toSet().size != handler.order.size) fail(RefloatConfigurationCodecError.DUPLICATE_ORDER)
        val fields = handler.order.map { name -> normalizedField(name, rawByName[name] ?: fail(RefloatConfigurationCodecError.MISSING_REFERENCE)) }
        val digest = MessageDigest.getInstance("SHA-256").digest(xml).joinToString("") { "%02x".format(it) }
        return RefloatConfigurationSchema(digest, RefloatConfigurationValueCodec.signature(fields), fields, handler.groups)
    }

    private fun normalizedField(name: String, raw: RawField): RefloatConfigurationField {
        bounded(name, RefloatConfigurationLimits.NAME_BYTES)
        val type = raw.first("type").toIntOrNull()?.let(RefloatConfigurationFieldType::from)
            ?: fail(RefloatConfigurationCodecError.UNSUPPORTED_TYPE)
        val wireType = raw.values["vTx"]?.firstOrNull()?.toIntOrNull()?.let(RefloatConfigurationWireType::from)
        if ((type == RefloatConfigurationFieldType.DOUBLE || type == RefloatConfigurationFieldType.INTEGER) && wireType == null) {
            fail(RefloatConfigurationCodecError.UNSUPPORTED_ENCODING)
        }
        val choices = raw.values["enumNames"]?.toList() ?: emptyList()
        if (choices.size > RefloatConfigurationLimits.ENUM_CHOICES) fail(RefloatConfigurationCodecError.STRUCTURAL_LIMIT)
        val default = when (type) {
            RefloatConfigurationFieldType.DOUBLE -> RefloatConfigurationValue.Decimal(finite(raw.first("valDouble")) ?: fail(RefloatConfigurationCodecError.INVALID_NUMBER))
            RefloatConfigurationFieldType.INTEGER, RefloatConfigurationFieldType.ENUMERATION, RefloatConfigurationFieldType.BITFIELD ->
                RefloatConfigurationValue.Integer(raw.first("valInt").toLongOrNull() ?: fail(RefloatConfigurationCodecError.INVALID_NUMBER))
            RefloatConfigurationFieldType.BOOLEAN -> {
                val value = raw.first("valInt").toIntOrNull()
                if (value != 0 && value != 1) fail(RefloatConfigurationCodecError.INVALID_NUMBER)
                RefloatConfigurationValue.BooleanValue(value == 1)
            }
            RefloatConfigurationFieldType.STRING -> RefloatConfigurationValue.StringValue(raw.first("valString"))
        }
        val description = sanitize(raw.first("description"))
        bounded(description, RefloatConfigurationLimits.DESCRIPTION_BYTES)
        return RefloatConfigurationField(
            name, raw.first("longName"), type, wireType, default,
            optionalFinite(if (type == RefloatConfigurationFieldType.DOUBLE) raw.first("minDouble") else raw.first("minInt")),
            optionalFinite(if (type == RefloatConfigurationFieldType.DOUBLE) raw.first("maxDouble") else raw.first("maxInt")),
            optionalFinite(if (type == RefloatConfigurationFieldType.DOUBLE) raw.first("stepDouble") else raw.first("stepInt")),
            raw.first("editorDecimalsDouble").toIntOrNull(), optionalFinite(raw.first("vTxDoubleScale")),
            raw.first("suffix"), choices, description,
        )
    }

    private fun finite(value: String): Double? = value.toDoubleOrNull()?.takeIf { it.isFinite() }
    private fun optionalFinite(value: String): Double? = if (value.isEmpty()) null else finite(value) ?: fail(RefloatConfigurationCodecError.INVALID_NUMBER)
    private fun bounded(value: String, maximum: Int) {
        if (value.isEmpty() || value.toByteArray().size > maximum) fail(RefloatConfigurationCodecError.INVALID_METADATA)
    }
    private fun sanitize(value: String): String = value.replace(Regex("<[^>]*>"), " ").split(Regex("\\s+")).filter(String::isNotEmpty).joinToString(" ")
    private fun fail(error: RefloatConfigurationCodecError): Nothing = throw RefloatConfigurationCodecException(error)
}

private class ConfigurationByteReader(private val bytes: ByteArray) {
    var offset = 0
    val remaining get() = bytes.size - offset
    fun unsigned(count: Int): Long {
        if (count < 0 || offset + count > bytes.size) fail(RefloatConfigurationCodecError.TRUNCATED_VALUES)
        var value = 0L
        repeat(count) { value = (value shl 8) or (bytes[offset++].toLong() and 0xff) }
        return value
    }
    fun signed(count: Int): Long {
        val value = unsigned(count)
        val shift = 64 - count * 8
        return (value shl shift) shr shift
    }
    fun nullTerminated(): String {
        val end = (offset until bytes.size).firstOrNull { bytes[it].toInt() == 0 } ?: fail(RefloatConfigurationCodecError.TRUNCATED_VALUES)
        if (end - offset > RefloatConfigurationLimits.STRING_VALUE_BYTES) fail(RefloatConfigurationCodecError.INVALID_VALUE)
        val decoder = Charsets.UTF_8.newDecoder().onMalformedInput(CodingErrorAction.REPORT).onUnmappableCharacter(CodingErrorAction.REPORT)
        return try { decoder.decode(java.nio.ByteBuffer.wrap(bytes, offset, end - offset)).toString().also { offset = end + 1 } }
        catch (_: Exception) { fail(RefloatConfigurationCodecError.INVALID_VALUE) }
    }
    private fun fail(error: RefloatConfigurationCodecError): Nothing = throw RefloatConfigurationCodecException(error)
}

object RefloatConfigurationValueCodec {
    fun signature(fields: List<RefloatConfigurationField>): Long {
        val input = buildString {
            fields.forEach { field ->
                append(field.name); append(field.type.wireValue); append(field.wireType?.wireValue ?: 0); field.choices.forEach(::append)
            }
        }
        return crc32c(input.toByteArray()).toLong() and 0xffff_ffffL
    }

    fun decode(data: ByteArray, schema: RefloatConfigurationSchema): Map<String, RefloatConfigurationValue> {
        val reader = ConfigurationByteReader(data)
        if (reader.unsigned(4) != schema.wireSignature) fail(RefloatConfigurationCodecError.SIGNATURE_MISMATCH)
        val values = linkedMapOf<String, RefloatConfigurationValue>()
        schema.fields.forEach { values[it.name] = decode(it, reader) }
        if (reader.remaining != 0) fail(RefloatConfigurationCodecError.REMAINDER)
        return values
    }

    fun encode(values: Map<String, RefloatConfigurationValue>, schema: RefloatConfigurationSchema): ByteArray {
        val expected = schema.fields.map { it.name }.toSet()
        if (!values.keys.containsAll(expected)) fail(RefloatConfigurationCodecError.MISSING_VALUE)
        if (!expected.containsAll(values.keys)) fail(RefloatConfigurationCodecError.EXTRA_VALUE)
        val output = mutableListOf<Byte>()
        append(schema.wireSignature, 4, output)
        schema.fields.forEach { encode(values.getValue(it.name), it, output) }
        return output.toByteArray()
    }

    private fun decode(field: RefloatConfigurationField, reader: ConfigurationByteReader): RefloatConfigurationValue = when (field.type) {
        RefloatConfigurationFieldType.DOUBLE -> {
            val scale = field.scale?.takeIf { it > 0 } ?: fail(RefloatConfigurationCodecError.INVALID_METADATA)
            val raw = when (field.wireType) {
                RefloatConfigurationWireType.DOUBLE16 -> reader.signed(2)
                RefloatConfigurationWireType.DOUBLE32 -> reader.signed(4)
                else -> fail(RefloatConfigurationCodecError.UNSUPPORTED_ENCODING)
            }
            RefloatConfigurationValue.Decimal(raw / scale)
        }
        RefloatConfigurationFieldType.INTEGER -> RefloatConfigurationValue.Integer(when (field.wireType) {
            RefloatConfigurationWireType.UINT8 -> reader.unsigned(1)
            RefloatConfigurationWireType.INT8 -> reader.signed(1)
            RefloatConfigurationWireType.UINT16 -> reader.unsigned(2)
            RefloatConfigurationWireType.INT16 -> reader.signed(2)
            RefloatConfigurationWireType.UINT32 -> reader.unsigned(4)
            RefloatConfigurationWireType.INT32 -> reader.signed(4)
            else -> fail(RefloatConfigurationCodecError.UNSUPPORTED_ENCODING)
        })
        RefloatConfigurationFieldType.ENUMERATION -> RefloatConfigurationValue.Integer(reader.signed(1))
        RefloatConfigurationFieldType.BITFIELD -> RefloatConfigurationValue.Integer(reader.unsigned(1))
        RefloatConfigurationFieldType.BOOLEAN -> when (val raw = reader.signed(1)) {
            0L -> RefloatConfigurationValue.BooleanValue(false)
            1L -> RefloatConfigurationValue.BooleanValue(true)
            else -> fail(RefloatConfigurationCodecError.INVALID_VALUE)
        }
        RefloatConfigurationFieldType.STRING -> RefloatConfigurationValue.StringValue(reader.nullTerminated())
    }

    private fun encode(value: RefloatConfigurationValue, field: RefloatConfigurationField, output: MutableList<Byte>) {
        when {
            field.type == RefloatConfigurationFieldType.DOUBLE && value is RefloatConfigurationValue.Decimal -> {
                val scale = field.scale?.takeIf { it > 0 } ?: fail(RefloatConfigurationCodecError.INVALID_METADATA)
                if (!value.value.isFinite() || !within(value.value, field)) fail(RefloatConfigurationCodecError.INVALID_VALUE)
                val scaled = value.value * scale
                if (round(scaled) != scaled) fail(RefloatConfigurationCodecError.INVALID_VALUE)
                val raw = scaled.toLong()
                when (field.wireType) {
                    RefloatConfigurationWireType.DOUBLE16 -> if (raw in Short.MIN_VALUE..Short.MAX_VALUE) append(raw, 2, output) else fail(RefloatConfigurationCodecError.INVALID_VALUE)
                    RefloatConfigurationWireType.DOUBLE32 -> if (raw in Int.MIN_VALUE..Int.MAX_VALUE) append(raw, 4, output) else fail(RefloatConfigurationCodecError.INVALID_VALUE)
                    else -> fail(RefloatConfigurationCodecError.UNSUPPORTED_ENCODING)
                }
            }
            field.type == RefloatConfigurationFieldType.INTEGER && value is RefloatConfigurationValue.Integer -> {
                if (!within(value.value.toDouble(), field)) fail(RefloatConfigurationCodecError.INVALID_VALUE)
                val (minimum, maximum, count) = when (field.wireType) {
                    RefloatConfigurationWireType.UINT8 -> Triple(0L, 255L, 1)
                    RefloatConfigurationWireType.INT8 -> Triple(-128L, 127L, 1)
                    RefloatConfigurationWireType.UINT16 -> Triple(0L, 65_535L, 2)
                    RefloatConfigurationWireType.INT16 -> Triple(-32_768L, 32_767L, 2)
                    RefloatConfigurationWireType.UINT32 -> Triple(0L, 4_294_967_295L, 4)
                    RefloatConfigurationWireType.INT32 -> Triple(-2_147_483_648L, 2_147_483_647L, 4)
                    else -> fail(RefloatConfigurationCodecError.UNSUPPORTED_ENCODING)
                }
                if (value.value !in minimum..maximum) fail(RefloatConfigurationCodecError.INVALID_VALUE)
                append(value.value, count, output)
            }
            field.type == RefloatConfigurationFieldType.ENUMERATION && value is RefloatConfigurationValue.Integer -> {
                if (value.value < 0 || value.value >= field.choices.size || value.value > 127) fail(RefloatConfigurationCodecError.INVALID_VALUE)
                append(value.value, 1, output)
            }
            field.type == RefloatConfigurationFieldType.BITFIELD && value is RefloatConfigurationValue.Integer -> {
                if (value.value !in 0..255) fail(RefloatConfigurationCodecError.INVALID_VALUE)
                append(value.value, 1, output)
            }
            field.type == RefloatConfigurationFieldType.BOOLEAN && value is RefloatConfigurationValue.BooleanValue -> output += (if (value.value) 1 else 0).toByte()
            field.type == RefloatConfigurationFieldType.STRING && value is RefloatConfigurationValue.StringValue -> {
                val bytes = value.value.toByteArray()
                if (bytes.size > RefloatConfigurationLimits.STRING_VALUE_BYTES || bytes.any { it.toInt() == 0 }) fail(RefloatConfigurationCodecError.INVALID_VALUE)
                output += bytes.toList(); output += 0.toByte()
            }
            else -> fail(RefloatConfigurationCodecError.INVALID_VALUE)
        }
    }

    private fun within(value: Double, field: RefloatConfigurationField) =
        (field.minimum == null || value >= field.minimum) && (field.maximum == null || value <= field.maximum)
    private fun append(value: Long, count: Int, output: MutableList<Byte>) {
        for (shift in (count - 1) * 8 downTo 0 step 8) output += (value ushr shift).toByte()
    }
    private fun crc32c(data: ByteArray): Int {
        var crc = -1
        data.forEach { byte ->
            crc = crc xor (byte.toInt() and 0xff)
            repeat(8) { crc = (crc ushr 1) xor (if (crc and 1 != 0) 0x82F63B78.toInt() else 0) }
        }
        return crc.inv()
    }
    private fun fail(error: RefloatConfigurationCodecError): Nothing = throw RefloatConfigurationCodecException(error)
}
