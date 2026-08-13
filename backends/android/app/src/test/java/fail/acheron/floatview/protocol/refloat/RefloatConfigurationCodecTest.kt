package fail.acheron.floatview.protocol.refloat

import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertTrue
import org.junit.Assert.assertThrows
import org.junit.Test
import java.io.File

class RefloatConfigurationCodecTest {
    private data class Expected(val version: String, val fingerprint: String, val signature: Long, val fields: Int)

    private val root = File(System.getProperty("floatViewFixtureRoot"), "refloat-configuration")
    private val expected = listOf(
        Expected("1.0.0", "324e7f5ef1ea1b2038478af7e6f43c91d63ec06a6caf2907c5b7e6c812382ad9", 0xf3d49d76L, 144),
        Expected("1.0.1", "887e7b03c03304a2f8455e71b369b89830e8d6446a73aa598ab067ac81944526", 0xf3d49d76L, 144),
        Expected("1.1.0", "b5849e38371a65721bb48182048023d1c707012228554878ce263b1c49ee168f", 0x00f1c21bL, 156),
        Expected("1.1.1", "b5849e38371a65721bb48182048023d1c707012228554878ce263b1c49ee168f", 0x00f1c21bL, 156),
        Expected("1.1.2", "b5849e38371a65721bb48182048023d1c707012228554878ce263b1c49ee168f", 0x00f1c21bL, 156),
        Expected("1.2.0", "2561e00a5b6694de0ac272bae077e06050cfc00cf4ba371bb1d5d9616e137f7e", 0x90b7a9baL, 167),
        Expected("1.2.1", "9b8979a36759810a7fa9c19d01534ed52ec9ed8ac4a2d3ab3aefc594ba3b93cc", 0x90b7a9baL, 167),
        Expected("1.2.2", "d7ca6a2c06ce5701858eebfaaf224e97e08a212d1dab60b664d268a90763eb39", 0x90b7a9baL, 167),
        Expected("1.2.3", "fa40717969d1d86b429d7c78b59144b8387b4850be83a72fd3c7cf15ffecd01c", 0x90b7a9baL, 167),
    )

    @Test fun schemasAndValuesMatchEveryAuditedRelease() {
        expected.forEach { release ->
            val schema = RefloatConfigurationSchemaCodec.decodeQtCompressed(File(root, "schemas/${release.version}.qcompress").readBytes())
            assertEquals(release.fingerprint, schema.fingerprint)
            assertEquals(release.signature, schema.wireSignature)
            assertEquals(release.fields, schema.fields.size)
            assertTrue(schema.canonicalRecord?.isNotEmpty() == true)
            val defaults = File(root, "values/${release.version}-default.bin").readBytes()
            val decoded = RefloatConfigurationValueCodec.decode(defaults, schema)
            assertEquals(release.fields, decoded.size)
            assertArrayEquals(defaults, RefloatConfigurationValueCodec.encode(decoded, schema))
        }
        assertEquals(expected[0].signature, expected[1].signature)
        assertNotEquals(expected[0].fingerprint, expected[1].fingerprint)
    }

    @Test fun rejectsEnvelopeSignatureAndRemainder() {
        assertEquals(
            RefloatConfigurationCodecError.TRUNCATED_ENVELOPE,
            assertThrows(RefloatConfigurationCodecException::class.java) {
                RefloatConfigurationSchemaCodec.decodeQtCompressed(byteArrayOf(0, 0))
            }.reason,
        )
        val schema = RefloatConfigurationSchemaCodec.decodeQtCompressed(File(root, "schemas/1.2.3.qcompress").readBytes())
        val mismatch = File(root, "values/1.2.3-default.bin").readBytes().also { it[0] = (it[0].toInt() xor 0xff).toByte() }
        assertEquals(
            RefloatConfigurationCodecError.SIGNATURE_MISMATCH,
            assertThrows(RefloatConfigurationCodecException::class.java) { RefloatConfigurationValueCodec.decode(mismatch, schema) }.reason,
        )
        val remainder = File(root, "values/1.2.3-default.bin").readBytes() + 0
        assertEquals(
            RefloatConfigurationCodecError.REMAINDER,
            assertThrows(RefloatConfigurationCodecException::class.java) { RefloatConfigurationValueCodec.decode(remainder, schema) }.reason,
        )
    }

    @Test fun bridgeUsesAtomicExactCapacityAndRejectsInvalidArrays() {
        assertEquals(REFLOAT_CONFIGURATION_CODEC_ABI_VERSION, RustRefloatConfigurationCodecBridge.abiVersion())
        val compressed = File(root, "schemas/1.2.3.qcompress").readBytes()
        val result = IntArray(1)
        assertEquals(4, RustRefloatConfigurationCodecBridge.decodeSchema(compressed, ByteArray(0), result))
        assertTrue(result[0] > 48)
        val record = ByteArray(result[0])
        result[0] = 0
        assertEquals(0, RustRefloatConfigurationCodecBridge.decodeSchema(compressed, record, result))
        assertEquals(record.size, result[0])
        assertEquals(0, RustRefloatConfigurationCodecBridge.decodeSchema(compressed, record.copyOf(), result))
        assertEquals(1, RustRefloatConfigurationCodecBridge.decodeSchema(null, ByteArray(0), result))
        assertEquals(1, RustRefloatConfigurationCodecBridge.decodeSchema(compressed, compressed, result))
        assertEquals(1, RustRefloatConfigurationCodecBridge.decodeSchema(compressed, ByteArray(0), IntArray(2)))
    }

    @Test fun strictRecordAdaptersRejectMalformedOrInconsistentRecords() {
        val schema = RefloatConfigurationSchemaCodec.decodeQtCompressed(File(root, "schemas/1.2.3.qcompress").readBytes())
        val canonical = requireNotNull(schema.canonicalRecord)
        val reserved = canonical.copyOf().also { it[2] = 1 }
        assertEquals(
            RefloatConfigurationCodecError.INVALID_METADATA,
            assertThrows(RefloatConfigurationCodecException::class.java) {
                RefloatConfigurationSchemaCodec.schema(reserved)
            }.reason,
        )
        val truncated = canonical.copyOf(canonical.size - 1)
        assertEquals(
            RefloatConfigurationCodecError.INVALID_METADATA,
            assertThrows(RefloatConfigurationCodecException::class.java) {
                RefloatConfigurationSchemaCodec.schema(truncated)
            }.reason,
        )
        val defaults = RefloatConfigurationValueCodec.decode(
            File(root, "values/1.2.3-default.bin").readBytes(),
            schema,
        )
        val valuesRecord = RefloatConfigurationValueCodec.record(defaults)
        assertEquals(defaults, RefloatConfigurationValueCodec.values(valuesRecord, schema))
        val badKind = valuesRecord.copyOf().also {
            val firstNameLength = ((it[12].toInt() and 0xff) shl 24) or
                ((it[13].toInt() and 0xff) shl 16) or
                ((it[14].toInt() and 0xff) shl 8) or (it[15].toInt() and 0xff)
            it[16 + firstNameLength] = 99
        }
        assertEquals(
            RefloatConfigurationCodecError.INVALID_VALUE,
            assertThrows(RefloatConfigurationCodecException::class.java) {
                RefloatConfigurationValueCodec.values(badKind, schema)
            }.reason,
        )
        RefloatConfigurationSchemaCodec.requireCompatibleABI(REFLOAT_CONFIGURATION_CODEC_ABI_VERSION)
        assertThrows(RefloatConfigurationCodecException::class.java) {
            RefloatConfigurationSchemaCodec.requireCompatibleABI(REFLOAT_CONFIGURATION_CODEC_ABI_VERSION + 1)
        }
        checkRefloatConfigurationBridgeStatus(0)
        for (status in listOf(1, 2, 3, 4, 5, 23, 999)) {
            assertEquals(
                RefloatConfigurationCodecError.INVALID_METADATA,
                assertThrows(RefloatConfigurationCodecException::class.java) {
                    checkRefloatConfigurationBridgeStatus(status)
                }.reason,
            )
        }
    }
}
