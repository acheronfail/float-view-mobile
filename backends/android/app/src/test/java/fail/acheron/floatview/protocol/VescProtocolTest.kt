package fail.acheron.floatview.protocol

import java.nio.file.Files
import java.nio.file.Path
import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Test
import fail.acheron.floatview.backend.VescProtocolMessage
import fail.acheron.floatview.backend.FootpadState
import fail.acheron.floatview.backend.RideState
import fail.acheron.floatview.protocol.refloat.RefloatCompatibility
import fail.acheron.floatview.protocol.refloat.RefloatDynamicTelemetryDecoder
import fail.acheron.floatview.protocol.refloat.RefloatLegacyTelemetryDecoder
import fail.acheron.floatview.protocol.refloat.RefloatTelemetryFamily
import fail.acheron.floatview.protocol.vesc.VescCompatibility
import fail.acheron.floatview.protocol.vesc.ControllerRoute
import fail.acheron.floatview.protocol.vesc.VescFirmwareVersion

private fun String.hexBytes(): ByteArray =
    chunked(2).map { it.toInt(16).toByte() }.toByteArray()

private fun fixture(path: String): String {
    val json = String(Files.readAllBytes(Path.of(System.getProperty("floatViewFixtureRoot"), path)))
    return Regex("\"frameHex\"\\s*:\\s*\"([0-9a-f]+)\"").find(json)!!.groupValues[1]
}

private fun fixtureCases(path: String): List<String> {
    val json = String(Files.readAllBytes(Path.of(System.getProperty("floatViewFixtureRoot"), path)))
    return Regex("\"frameHex\"\\s*:\\s*\"([0-9a-f]+)\"")
        .findAll(json)
        .map { it.groupValues[1] }
        .toList()
}

private fun refloatSourcePayload(name: String): ByteArray {
    val json = String(
        Files.readAllBytes(
            Path.of(
                System.getProperty("floatViewFixtureRoot"),
                "refloat/detailed-source-provenance.json",
            ),
        ),
    )
    val pattern =
        Regex(
            "\"name\"\\s*:\\s*\"${Regex.escape(name)}\"[\\s\\S]*?\"payloadHex\"\\s*:\\s*\"([0-9a-f]+)\"",
        )
    return requireNotNull(pattern.find(json)).groupValues[1].hexBytes()
}

class VescProtocolTest {
    @Test
    fun exactCompatibilityCatalogsAndLegacyWireIdentityAreStable() {
        assertEquals(
            setOf(
                VescFirmwareVersion(6, 2),
                VescFirmwareVersion(6, 5),
                VescFirmwareVersion(6, 6),
            ),
            VescCompatibility.supported,
        )
        assertEquals(
            listOf("1.0.0", "1.1.0", "1.2.3"),
            RefloatCompatibility.entries.map { it.sourceVersion },
        )
        assertEquals(
            RefloatTelemetryFamily.LEGACY_FIXED,
            RefloatCompatibility.entry("1.0")?.family,
        )
        assertEquals(null, RefloatCompatibility.entry("1.2.0"))

        val service = VescProtocolService()
        val legacy = VescFrameCodec.encode(byteArrayOf(36, 101, 0, 10))
        assertEquals(
            listOf(VescProtocolMessage.RefloatInfo("1.0")),
            service.receive(legacy),
        )
        assertEquals(
            listOf(36, 101, 10, 4),
            VescFrameCodec.decode(service.refloatLegacyTelemetryRequest(ControllerRoute.Direct))
                .map { it.toInt() and 0xff },
        )
        assertEquals(
            listOf(36, 101, 31),
            VescFrameCodec.decode(service.refloatTelemetryRequest(ControllerRoute.Direct))
                .map { it.toInt() and 0xff },
        )
    }

    @Test
    fun firmwareFixtureDecodesAcrossChunks() {
        val frame = fixture("vesc/firmware-6.06.json").hexBytes()
        val stream = VescFrameStreamDecoder()
        val sizes = listOf(2, 1, 4, 9)
        var offset = 0
        val payloads = sizes.flatMap { size ->
            stream.append(frame.copyOfRange(offset, offset + size)).also { offset += size }
        }
        assertEquals(FirmwareInfo(6, 6, "THOR300"), VescPayloadDecoder.firmware(payloads.single()))
    }

    @Test
    fun composedThorFirmwareVariantsDecode() {
        listOf("6.02" to 2, "6.05" to 5, "6.06" to 6).forEach { (version, minor) ->
            val payload = VescFrameCodec.decode(
                fixture("vesc/firmware-thor300-$version.json").hexBytes(),
            )
            assertEquals(
                FirmwareInfo(
                    6,
                    minor,
                    "THOR300",
                    hardwareType = 0,
                    controllerUuid = ByteArray(12),
                ),
                VescPayloadDecoder.firmware(payload),
            )
        }
    }

    @Test
    fun realtimeAndRefloatFixturesDecode() {
        val values = VescPayloadDecoder.realtime(
            VescFrameCodec.decode(fixture("vesc/realtime-values.json").hexBytes()),
        )
        assertEquals(25.0, values.controllerTempC, 0.0)
        assertEquals(23.0, values.motorTempC, 0.0)
        assertEquals(12.34, values.motorCurrentA, 0.0)
        assertEquals(-5.0, values.batteryCurrentA, 0.0)
        assertEquals(0.5, values.dutyCycle, 0.0)
        assertEquals(10_000, values.erpm)
        assertEquals(51.6, values.packVoltageV, 0.0)
        assertEquals(0, values.faultCode)
        assertEquals(VescFault.fromCode(0), values.fault)

        val faultValues = VescPayloadDecoder.realtime(
            VescFrameCodec.decode(fixture("vesc/realtime-values-over-voltage.json").hexBytes()),
        )
        assertEquals(1, faultValues.faultCode)
        assertEquals(VescFault.fromCode(1), faultValues.fault)
        assertEquals("over_voltage", faultValues.fault.kind.stableName)
        assertEquals(VescFault.Kind.UNKNOWN, VescFault.fromCode(255).kind)
        assertEquals(255, VescFault.fromCode(255).code)

    }

    @Test
    fun detailedSourceFixturesDecodeEveryKnownLegacyAndDynamicField() {
        val legacy = RefloatLegacyTelemetryDecoder.decode(
            refloatSourcePayload("1.0.0-command-10-detail-4"),
        )
        assertEquals("running", legacy.packageState?.wireValue)
        assertEquals("normal", legacy.packageMode?.wireValue)
        assertEquals(FootpadState.BOTH, legacy.footpadState)
        listOf(
            legacy.packVoltageV,
            legacy.dutyCycle,
            legacy.motorCurrentA,
            legacy.batteryCurrentA,
            legacy.erpm,
            legacy.speedKph,
            legacy.pitchDeg,
            legacy.balancePitchDeg,
            legacy.rollDeg,
            legacy.requestedBalanceCurrentA,
            legacy.boosterCurrentA,
            legacy.fieldWeakeningCurrentA,
            legacy.footpadAdc1V,
            legacy.footpadAdc2V,
            legacy.setpointDeg,
            legacy.atrSetpointDeg,
            legacy.brakeTiltSetpointDeg,
            legacy.torqueTiltSetpointDeg,
            legacy.turnTiltSetpointDeg,
            legacy.remoteSetpointDeg,
            legacy.distanceM,
            legacy.odometerM,
            legacy.ampHoursDrawnAh,
            legacy.ampHoursChargedAh,
            legacy.wattHoursDrawnWh,
            legacy.wattHoursChargedWh,
            legacy.batteryLevel,
            legacy.chargingCurrentA,
            legacy.chargingVoltageV,
        ).forEach { requireNotNull(it) }

        val decoder = RefloatDynamicTelemetryDecoder()
        val (fields, runtimeFields) = decoder.decodeFields(
            refloatSourcePayload("1.1.0-and-1.2.3-command-32-layout"),
        )
        assertEquals(16, fields.size)
        assertEquals(10, runtimeFields.size)
        val dynamic = decoder.decodeData(
            refloatSourcePayload("1.2.3-command-31-running-alert-tail"),
        )
        assertEquals(RideState.RIDING, dynamic.rideState)
        assertEquals(FootpadState.BOTH, dynamic.footpadState)
        requireNotNull(dynamic.directionalMotorCurrentA)
        requireNotNull(dynamic.filteredMotorCurrentA)
        requireNotNull(dynamic.remoteInput)
        requireNotNull(dynamic.atrAccelerationDifferenceErpmPerSecond)
        requireNotNull(dynamic.atrSpeedBoost)
        assertEquals(emptyMap<String, Double>(), dynamic.extensions)

        val fault = decoder.decodeData(
            refloatSourcePayload("1.2.3-command-31-active-firmware-fault"),
        )
        assertEquals("firmwareFault", fault.alertReason?.wireValue)
        assertEquals("overVoltage", fault.firmwareFault?.wireValue)
    }

    @Test
    fun corruptCrcIsRejected() {
        assertThrows(VescProtocolException.InvalidCrc::class.java) {
            VescFrameCodec.decode("020b00060654484f52333030002ad903".hexBytes())
        }
    }

    @Test
    fun longFramesNoiseMultiplePacketsAndTruncation() {
        val payload = byteArrayOf(4) + ByteArray(255)
        val longFrame = VescFrameCodec.encode(payload)
        assertEquals(3, longFrame.first().toInt())
        assertEquals(payload.toList(), VescFrameCodec.decode(longFrame).toList())

        val shortFrame = VescFrameCodec.encode(byteArrayOf(0, 6, 6, 0))
        val stream = VescFrameStreamDecoder()
        val packets = stream.append(byteArrayOf(0xff.toByte(), 1) + shortFrame + shortFrame)
        assertEquals(2, packets.size)
        assertEquals(packets[0].toList(), packets[1].toList())

        val truncated = VescFrameStreamDecoder()
        assertEquals(emptyList<ByteArray>(), truncated.append(shortFrame.dropLast(1).toByteArray()))
        assertEquals(1, truncated.append(byteArrayOf(shortFrame.last())).size)
    }

    @Test
    fun canDiscoveryAndForwardedFirmwarePreserveCanIdentity() {
        val service = VescProtocolService()
        assertEquals(
            listOf(62),
            VescFrameCodec.decode(service.canPingRequest()).map { it.toInt() and 0xff },
        )
        assertEquals(
            VescProtocolMessage.CanDevices(listOf(7, 42)),
            service.receive(VescFrameCodec.encode(byteArrayOf(62, 42, 7, 42))).single(),
        )

        assertEquals(
            listOf(34, 42, 0),
            VescFrameCodec.decode(service.firmwareRequest(ControllerRoute.Can(42)))
                .map { it.toInt() and 0xff },
        )
        val firmwareFrame = fixture("vesc/firmware-6.06.json").hexBytes()
        val firmware = service.receive(firmwareFrame).single() as VescProtocolMessage.Firmware
        assertEquals(42, firmware.value.canId)
        assertEquals("THOR300", firmware.value.hardwareName)

        assertEquals(
            listOf(34, 42, 36, 101, 0, 2, 0),
            VescFrameCodec.decode(service.refloatInfoRequest(ControllerRoute.Can(42)))
                .map { it.toInt() and 0xff },
        )
        assertEquals(
            VescProtocolMessage.RefloatInfo("1.2"),
            service.receive(VescFrameCodec.encode(byteArrayOf(36, 101, 0, 12))).single(),
        )
    }

    @Test
    fun refloatRealtimeIdsAndDataNormalizeCoreTelemetry() {
        val service = VescProtocolService()
        assertEquals(
            listOf(34, 42, 36, 101, 32),
            VescFrameCodec.decode(service.refloatTelemetryIdsRequest(ControllerRoute.Can(42)))
                .map { it.toInt() and 0xff },
        )
        assertEquals(
            listOf(34, 42, 36, 101, 31),
            VescFrameCodec.decode(service.refloatTelemetryRequest(ControllerRoute.Can(42)))
                .map { it.toInt() and 0xff },
        )
        val fields = listOf(
            "motor.speed",
            "motor.erpm",
            "motor.current",
            "motor.duty_cycle",
            "motor.batt_voltage",
            "motor.batt_current",
            "motor.mosfet_temp",
            "motor.motor_temp",
            "future.energy",
        )
        val idsPayload = mutableListOf(36, 101, 32, fields.size)
        fields.forEach { field ->
            idsPayload += field.length
            idsPayload += field.encodeToByteArray().map { it.toInt() and 0xff }
        }
        idsPayload += 0
        val layout = service.receive(
            VescFrameCodec.encode(idsPayload.map(Int::toByte).toByteArray()),
        ).single()
        assertEquals(
            VescProtocolMessage.RefloatTelemetryLayout(fields, emptyList()),
            layout,
        )

        val payload = byteArrayOf(
            36, 101, 31,
            0, 0, 0, 0, 0, 1,
            3, 0x40, 0, 0,
            0x49, 0x00,
            0x64, 0x00,
            0x48, 0x00,
            0x38, 0x00,
            0x54, 0x00,
            0xc4.toByte(), 0x00,
            0x50, 0x00,
            0x4c, 0x00,
            0x45, 0x00,
        )

        val message = service.receive(VescFrameCodec.encode(payload)).single()
            as VescProtocolMessage.RefloatTelemetry
        assertEquals(RideState.RIDING, message.value.rideState)
        assertEquals(FootpadState.LEFT, message.value.footpadState)
        assertEquals(64.0, message.value.packVoltageV!!, 0.0)
        assertEquals(0.5, message.value.dutyCycle!!, 0.0)
        assertEquals(32.0, message.value.controllerTempC!!, 0.0)
        assertEquals(16.0, message.value.motorTempC!!, 0.0)
        assertEquals(8.0, message.value.motorCurrentA!!, 0.0)
        assertEquals(-4.0, message.value.batteryCurrentA!!, 0.0)
        assertEquals(1024.0, message.value.erpm)
        assertEquals(36.0, message.value.speedKph!!, 0.0)
        assertEquals(mapOf("future.energy" to 5.0), message.value.extensions)
    }

    @Test
    fun androidHardwareRefloatCaptureDecodes() {
        val service = VescProtocolService()
        val messages = fixtureCases("refloat/rtdata-1.2.3-android-hardware.json")
            .flatMap { service.receive(it.hexBytes()) }

        assertEquals(3, messages.size)
        assertEquals(VescProtocolMessage.RefloatInfo("1.2.3"), messages[0])
        val layout = messages[1] as VescProtocolMessage.RefloatTelemetryLayout
        assertEquals(16, layout.fields.size)
        assertEquals(10, layout.runtimeFields.size)
        assertEquals(
            listOf("motor.speed", "motor.erpm", "motor.current"),
            layout.fields.take(3),
        )
        val telemetry = (messages[2] as VescProtocolMessage.RefloatTelemetry).value
        assertEquals(RideState.IDLE, telemetry.rideState)
        assertEquals(FootpadState.NONE, telemetry.footpadState)
        assertEquals(0.0, telemetry.motorCurrentA!!, 0.0)
        assertEquals(0.0, telemetry.batteryCurrentA!!, 0.0)
        assertEquals(-0.07818603515625, telemetry.erpm)
    }
}
