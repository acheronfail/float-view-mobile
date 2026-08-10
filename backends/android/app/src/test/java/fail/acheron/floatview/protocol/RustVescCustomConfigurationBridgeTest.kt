package fail.acheron.floatview.protocol

import fail.acheron.floatview.protocol.vesc.VescCommand
import fail.acheron.floatview.protocol.vesc.VescCustomConfigurationCodec
import fail.acheron.floatview.protocol.vesc.VescCustomConfigurationResponse
import java.nio.file.Files
import java.nio.file.Path
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertThrows
import org.junit.Test

class RustVescCustomConfigurationBridgeTest {
    @Test
    fun rawBridgeReportsExactAbiAndRejectsMismatch() {
        assertEquals(1, RustVescCustomConfigurationBridge.abiVersion())
        assertThrows(VescProtocolException.TruncatedPayload::class.java) {
            verifyVescCustomConfigurationAbiVersion(2)
        }
    }

    @Test
    fun rawEncodeValidatesArraysShapesAliasesOperationsAndCapacity() {
        val empty = ByteArray(0)
        val output = ByteArray(10) { 0x5a }
        val result = IntArray(1)
        assertEquals(
            0,
            RustVescCustomConfigurationBridge.encode(1, 7, 400, 3, empty, output, result),
        )
        assertEquals(10, result[0])
        assertArrayEquals(byteArrayOf(92, 7, 0, 0, 1, 0x90.toByte(), 0, 0, 0, 3), output)

        assertEquals(1, RustVescCustomConfigurationBridge.encode(1, 0, 1, 0, null, output, result))
        assertEquals(1, RustVescCustomConfigurationBridge.encode(1, 0, 1, 0, empty, null, result))
        assertEquals(1, RustVescCustomConfigurationBridge.encode(1, 0, 1, 0, empty, output, null))
        assertEquals(1, RustVescCustomConfigurationBridge.encode(1, 0, 1, 0, empty, output, IntArray(0)))
        val aliased = byteArrayOf(1)
        assertEquals(1, RustVescCustomConfigurationBridge.encode(4, 0, 0, 0, aliased, aliased, result))
        assertEquals(
            1,
            RustVescCustomConfigurationBridge.encode(
                1,
                0,
                1,
                0,
                empty,
                ByteArray(65_537),
                result,
            ),
        )
        assertEquals(1, RustVescCustomConfigurationBridge.encode(99, 0, 0, 0, empty, output, result))
        assertArrayEquals(intArrayOf(0), result)

        val short = ByteArray(1)
        assertEquals(4, RustVescCustomConfigurationBridge.encode(2, 0, 0, 0, empty, short, result))
        assertArrayEquals(intArrayOf(2), result)
        assertArrayEquals(ByteArray(1), short)
    }

    @Test
    fun rawDecodeReturnsFixedMetadataAndBoundedFailures() {
        val schema = byteArrayOf(92, 7, 0, 0, 0, 4, 0, 0, 0, 0, 1, 2, 3, 4)
        val result = IntArray(8)
        assertEquals(0, RustVescCustomConfigurationBridge.decode(schema, result))
        assertArrayEquals(intArrayOf(1, 92, 7, 4, 0, 10, 4, 0), result)

        assertEquals(1, RustVescCustomConfigurationBridge.decode(null, result))
        assertEquals(1, RustVescCustomConfigurationBridge.decode(schema, null))
        assertEquals(1, RustVescCustomConfigurationBridge.decode(schema, IntArray(7)))

        result.fill(99)
        assertEquals(2, RustVescCustomConfigurationBridge.decode(ByteArray(65_537), result))
        assertArrayEquals(IntArray(8), result)
        result.fill(99)
        assertEquals(3, RustVescCustomConfigurationBridge.decode(byteArrayOf(0), result))
        assertArrayEquals(IntArray(8), result)
        result.fill(99)
        assertEquals(2, RustVescCustomConfigurationBridge.decode(byteArrayOf(95, 0), result))
        assertArrayEquals(IntArray(8), result)
    }

    @Test
    fun publicAdapterCrossesRustForEveryReleaseValuesWriteAndReadback() {
        val fixtureRoot = Path.of(requireNotNull(System.getProperty("floatViewFixtureRoot")))
        val versions = listOf("1.0.0", "1.0.1", "1.1.0", "1.1.1", "1.1.2", "1.2.0", "1.2.1", "1.2.2", "1.2.3")
        for (version in versions) {
            val valuesRoot = fixtureRoot.resolve("refloat-configuration/values")
            val current = Files.readAllBytes(valuesRoot.resolve("$version-current.bin"))
            val defaults = Files.readAllBytes(valuesRoot.resolve("$version-default.bin"))
            val write = Files.readAllBytes(valuesRoot.resolve("$version-write.bin"))
            val readback = Files.readAllBytes(valuesRoot.resolve("$version-readback.bin"))

            for ((command, expected, bytes) in listOf(
                Triple(93, VescCommand.GET_CUSTOM_CONFIGURATION, current),
                Triple(94, VescCommand.GET_CUSTOM_CONFIGURATION_DEFAULT, defaults),
                Triple(93, VescCommand.GET_CUSTOM_CONFIGURATION, readback),
            )) {
                val decoded = VescCustomConfigurationCodec.decode(
                    byteArrayOf(command.toByte(), 0) + bytes,
                ) as VescCustomConfigurationResponse.Values
                assertEquals(expected, decoded.command)
                assertArrayEquals(bytes, decoded.bytes)
            }
            assertArrayEquals(
                byteArrayOf(95, 0) + write,
                VescCustomConfigurationCodec.writeRequest(values = write),
            )
        }
    }

    @Test
    fun adapterRejectsImpossibleMetadataAndPayloadDiagnostics() {
        val payload = byteArrayOf(93, 0, 0, 0, 0, 1)
        assertThrows(VescProtocolException.TruncatedPayload::class.java) {
            VescCustomConfigurationCodec.decodeBridgeResult(0, payload, IntArray(7))
        }
        assertThrows(VescProtocolException.TruncatedPayload::class.java) {
            VescCustomConfigurationCodec.decodeBridgeResult(
                0,
                payload,
                intArrayOf(2, 93, 0, 0, 0, 2, 99, 0),
            )
        }
        val unsupported = assertThrows(VescProtocolException.UnsupportedCommand::class.java) {
            VescCustomConfigurationCodec.decode(byteArrayOf(36))
        }
        assertEquals(36, unsupported.command)

        val privatePayload = "SECRET-CONFIGURATION".encodeToByteArray()
        val malformed = assertThrows(VescProtocolException.TruncatedPayload::class.java) {
            VescCustomConfigurationCodec.decode(privatePayload)
        }
        assertFalse(malformed.toString().contains("SECRET"))
    }
}
