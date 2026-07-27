package fail.acheron.floatview.protocol

import java.nio.file.Files
import java.nio.file.Path
import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Test

private fun String.hexBytes(): ByteArray =
    chunked(2).map { it.toInt(16).toByte() }.toByteArray()

private fun fixture(path: String): String {
    val json = Files.readString(Path.of(System.getProperty("floatViewFixtureRoot"), path))
    return Regex("\"frameHex\"\\s*:\\s*\"([0-9a-f]+)\"").find(json)!!.groupValues[1]
}

class VescProtocolTest {
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

        val refloat = VescPayloadDecoder.refloatRealtime(
            VescFrameCodec.decode(fixture("refloat/rtdata-1.2.json").hexBytes()),
        )
        assertEquals(RefloatRealtime(101, 1, 0, 0), refloat)
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
}
