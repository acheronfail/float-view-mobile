package fail.acheron.floatview

import java.nio.ByteBuffer
import java.nio.ByteOrder
import kotlin.math.roundToInt

class VescBuffer(private val buffer: ByteArray) {
    private val bb: ByteBuffer = ByteBuffer.wrap(buffer).order(ByteOrder.BIG_ENDIAN)
    var index: Int = 0

    fun raw(): ByteArray = buffer

    fun size(): Int = buffer.size

    override fun toString(): String {
        return buffer.joinToString(", ", "[", "]") { (it.toInt() and 0xFF).toString() }
    }

    fun slice(expected: Int): ByteArray {
        return if (index + expected <= buffer.size) {
            val value = buffer.copyOfRange(index, index + expected - 1)
            index += expected - 1
            value
        } else {
            ByteArray(0)
        }
    }

    fun readUint8Pair(): Pair<UInt, UInt> {
        val byte = readUInt8()
        val upper = (byte shr 4) and 0x0FU  // Extract upper 4 bits
        val lower = byte and 0x0FU          // Extract lower 4 bits
        return Pair(upper, lower)
    }

    fun readUInt8(): UInt {
        val value = buffer[index].toUInt() and 0xFFU
        index += 1
        return value
    }

    fun readInt8(): Int {
        val value = buffer[index].toInt()
        index += 1
        return value
    }

    fun readUInt16(): Int {
        val value = ((buffer[index].toInt() and 0xFF) shl 8) or
                    (buffer[index + 1].toInt() and 0xFF)
        index += 2
        return value
    }

    fun readInt16(): Short {
        val value = bb.getShort(index)
        index += 2
        return value
    }

    fun readUInt32(): Long {
        val value = ((buffer[index].toLong() and 0xFF) shl 24) or
                    ((buffer[index + 1].toLong() and 0xFF) shl 16) or
                    ((buffer[index + 2].toLong() and 0xFF) shl 8) or
                    (buffer[index + 3].toLong() and 0xFF)
        index += 4
        return value
    }

    fun readInt32(): Int {
        val value = bb.getInt(index)
        index += 4
        return value
    }

    fun readDouble16(scale: Int): Double {
        return readInt16().toDouble() / scale
    }

    fun readDouble32(scale: Int): Double {
        return readInt32().toDouble() / scale
    }

    fun readFloat32(): Float {
        val value = bb.getFloat(index)
        index += 4
        return value
    }

    fun readString(): String {
        val sb = StringBuilder()
        while (index < buffer.size) {
            val b = readUInt8()
            if (b == 0x00U) break
            sb.append(b.toInt().toChar())
        }
        return sb.toString()
    }

    // Writers

    fun writeUInt8(value: Int) {
        buffer[index] = value.toByte()
        index++
    }

    fun writeInt8(value: Int) {
        buffer[index] = value.toByte()
        index++
    }

    fun writeUInt16(value: Int) {
        buffer[index] = ((value shr 8) and 0xFF).toByte()
        buffer[index + 1] = (value and 0xFF).toByte()
        index += 2
    }

    fun writeInt16(value: Int) {
        buffer[index] = ((value shr 8) and 0xFF).toByte()
        buffer[index + 1] = (value and 0xFF).toByte()
        index += 2
    }

    fun writeUInt32(value: Long) {
        buffer[index]     = ((value shr 24) and 0xFF).toByte()
        buffer[index + 1] = ((value shr 16) and 0xFF).toByte()
        buffer[index + 2] = ((value shr 8) and 0xFF).toByte()
        buffer[index + 3] = (value and 0xFF).toByte()
        index += 4
    }

    fun writeInt32(value: Int) {
        buffer[index]     = ((value shr 24) and 0xFF).toByte()
        buffer[index + 1] = ((value shr 16) and 0xFF).toByte()
        buffer[index + 2] = ((value shr 8) and 0xFF).toByte()
        buffer[index + 3] = (value and 0xFF).toByte()
        index += 4
    }

    fun writeDouble16(value: Double, scale: Int) {
        val scaled = (value * scale).roundToInt()
        writeInt16(scaled)
    }

    fun writeDouble32(value: Double, scale: Int) {
        val scaled = (value * scale).toLong()
        val bbOut = ByteBuffer.allocate(8).order(ByteOrder.BIG_ENDIAN)
        bbOut.putDouble(scaled.toDouble())
        val arr = bbOut.array()
        System.arraycopy(arr, 0, buffer, index, 8)
        index += 8
    }

    fun writeFloat32(value: Float) {
        val bbOut = ByteBuffer.allocate(4).order(ByteOrder.BIG_ENDIAN)
        bbOut.putFloat(value)
        val arr = bbOut.array()
        System.arraycopy(arr, 0, buffer, index, 4)
        index += 4
    }

    fun writeString(value: String, terminate: Boolean = true) {
        for (c in value) {
            writeUInt8(c.code)
        }
        if (terminate) {
            writeUInt8(0) // Null terminator
        }
    }
}
