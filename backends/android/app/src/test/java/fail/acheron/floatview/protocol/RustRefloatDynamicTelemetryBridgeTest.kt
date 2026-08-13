package fail.acheron.floatview.protocol

import fail.acheron.floatview.application.FirmwareFault
import fail.acheron.floatview.application.FootpadState
import fail.acheron.floatview.application.PackageMode
import fail.acheron.floatview.application.PackageState
import fail.acheron.floatview.application.RideState
import fail.acheron.floatview.application.SetpointAdjustment
import fail.acheron.floatview.application.StopCondition
import fail.acheron.floatview.protocol.refloat.RefloatDynamicTelemetryDecoder
import java.nio.file.Files
import java.nio.file.Path
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Test

class RustRefloatDynamicTelemetryBridgeTest {
    @Test
    fun `raw bridge reports exact ABI and rejects a mismatched adapter version`() {
        assertEquals(1, RustRefloatDynamicTelemetryBridge.abiVersion())
        assertThrows(VescProtocolException.TruncatedPayload::class.java) {
            verifyRefloatDynamicTelemetryAbiVersion(2)
        }
    }

    @Test
    fun `raw bridge validates null aliases shapes and bounded capacities`() {
        val input = layoutPayload("motor.speed")
        val output = ByteArray(MAX_LAYOUT_RECORD_BYTES) { 0x5a }
        val unchanged = output.copyOf()

        assertEquals(1, RustRefloatDynamicTelemetryBridge.decodeLayout(null, output, IntArray(1)))
        assertEquals(1, RustRefloatDynamicTelemetryBridge.decodeLayout(input, null, IntArray(1)))
        assertEquals(1, RustRefloatDynamicTelemetryBridge.decodeLayout(input, output, null))
        assertEquals(1, RustRefloatDynamicTelemetryBridge.decodeLayout(input, output, IntArray(0)))
        assertEquals(1, RustRefloatDynamicTelemetryBridge.decodeLayout(input, output, IntArray(2)))
        assertEquals(1, RustRefloatDynamicTelemetryBridge.decodeLayout(input, input, IntArray(1)))
        assertEquals(1, RustRefloatDynamicTelemetryBridge.decodeLayout(input, ByteArray(MAX_LAYOUT_RECORD_BYTES + 1), IntArray(1)))
        assertEquals(3, RustRefloatDynamicTelemetryBridge.decodeLayout(ByteArray(MAX_LAYOUT_PAYLOAD_BYTES + 1), output, IntArray(1)))
        assertArrayEquals(unchanged, output)

        val layout = bridgeLayout(input)
        val realtime = readyPayload()
        assertEquals(1, RustRefloatDynamicTelemetryBridge.decodeRealtime(null, realtime, ByteArray(MAX_SAMPLE_RECORD_BYTES), IntArray(1)))
        assertEquals(1, RustRefloatDynamicTelemetryBridge.decodeRealtime(layout, null, ByteArray(MAX_SAMPLE_RECORD_BYTES), IntArray(1)))
        assertEquals(1, RustRefloatDynamicTelemetryBridge.decodeRealtime(layout, realtime, null, IntArray(1)))
        assertEquals(1, RustRefloatDynamicTelemetryBridge.decodeRealtime(layout, realtime, ByteArray(MAX_SAMPLE_RECORD_BYTES), null))
        assertEquals(1, RustRefloatDynamicTelemetryBridge.decodeRealtime(layout, layout, ByteArray(MAX_SAMPLE_RECORD_BYTES), IntArray(1)))
        assertEquals(1, RustRefloatDynamicTelemetryBridge.decodeRealtime(layout, realtime, ByteArray(MAX_SAMPLE_RECORD_BYTES + 1), IntArray(1)))
        assertEquals(4, RustRefloatDynamicTelemetryBridge.decodeRealtime(ByteArray(MAX_LAYOUT_RECORD_BYTES + 1), realtime, ByteArray(MAX_SAMPLE_RECORD_BYTES), IntArray(1)))
        assertEquals(3, RustRefloatDynamicTelemetryBridge.decodeRealtime(layout, ByteArray(MAX_REALTIME_PAYLOAD_BYTES + 1), ByteArray(MAX_SAMPLE_RECORD_BYTES), IntArray(1)))
    }

    @Test
    fun `raw bridge initializes lengths and publishes exact source records atomically`() {
        val layoutOutput = ByteArray(MAX_LAYOUT_RECORD_BYTES) { 0x5a }
        val layoutResult = intArrayOf(99)
        assertEquals(3, RustRefloatDynamicTelemetryBridge.decodeLayout(byteArrayOf(36, 101, 32), layoutOutput, layoutResult))
        assertArrayEquals(intArrayOf(0), layoutResult)
        assertArrayEquals(ByteArray(MAX_LAYOUT_RECORD_BYTES) { 0x5a }, layoutOutput)

        val sourceLayout = sourcePayload("1.1.0-and-1.2.3-command-32-layout")
        layoutResult[0] = 99
        assertEquals(6, RustRefloatDynamicTelemetryBridge.decodeLayout(sourceLayout, ByteArray(2), layoutResult))
        assertEquals(409, layoutResult[0])

        val layout = bridgeLayout(sourceLayout)
        assertArrayEquals(byteArrayOf(1, 1, 16, 10, 1, 119, 0, 0), layout.copyOf(8))
        val sample = bridgeSample(layout, sourcePayload("1.2.3-command-31-running-alert-tail"))
        assertEquals(1, sample[0].unsigned())
        assertEquals(2, sample[1].unsigned())
        assertEquals(4, sample[2].unsigned())
        assertEquals(4, sample[4].unsigned())
    }

    @Test
    fun `public decoder selects Rust for source layouts realtime options and faults`() {
        val decoder = RefloatDynamicTelemetryDecoder()
        val (fields, runtimeFields) = decoder.decodeFields(sourcePayload("1.1.0-and-1.2.3-command-32-layout"))
        assertEquals(16, fields.size)
        assertEquals(10, runtimeFields.size)

        val running = decoder.decodeData(sourcePayload("1.2.3-command-31-running-alert-tail"))
        assertEquals(RideState.RIDING, running.rideState)
        assertEquals(PackageState.RUNNING, running.packageState)
        assertEquals(FootpadState.BOTH, running.footpadState)
        assertEquals(emptyMap<String, Double>(), running.extensions)

        val charging = decoder.decodeData(sourcePayload("1.2.3-command-31-charging-tail"))
        assertEquals(true, charging.charging)
        requireNotNull(charging.chargingCurrentA)
        requireNotNull(charging.chargingVoltageV)

        val fault = decoder.decodeData(sourcePayload("1.2.3-command-31-active-firmware-fault"))
        assertEquals(FirmwareFault.OVER_VOLTAGE, fault.firmwareFault)
    }

    @Test
    fun `layout lifetime resets and replaces only after complete validation`() {
        val decoder = RefloatDynamicTelemetryDecoder()
        decoder.decodeFields(layoutPayload("motor.speed"))
        assertEquals(1.0, decoder.decodeData(readyPayload()).speedKph!!, 0.0)

        assertThrows(VescProtocolException.InvalidFrame::class.java) {
            decoder.decodeFields(byteArrayOf(36, 101, 32, 0, 0))
        }
        assertEquals(1.0, decoder.decodeData(readyPayload()).speedKph!!, 0.0)

        decoder.decodeFields(layoutPayload("future.energy"))
        val replacement = decoder.decodeData(readyPayload(0x4200))
        assertNull(replacement.speedKph)
        assertEquals(3.0, replacement.extensions["future.energy"]!!, 0.0)

        decoder.reset()
        assertEquals(emptyList<String>(), decoder.fields)
        assertEquals(emptyList<String>(), decoder.runtimeFields)
        assertThrows(VescProtocolException.TruncatedPayload::class.java) {
            decoder.decodeData(readyPayload())
        }
    }

    @Test
    fun `adapter maps every canonical semantic and fixed value slot`() {
        val decoder = RefloatDynamicTelemetryDecoder()
        listOf(
            PackageState.DISABLED,
            PackageState.STARTUP,
            PackageState.READY,
            PackageState.RUNNING,
        ).forEachIndexed { index, expected ->
            assertEquals(expected, decoder.telemetry(canonicalSampleRecord().also { it[2] = (index + 1).toByte() }).packageState)
        }
        listOf(PackageMode.NORMAL, PackageMode.HAND_TEST, PackageMode.FLYWHEEL, PackageMode.UNKNOWN)
            .forEachIndexed { index, expected ->
                assertEquals(expected, decoder.telemetry(canonicalSampleRecord().also { it[3] = (index + 1).toByte() }).packageMode)
            }
        listOf(FootpadState.NONE, FootpadState.LEFT, FootpadState.RIGHT, FootpadState.BOTH)
            .forEachIndexed { index, expected ->
                assertEquals(expected, decoder.telemetry(canonicalSampleRecord().also { it[4] = (index + 1).toByte() }).footpadState)
            }
        listOf(
            StopCondition.NONE,
            StopCondition.PITCH,
            StopCondition.ROLL,
            StopCondition.SWITCH_HALF,
            StopCondition.SWITCH_FULL,
            StopCondition.REVERSE_STOP,
            StopCondition.QUICK_STOP,
            StopCondition.UNKNOWN,
        ).forEachIndexed { index, expected ->
            assertEquals(expected, decoder.telemetry(canonicalSampleRecord().also { it[5] = (index + 1).toByte() }).stopCondition)
        }
        listOf(
            SetpointAdjustment.NONE,
            SetpointAdjustment.CENTERING,
            SetpointAdjustment.REVERSE_STOP,
            SetpointAdjustment.SPEED,
            SetpointAdjustment.DUTY,
            SetpointAdjustment.ERROR,
            SetpointAdjustment.HIGH_VOLTAGE,
            SetpointAdjustment.LOW_VOLTAGE,
            SetpointAdjustment.TEMPERATURE,
            SetpointAdjustment.UNKNOWN,
        ).forEachIndexed { index, expected ->
            assertEquals(expected, decoder.telemetry(canonicalSampleRecord().also { it[6] = (index + 1).toByte() }).setpointAdjustment)
        }

        val complete = decoder.telemetry(completeSampleRecord())
        listOf(
            complete.speedKph,
            complete.erpm,
            complete.motorCurrentA,
            complete.directionalMotorCurrentA,
            complete.filteredMotorCurrentA,
            complete.dutyCycle,
            complete.packVoltageV,
            complete.batteryCurrentA,
            complete.controllerTempC,
            complete.motorTempC,
            complete.pitchDeg,
            complete.balancePitchDeg,
            complete.rollDeg,
            complete.footpadAdc1V,
            complete.footpadAdc2V,
            complete.remoteInput,
            complete.setpointDeg,
            complete.atrSetpointDeg,
            complete.brakeTiltSetpointDeg,
            complete.torqueTiltSetpointDeg,
            complete.turnTiltSetpointDeg,
            complete.remoteSetpointDeg,
            complete.requestedBalanceCurrentA,
            complete.atrAccelerationDifferenceErpmPerSecond,
            complete.atrSpeedBoost,
            complete.boosterCurrentA,
            complete.chargingCurrentA,
            complete.chargingVoltageV,
        ).forEachIndexed { index, value -> assertEquals(index + 0.5, value!!, 0.0) }
        assertEquals(true, complete.charging)
    }

    @Test
    fun `adapter rejects impossible statuses lengths and canonical records`() {
        val decoder = RefloatDynamicTelemetryDecoder()
        val validLayout = canonicalLayoutRecord()
        listOf(
            Triple(0, intArrayOf(MAX_LAYOUT_RECORD_BYTES + 1), ByteArray(MAX_LAYOUT_RECORD_BYTES)),
            Triple(2, intArrayOf(1), validLayout),
            Triple(3, intArrayOf(1), validLayout),
            Triple(4, intArrayOf(1), validLayout),
            Triple(99, intArrayOf(0), validLayout),
        ).forEach { (status, result, record) ->
            assertThrows(VescProtocolException::class.java) {
                decoder.layoutBridgeResult(status, record, result)
            }
        }

        listOf(
            validLayout.copyOf(validLayout.size - 1),
            validLayout.copyOf().also { it[0] = 2 },
            validLayout.copyOf().also { it[1] = 2 },
            validLayout.copyOf().also { it[2] = 0 },
            validLayout.copyOf().also { it[6] = 1 },
            validLayout.copyOf().also { it[8] = 0 },
            validLayout.copyOf().also { it[9] = 0xff.toByte() },
        ).forEach { record ->
            assertThrows(VescProtocolException.TruncatedPayload::class.java) {
                decoder.layout(record)
            }
        }

        val validSample = canonicalSampleRecord()
        listOf(
            validSample.copyOf(SAMPLE_FIXED_BYTES - 1),
            validSample.copyOf().also { it[0] = 2 },
            validSample.copyOf().also { it[1] = 1 },
            validSample.copyOf().also { it[2] = 0 },
            validSample.copyOf().also { it[7] = 8 },
            validSample.copyOf().also { it[11] = 8 },
            validSample.copyOf().also { it[13] = 1 },
            validSample.copyOf().also { it[40] = 1 },
            canonicalSampleRecord("motor.speed"),
            canonicalSampleRecord("future.energy").copyOf().also {
                it[it.size - Long.SIZE_BYTES] = 0x7f
                it[it.size - Long.SIZE_BYTES + 1] = 0xf8.toByte()
            },
        ).forEach { record ->
            assertThrows(VescProtocolException.TruncatedPayload::class.java) {
                decoder.telemetry(record)
            }
        }
    }

    private fun bridgeLayout(payload: ByteArray): ByteArray {
        val output = ByteArray(MAX_LAYOUT_RECORD_BYTES)
        val result = IntArray(1)
        assertEquals(0, RustRefloatDynamicTelemetryBridge.decodeLayout(payload, output, result))
        return output.copyOf(result[0])
    }

    private fun bridgeSample(layout: ByteArray, payload: ByteArray): ByteArray {
        val output = ByteArray(MAX_SAMPLE_RECORD_BYTES)
        val result = IntArray(1)
        assertEquals(0, RustRefloatDynamicTelemetryBridge.decodeRealtime(layout, payload, output, result))
        return output.copyOf(result[0])
    }

    private fun canonicalLayoutRecord(): ByteArray =
        byteArrayOf(1, 1, 1, 0, 0, 11, 0, 0, 11) + "motor.speed".encodeToByteArray()

    private fun canonicalSampleRecord(extensionName: String? = null): ByteArray {
        val record = ByteArray(SAMPLE_RECORD_HEADER_BYTES).toMutableList()
        record[0] = 1
        record[1] = 2
        record[2] = 4
        record[3] = 1
        record[4] = 4
        record[5] = 1
        record[6] = 1
        record[8] = 1
        record[19] = 9
        record[23] = 1
        record.addAll(12.5.toBits().bigEndianBytes().asList())
        repeat(27 * Long.SIZE_BYTES) { record.add(0) }
        extensionName?.let { name ->
            val bytes = name.encodeToByteArray()
            record[12] = 1
            record.add(bytes.size.toByte())
            record.addAll(bytes.asList())
            record.addAll(3.25.toBits().bigEndianBytes().asList())
        }
        return record.toByteArray()
    }

    private fun completeSampleRecord(): ByteArray {
        val record = canonicalSampleRecord()
        record[7] = 0x04
        record[20] = 0x0f
        record[21] = 0xff.toByte()
        record[22] = 0xff.toByte()
        record[23] = 0xff.toByte()
        repeat(28) { index ->
            (index + 0.5).toBits().bigEndianBytes().copyInto(record, SAMPLE_RECORD_HEADER_BYTES + index * Long.SIZE_BYTES)
        }
        return record
    }

    private fun layoutPayload(name: String): ByteArray {
        val bytes = name.encodeToByteArray()
        return byteArrayOf(36, 101, 32, 1, bytes.size.toByte()) + bytes + byteArrayOf(0)
    }

    private fun readyPayload(bits: Int = 0x3c00): ByteArray = byteArrayOf(
        36, 101, 31, 0, 0, 0, 0, 0, 9, 2, 0, 0, 0,
        (bits ushr 8).toByte(), bits.toByte(),
    )

    private fun sourcePayload(name: String): ByteArray {
        val json = String(
            Files.readAllBytes(
                Path.of(System.getProperty("floatViewFixtureRoot"), "refloat/detailed-source-provenance.json"),
            ),
        )
        val match = requireNotNull(
            Regex(
                "\"name\"\\s*:\\s*\"${Regex.escape(name)}\"[\\s\\S]*?" +
                    "\"payloadHex\"\\s*:\\s*\"([0-9a-f]+)\"",
            ).find(json),
        )
        return match.groupValues[1].chunked(2).map { it.toInt(16).toByte() }.toByteArray()
    }

    private fun Long.bigEndianBytes(): ByteArray =
        ByteArray(Long.SIZE_BYTES) { index -> (this ushr (56 - index * 8)).toByte() }

    private fun Byte.unsigned(): Int = toInt() and 0xff

    private companion object {
        const val MAX_LAYOUT_PAYLOAD_BYTES = 4_165
        const val MAX_REALTIME_PAYLOAD_BYTES = 154
        const val MAX_LAYOUT_RECORD_BYTES = 4_168
        const val SAMPLE_RECORD_HEADER_BYTES = 32
        const val SAMPLE_FIXED_BYTES = 256
        const val MAX_SAMPLE_RECORD_BYTES = 2_592
    }
}
