package fail.acheron.floatview.protocol

import fail.acheron.floatview.protocol.refloat.RefloatCompatibility
import fail.acheron.floatview.protocol.refloat.RefloatControlProfile
import fail.acheron.floatview.protocol.refloat.RefloatDynamicTelemetryDecoder
import fail.acheron.floatview.protocol.refloat.RefloatLightingProfile
import fail.acheron.floatview.protocol.refloat.RefloatTelemetryFamily
import fail.acheron.floatview.protocol.refloat.RefloatWireProfile
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test

class RustRefloatCompatibilityBridgeTest {
    @Test
    fun `raw bridge reports exact ABI and rejects mismatched adapter version`() {
        assertEquals(1, RustRefloatCompatibilityBridge.abiVersion())
        assertThrows(VescProtocolException.TruncatedPayload::class.java) {
            verifyRefloatCompatibilityAbiVersion(2)
        }
    }

    @Test
    fun `raw bridge validates null aliases shapes capacities and initializes results`() {
        val request = ByteArray(5) { 0x5a }
        val unchangedRequest = request.copyOf()
        assertEquals(1, RustRefloatCompatibilityBridge.infoRequest(null, IntArray(1)))
        assertEquals(1, RustRefloatCompatibilityBridge.infoRequest(request, null))
        assertEquals(1, RustRefloatCompatibilityBridge.infoRequest(request, IntArray(0)))
        assertEquals(1, RustRefloatCompatibilityBridge.infoRequest(request, IntArray(2)))
        assertEquals(1, RustRefloatCompatibilityBridge.infoRequest(ByteArray(6), IntArray(1)))
        assertArrayEquals(unchangedRequest, request)

        val shortRequest = ByteArray(4) { 0x5a }
        val shortResult = intArrayOf(99)
        assertEquals(7, RustRefloatCompatibilityBridge.infoRequest(shortRequest, shortResult))
        assertArrayEquals(intArrayOf(5), shortResult)
        assertArrayEquals(ByteArray(4) { 0x5a }, shortRequest)

        val payload = refloatInfoPayload("1.2.3")
        val output = ByteArray(12) { 0x5a }
        val unchangedOutput = output.copyOf()
        assertEquals(1, RustRefloatCompatibilityBridge.decodeInfo(null, output, IntArray(1)))
        assertEquals(1, RustRefloatCompatibilityBridge.decodeInfo(payload, null, IntArray(1)))
        assertEquals(1, RustRefloatCompatibilityBridge.decodeInfo(payload, output, null))
        assertEquals(1, RustRefloatCompatibilityBridge.decodeInfo(payload, output, IntArray(0)))
        assertEquals(1, RustRefloatCompatibilityBridge.decodeInfo(payload, output, IntArray(2)))
        assertEquals(1, RustRefloatCompatibilityBridge.decodeInfo(payload, payload, IntArray(1)))
        assertEquals(1, RustRefloatCompatibilityBridge.decodeInfo(payload, ByteArray(13), IntArray(1)))
        assertEquals(1, RustRefloatCompatibilityBridge.decodeInfo(ByteArray(4_097), output, IntArray(1)))
        assertArrayEquals(unchangedOutput, output)

        val truncatedResult = intArrayOf(99)
        assertEquals(3, RustRefloatCompatibilityBridge.decodeInfo(byteArrayOf(36, 101, 0), output, truncatedResult))
        assertArrayEquals(intArrayOf(0), truncatedResult)
        assertArrayEquals(unchangedOutput, output)
        assertEquals(1, RustRefloatCompatibilityBridge.admitLayout(null))
        assertEquals(6, RustRefloatCompatibilityBridge.admitLayout(ByteArray(4_169)))
    }

    @Test
    fun `raw bridge publishes exact requests decisions and layout admission`() {
        val request = ByteArray(5)
        val requestResult = IntArray(1)
        assertEquals(0, RustRefloatCompatibilityBridge.infoRequest(request, requestResult))
        assertArrayEquals(byteArrayOf(36, 101, 0, 2, 0), request)
        assertArrayEquals(intArrayOf(5), requestResult)

        val record = ByteArray(12)
        val result = IntArray(1)
        assertEquals(0, RustRefloatCompatibilityBridge.decodeInfo(refloatInfoPayload("1.2.3"), record, result))
        assertArrayEquals(byteArrayOf(1, 1, 1, 2, 1, 3, 3, 2, 2, 1, 0, 0), record)
        assertArrayEquals(intArrayOf(12), result)

        val decoder = RefloatDynamicTelemetryDecoder()
        val accepted = decoder.decodeFieldsWithRecord(layoutPayload(refloatRequiredFields, refloatRequiredRuntimeFields))
        assertEquals(0, RustRefloatCompatibilityBridge.admitLayout(accepted.record))
        val rejected = decoder.decodeFieldsWithRecord(layoutPayload(refloatRequiredFields.drop(1), refloatRequiredRuntimeFields))
        assertEquals(6, RustRefloatCompatibilityBridge.admitLayout(rejected.record))
    }

    @Test
    fun `public adapter maps every supported wire decision and fails closed`() {
        assertArrayEquals(byteArrayOf(36, 101, 0, 2, 0), RefloatCompatibility.infoRequest())
        val legacy = refloatCompatibilityDecision("1.0")
        assertEquals(RefloatWireProfile.LEGACY_FIXED, legacy.profile)
        assertEquals(RefloatTelemetryFamily.LEGACY_FIXED, legacy.family)
        assertEquals(
            RefloatControlProfile(RefloatLightingProfile.LEGACY_COMMAND_202, false),
            legacy.controlProfile,
        )
        listOf("1.1.0", "1.1.1", "1.1.2").forEach { version ->
            val decision = refloatCompatibilityDecision(version)
            assertEquals(version, decision.wireVersion)
            assertEquals(RefloatWireProfile.DYNAMIC, decision.profile)
            assertEquals(RefloatLightingProfile.LEGACY_COMMAND_202, decision.controlProfile.lighting)
            assertFalse(decision.controlProfile.fatalClearAvailable)
        }
        listOf("1.2.0", "1.2.1", "1.2.2", "1.2.3").forEach { version ->
            val decision = refloatCompatibilityDecision(version)
            assertEquals(version, decision.wireVersion)
            assertEquals(RefloatWireProfile.DYNAMIC_ALERTS, decision.profile)
            assertEquals(RefloatTelemetryFamily.DYNAMIC, decision.family)
            assertEquals(RefloatLightingProfile.STABLE_COMMAND_20, decision.controlProfile.lighting)
            assertTrue(decision.controlProfile.fatalClearAvailable)
        }
        assertThrows(VescProtocolException.InvalidFrame::class.java) {
            RefloatCompatibility.decodeInfo(refloatInfoPayload("1.2.4"))
        }
        assertThrows(VescProtocolException.InvalidFrame::class.java) {
            RefloatCompatibility.decodeInfo(byteArrayOf(36, 101, 0, 12))
        }
    }

    @Test
    fun `adapter rejects every impossible canonical record combination`() {
        val legacy = byteArrayOf(1, 1, 1, 0, 0, 0, 1, 1, 1, 0, 0, 0)
        val dynamic = byteArrayOf(1, 1, 1, 1, 1, 2, 2, 2, 1, 0, 0, 0)
        val alerts = byteArrayOf(1, 1, 1, 2, 1, 3, 3, 2, 2, 1, 0, 0)
        assertEquals("1.0", RefloatCompatibility.decision(legacy).wireVersion)
        assertEquals("1.1.2", RefloatCompatibility.decision(dynamic).wireVersion)
        assertEquals("1.2.3", RefloatCompatibility.decision(alerts).wireVersion)

        val malformed = mutableListOf<ByteArray>()
        malformed += legacy.copyOf(11)
        listOf(0 to 2, 1 to 2, 4 to 2, 6 to 0, 7 to 0, 8 to 0, 9 to 2, 10 to 1, 11 to 1).forEach { (index, value) ->
            malformed += legacy.copyOf().also { it[index] = value.toByte() }
        }
        malformed += legacy.copyOf().also { it[4] = 0; it[5] = 1 }
        for (profile in 1..3) for (family in 1..2) for (lighting in 1..2) for (fatal in 0..1) {
            val combination = listOf(profile, family, lighting, fatal)
            if (combination in listOf(listOf(1, 1, 1, 0), listOf(2, 2, 1, 0), listOf(3, 2, 2, 1))) continue
            malformed += alerts.copyOf().also {
                it[6] = profile.toByte(); it[7] = family.toByte()
                it[8] = lighting.toByte(); it[9] = fatal.toByte()
            }
        }
        malformed.forEach { record ->
            assertThrows(VescProtocolException.TruncatedPayload::class.java) {
                RefloatCompatibility.decision(record)
            }
        }
    }

    private fun layoutPayload(fields: List<String>, runtime: List<String>): ByteArray {
        val bytes = mutableListOf<Byte>(36, 101, 32, fields.size.toByte())
        fields.forEach { name ->
            val encoded = name.encodeToByteArray()
            bytes += encoded.size.toByte()
            bytes += encoded.toList()
        }
        bytes += runtime.size.toByte()
        runtime.forEach { name ->
            val encoded = name.encodeToByteArray()
            bytes += encoded.size.toByte()
            bytes += encoded.toList()
        }
        return bytes.toByteArray()
    }
}
