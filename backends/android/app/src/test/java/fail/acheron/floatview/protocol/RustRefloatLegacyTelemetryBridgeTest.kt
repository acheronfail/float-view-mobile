package fail.acheron.floatview.protocol

import fail.acheron.floatview.application.AlertReason
import fail.acheron.floatview.application.FirmwareFault
import fail.acheron.floatview.application.PackageState
import fail.acheron.floatview.application.RideState
import fail.acheron.floatview.protocol.refloat.RefloatLegacyTelemetryDecoder
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Test

class RustRefloatLegacyTelemetryBridgeTest {
    @Test
    fun `raw bridge reports exact ABI and rejects a mismatched adapter version`() {
        assertEquals(1, RustRefloatLegacyTelemetryBridge.abiVersion())
        assertThrows(VescProtocolException.TruncatedPayload::class.java) {
            verifyRefloatLegacyTelemetryAbiVersion(2)
        }
    }

    @Test
    fun `raw bridge validates null aliases shapes and bounded capacities`() {
        val input = detailPayload(1)
        val output = ByteArray(264) { 0x5a }
        val unchanged = output.copyOf()

        assertEquals(1, RustRefloatLegacyTelemetryBridge.decode(null, output, IntArray(2)))
        assertEquals(1, RustRefloatLegacyTelemetryBridge.decode(input, null, IntArray(2)))
        assertEquals(1, RustRefloatLegacyTelemetryBridge.decode(input, output, null))
        assertEquals(1, RustRefloatLegacyTelemetryBridge.decode(input, output, IntArray(0)))
        assertEquals(1, RustRefloatLegacyTelemetryBridge.decode(input, output, IntArray(1)))
        assertEquals(1, RustRefloatLegacyTelemetryBridge.decode(input, output, IntArray(3)))
        assertEquals(1, RustRefloatLegacyTelemetryBridge.decode(input, input, IntArray(2)))
        assertEquals(1, RustRefloatLegacyTelemetryBridge.decode(input, ByteArray(265), IntArray(2)))
        assertEquals(3, RustRefloatLegacyTelemetryBridge.decode(ByteArray(60), output, IntArray(2)))
        assertArrayEquals(unchanged, output)
    }

    @Test
    fun `raw bridge initializes metadata and publishes records atomically`() {
        val output = ByteArray(264) { 0x5a }
        val metadata = intArrayOf(99, 99)

        assertEquals(3, RustRefloatLegacyTelemetryBridge.decode(byteArrayOf(36, 101, 10), output, metadata))
        assertArrayEquals(intArrayOf(0, 0), metadata)
        assertArrayEquals(ByteArray(264) { 0x5a }, output)

        metadata.fill(99)
        assertEquals(2, RustRefloatLegacyTelemetryBridge.decode(byteArrayOf(35, 101, 10, 1), output, metadata))
        assertArrayEquals(intArrayOf(0, 35), metadata)
        assertArrayEquals(ByteArray(264) { 0x5a }, output)

        metadata.fill(99)
        assertEquals(5, RustRefloatLegacyTelemetryBridge.decode(detailPayload(1), ByteArray(16), metadata))
        assertArrayEquals(intArrayOf(264, 0), metadata)

        metadata.fill(99)
        assertEquals(0, RustRefloatLegacyTelemetryBridge.decode(activeFault(1), output, metadata))
        assertArrayEquals(intArrayOf(16, 0), metadata)
        assertArrayEquals(
            byteArrayOf(1, 2, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0),
            output.copyOf(16),
        )
    }

    @Test
    fun `public decoder selects Rust for all details options and sentinels`() {
        (1..4).forEach { detail ->
            val decoded = RefloatLegacyTelemetryDecoder.decode(detailPayload(detail))
            assertEquals(PackageState.STARTUP, decoded.packageState)
            assertEquals(RideState.IDLE, decoded.rideState)
            assertEquals(0.0, decoded.packVoltageV!!, 0.0)
            if (detail == 1) {
                assertNull(decoded.controllerTempC)
                assertNull(decoded.distanceM)
            } else {
                assertEquals(0.0, decoded.controllerTempC!!, 0.0)
                assertEquals(0.0, decoded.distanceM!!, 0.0)
            }
            if (detail < 3) assertNull(decoded.odometerM) else assertEquals(0.0, decoded.odometerM!!, 0.0)
            if (detail < 4) assertNull(decoded.chargingVoltageV) else assertEquals(0.0, decoded.chargingVoltageV!!, 0.0)
            assertEquals(0.0, decoded.fieldWeakeningCurrentA!!, 0.0)
        }

        val unavailable = detailPayload(2).also { it[34] = 222.toByte() }
        assertNull(RefloatLegacyTelemetryDecoder.decode(unavailable).fieldWeakeningCurrentA)
    }

    @Test
    fun `public decoder preserves known unknown and structural errors`() {
        assertEquals(FirmwareFault.OVER_VOLTAGE, RefloatLegacyTelemetryDecoder.decode(activeFault(1)).firmwareFault)
        val unknown = RefloatLegacyTelemetryDecoder.decode(activeFault(255))
        assertEquals(FirmwareFault.UNKNOWN, unknown.firmwareFault)
        assertEquals(AlertReason.FIRMWARE_FAULT, unknown.alertReason)
        assertEquals(RideState.UNKNOWN, unknown.rideState)

        val unsupported = assertThrows(VescProtocolException.UnsupportedCommand::class.java) {
            RefloatLegacyTelemetryDecoder.decode(byteArrayOf(35, 101, 10, 1))
        }
        assertEquals(35, unsupported.command)
        assertThrows(VescProtocolException.TruncatedPayload::class.java) {
            RefloatLegacyTelemetryDecoder.decode(byteArrayOf(36, 101, 10))
        }
        assertThrows(VescProtocolException.InvalidFrame::class.java) {
            RefloatLegacyTelemetryDecoder.decode(byteArrayOf(36, 101, 10, 69, 0))
        }
    }

    @Test
    fun `adapter rejects impossible statuses metadata and records`() {
        val valid = bridgeRecord(detailPayload(4))
        listOf(
            Triple(0, intArrayOf(264, 1), valid),
            Triple(0, intArrayOf(265, 0), ByteArray(264)),
            Triple(2, intArrayOf(0, 0), valid),
            Triple(2, intArrayOf(1, 35), valid),
            Triple(3, intArrayOf(1, 0), valid),
            Triple(4, intArrayOf(0, 1), valid),
            Triple(99, intArrayOf(0, 0), valid),
        ).forEach { (status, result, record) ->
            assertThrows(VescProtocolException::class.java) {
                RefloatLegacyTelemetryDecoder.decodeBridgeResult(status, record, result)
            }
        }

        val malformed = mutableListOf<ByteArray>()
        malformed += valid.copyOf(15)
        malformed += valid.copyOf().also { it[0] = 2 }
        malformed += valid.copyOf().also { it[1] = 3 }
        malformed += valid.copyOf().also { it[2] = 0 }
        malformed += valid.copyOf().also { it[2] = 5 }
        malformed += valid.copyOf().also { it[8] = 8 }
        malformed += valid.copyOf().also { it[10] = 1 }
        malformed += valid.copyOf().also { it[12] = 0x10 }
        malformed += valid.copyOf().also { it[14] = 1 }
        malformed += valid.copyOf().also { it[16] = 0x7f; it[17] = 0xf8.toByte() }
        malformed += bridgeRecord(detailPayload(1)).also { it[12] = 0x04; it[13] = 0x04 }
        intArrayOf(3, 4, 5, 6, 7, 9).forEach { offset ->
            malformed += valid.copyOf().also { it[offset] = 0 }
            malformed += valid.copyOf().also { it[offset] = 0x7f }
        }
        malformed.forEach { record ->
            assertThrows(VescProtocolException.TruncatedPayload::class.java) {
                RefloatLegacyTelemetryDecoder.telemetry(record)
            }
        }

        val knownFault = bridgeRecord(activeFault(1))
        listOf(
            knownFault.copyOf().also { it[2] = 1 },
            knownFault.copyOf().also { it[10] = 28; it[11] = 1 },
            knownFault.copyOf().also { it[10] = 1; it[11] = 28 },
            knownFault.copyOf().also { it[12] = 1 },
        ).forEach { record ->
            assertThrows(VescProtocolException.TruncatedPayload::class.java) {
                RefloatLegacyTelemetryDecoder.telemetry(record)
            }
        }
    }

    private fun bridgeRecord(payload: ByteArray): ByteArray {
        val output = ByteArray(264)
        val result = IntArray(2)
        assertEquals(0, RustRefloatLegacyTelemetryBridge.decode(payload, output, result))
        return output.copyOf(result[0])
    }

    private fun activeFault(code: Int) = byteArrayOf(36, 101, 10, 69, code.toByte())

    private fun detailPayload(detail: Int): ByteArray {
        val length = when (detail) { 1 -> 35; 2 -> 42; 3 -> 55; else -> 59 }
        return ByteArray(length).also {
            it[0] = 36
            it[1] = 101
            it[2] = 10
            it[3] = detail.toByte()
            it[14] = 128.toByte()
            it[15] = 128.toByte()
            it[16] = 128.toByte()
            it[17] = 128.toByte()
            it[18] = 128.toByte()
            it[19] = 128.toByte()
            it[22] = 128.toByte()
            it[33] = 128.toByte()
        }
    }
}
