package fail.acheron.floatview.protocol

import fail.acheron.floatview.backend.FirmwareFault
import fail.acheron.floatview.backend.VescProtocolMessage
import fail.acheron.floatview.protocol.refloat.RefloatBooleanControl
import fail.acheron.floatview.protocol.refloat.RefloatCompatibility
import fail.acheron.floatview.protocol.refloat.RefloatControlCodec
import fail.acheron.floatview.protocol.refloat.RefloatControlProfile
import fail.acheron.floatview.protocol.refloat.RefloatDynamicTelemetryDecoder
import fail.acheron.floatview.protocol.refloat.RefloatLightingProfile
import fail.acheron.floatview.protocol.refloat.RefloatLightingState
import fail.acheron.floatview.protocol.vesc.ControllerRoute
import fail.acheron.floatview.protocol.vesc.VescCompatibility
import java.nio.file.Files
import java.nio.file.Path
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test

class RefloatControlCodecTest {
    @Test
    fun sourceDerivedControllerShutdownRequestsAreExact() {
        val service = VescProtocolService()
        assertArrayEquals(
            shutdownGolden("vesc-6.05-direct"),
            VescFrameCodec.decode(service.controllerShutdownRequest(ControllerRoute.Direct)),
        )
        assertArrayEquals(
            shutdownGolden("vesc-6.06-can-42"),
            VescFrameCodec.decode(service.controllerShutdownRequest(ControllerRoute.Can(42))),
        )
        assertTrue(VescCompatibility.supportsShutdown(firmware(6, 5)))
        assertTrue(VescCompatibility.supportsShutdown(firmware(7, 0)))
        assertEquals(false, VescCompatibility.supportsShutdown(firmware(6, 2)))
        assertEquals(false, VescCompatibility.supportsShutdown(firmware(6, 6, testVersion = 1)))
    }
    @Test
    fun exactControlProfilesFailClosed() {
        listOf("1.0", "1.1.0", "1.1.1", "1.1.2").forEach {
            assertEquals(
                RefloatControlProfile(RefloatLightingProfile.LEGACY_COMMAND_202, false),
                RefloatCompatibility.controlProfile(it),
            )
        }
        listOf("1.2.0", "1.2.1", "1.2.2", "1.2.3").forEach {
            assertEquals(
                RefloatControlProfile(RefloatLightingProfile.STABLE_COMMAND_20, true),
                RefloatCompatibility.controlProfile(it),
            )
        }
        assertNull(RefloatCompatibility.controlProfile("1.0.0"))
        assertNull(RefloatCompatibility.controlProfile("1.2.4"))
        assertNull(RefloatCompatibility.controlProfile("1.2.3-beta1"))
    }

    @Test
    fun sourceDerivedRequestsAndLightingResponsesAreExact() {
        val service = VescProtocolService()
        val legacy = requireNotNull(RefloatCompatibility.controlProfile("1.0"))
        val stable = requireNotNull(RefloatCompatibility.controlProfile("1.2.3"))
        val current = RefloatLightingState(true, true)
        val requests = mapOf(
            "package-enable" to service.refloatPackageControlRequest(ControllerRoute.Direct, true),
            "package-disable" to service.refloatPackageControlRequest(ControllerRoute.Direct, false),
            "flywheel-enable-defaults-footpad-abort" to
                service.refloatFlywheelControlRequest(ControllerRoute.Direct, true),
            "flywheel-disable-defaults-footpad-abort" to
                service.refloatFlywheelControlRequest(ControllerRoute.Direct, false),
            "legacy-lighting-leds-off-headlights-on" to service.refloatLightingControlRequest(
                ControllerRoute.Direct,
                legacy,
                RefloatBooleanControl.LEDS,
                false,
                current,
            ),
            "stable-lighting-headlights-off-leds-on" to service.refloatLightingControlRequest(
                ControllerRoute.Direct,
                stable,
                RefloatBooleanControl.HEADLIGHTS,
                false,
                current,
            ),
            "fatal-clear" to service.refloatFatalClearRequest(ControllerRoute.Direct, stable),
        )
        requests.forEach { (name, frame) ->
            assertArrayEquals(goldenArray(name, "request"), VescFrameCodec.decode(frame))
        }

        listOf("legacy-lighting-leds-off-headlights-on", "stable-lighting-headlights-off-leds-on").forEach { name ->
            val response = goldenArray(name, "response")
            val message = service.receive(VescFrameCodec.encode(response)).single()
            assertTrue(message is VescProtocolMessage.RefloatLighting)
            message as VescProtocolMessage.RefloatLighting
            assertEquals(response[3].toInt() and 1 != 0, message.state.ledsEnabled)
            assertEquals(response[3].toInt() and 2 != 0, message.state.headlightsEnabled)
        }
    }

    @Test
    fun routeAndResponseBoundsAreExact() {
        val service = VescProtocolService()
        assertArrayEquals(
            byteArrayOf(34, 42, 36, 101, 12, 0),
            VescFrameCodec.decode(service.refloatPackageControlRequest(ControllerRoute.Can(42), true)),
        )
        val stable = requireNotNull(RefloatCompatibility.controlProfile("1.2.3"))
        assertThrows(VescProtocolException::class.java) {
            RefloatControlCodec.decodeLightingResponse(byteArrayOf(36, 101, 20), stable)
        }
        assertThrows(VescProtocolException::class.java) {
            RefloatControlCodec.decodeLightingResponse(byteArrayOf(36, 101, 20, 3, 0), stable)
        }
        assertThrows(VescProtocolException::class.java) {
            RefloatControlCodec.decodeLightingResponse(byteArrayOf(36, 101, 202.toByte(), 3), stable)
        }
    }

    @Test
    fun fatalLatchAndActiveFaultDecodeIndependently() {
        val decoder = RefloatDynamicTelemetryDecoder()
        decoder.decodeFields(
            byteArrayOf(36, 101, 32, 1, 11) + "motor.speed".encodeToByteArray() + byteArrayOf(0),
        )
        val latched = byteArrayOf(
            36, 101, 31, 4, 8, 0, 0, 0, 0, 2, 0, 0, 0,
            0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0,
        )
        val latchedValue = decoder.decodeData(latched)
        assertEquals(true, latchedValue.fatalErrorLatched)
        assertNull(latchedValue.firmwareFault)

        val active = latched.copyOf()
        active[4] = 0
        active[18] = 1
        active[23] = 1
        val activeValue = decoder.decodeData(active)
        assertEquals(false, activeValue.fatalErrorLatched)
        assertEquals(FirmwareFault.OVER_VOLTAGE, activeValue.firmwareFault)
    }

    private fun goldenArray(name: String, field: String): ByteArray {
        val json = String(
            Files.readAllBytes(
                Path.of(System.getProperty("floatViewFixtureRoot"), "refloat/product-controls.json"),
            ),
        )
        val match = requireNotNull(
            Regex(
                "\\\"name\\\"\\s*:\\s*\\\"${Regex.escape(name)}\\\"[\\s\\S]*?" +
                    "\\\"$field\\\"\\s*:\\s*\\[([^]]*)]",
            ).find(json),
        )
        return match.groupValues[1]
            .split(',')
            .map(String::trim)
            .filter(String::isNotEmpty)
            .map { it.toInt().toByte() }
            .toByteArray()
    }

    private fun shutdownGolden(name: String): ByteArray {
        val json = String(
            Files.readAllBytes(
                Path.of(System.getProperty("floatViewFixtureRoot"), "vesc/controller-shutdown.json"),
            ),
        )
        val match = requireNotNull(
            Regex("\\\"name\\\"\\s*:\\s*\\\"${Regex.escape(name)}\\\"[\\s\\S]*?\\\"request\\\"\\s*:\\s*\\[([^]]*)]" ).find(json),
        )
        return match.groupValues[1].split(',').map(String::trim).filter(String::isNotEmpty)
            .map { it.toInt().toByte() }.toByteArray()
    }

    private fun firmware(major: Int, minor: Int, testVersion: Int = 0) =
        fail.acheron.floatview.backend.FirmwareRecord(
            major, minor, "Any", "Refloat", testVersion = testVersion,
        )
}
