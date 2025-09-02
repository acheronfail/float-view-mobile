package fail.acheron.floatview

import android.annotation.SuppressLint
import android.bluetooth.BluetoothGatt
import android.bluetooth.BluetoothGattCharacteristic
import android.bluetooth.BluetoothStatusCodes

val COMM_FW_VERSION = 0
val COMM_FORWARD_CAN = 34

class VescBluetoothLayer(
        private val gatt: BluetoothGatt,
        private val vescRx: BluetoothGattCharacteristic
) {

    interface CommandCallback {
        fun onResponse(command: Int, data: ByteArray)
        fun onError(command: Int, message: String)
    }

    private val commandCallbacks = mutableMapOf<Int, CommandCallback>()

    fun parseResponse(bytes: ByteArray) {
        val parsed = parseVescCharacteristicResponse(bytes)
        if (parsed == null) {
            printError(
                    "Failed to parse VESC response: ${bytes.joinToString(" ") { "%02X".format(it) }}"
            )
        } else {
            val (command, response) = parsed
            val callback = commandCallbacks[command]
            if (callback != null) {
                callback.onResponse(command, response)
                commandCallbacks.remove(command)
            } else {
                printError("No callback registered for command: $command")
            }
        }
    }

    private fun parseVescCharacteristicResponse(data: ByteArray): Pair<Int, ByteArray>? {
        try {
            if (data.size < 5) return null // Minimum packet size
            var index = 0

            // Check start byte
            if (data[index] != 2.toByte() && data[index] != 3.toByte()) return null
            val isLongPacket = data[index] == 3.toByte()
            index++

            // Read length
            val payloadLength =
                    if (isLongPacket) {
                        if (data.size < 6) return null
                        val lengthHigh = data[index++].toInt() and 0xFF
                        val lengthLow = data[index++].toInt() and 0xFF
                        (lengthHigh shl 8) or lengthLow
                    } else {
                        data[index++].toInt() and 0xFF
                    }

            if (data.size < index + payloadLength + 3) return null // payload + CRC + end

            // Extract payload
            val payload = data.sliceArray(index until index + payloadLength)
            index += payloadLength

            // Skip CRC validation for now
            index += 2

            // Check end byte
            if (data[index] != 3.toByte()) return null

            // Extract command from payload
            if (payload.isEmpty()) return null
            val command = payload[0].toInt() and 0xFF

            return Pair(command, payload.sliceArray(1 until payload.size))
        } catch (e: Exception) {
            printError("Error parsing VESC response: ${e.message}")
            return null
        }
    }

    // TODO: create enum for command
    @SuppressLint("NewApi")
    fun send(command: Int, data: List<Int>, callback: CommandCallback) {
        if (commandCallbacks[command] != null) {
            return callback.onError(command, "Command already in progress")
        }

        commandCallbacks[command] = callback

        // TODO: detect if need to use can or not, rather than hardcoding here
        // TODO: scan for VESCs on can bus, rather than hardcoding
        val canId = 1
        val payload = mutableListOf(COMM_FORWARD_CAN, canId, command)
        payload.addAll(data)

        val packet = generateVescPacket(payload)

        print("Command payload: ${payload.joinToString(" ") { "%02X".format(it) }}")
        print("Command packet: ${packet.joinToString(" ") { "%02X".format(it) }}")

        try {
            // we write the data here, and we will receive the response in the `onCharacteristicChanged`
            // callback function in the `VescManager` class
            val result =
                    gatt.writeCharacteristic(
                            vescRx,
                            packet,
                            BluetoothGattCharacteristic.WRITE_TYPE_DEFAULT
                    )

            if (result != BluetoothStatusCodes.SUCCESS) {
                commandCallbacks.remove(command)
                return callback.onError(command, "Failed to write characteristic: $result")
            }
        } catch (error: SecurityException) {
            commandCallbacks.remove(command)
            return callback.onError(command, "Permission denied: ${error.message}")
        }
    }

    /** Clean up resources and clear callbacks */
    fun cleanUp() {
        commandCallbacks.clear()
    }

    private fun generateVescPacket(data: List<Int>): ByteArray {
        val payload = data.map { it.toByte() }.toByteArray()
        val packetSizeWithoutPayload = 5 // Beginning (1) + Length (1) + CRC16 (2) + End (1)

        val buffer: ByteArray
        var index = 0

        if (payload.size <= 256) {
            buffer = ByteArray(payload.size + packetSizeWithoutPayload)

            // Beginning byte
            buffer[index++] = 2
            // Length byte
            buffer[index++] = payload.size.toByte()
        } else {
            buffer = ByteArray(payload.size + 1 + packetSizeWithoutPayload)

            // Beginning byte
            buffer[index++] = 3
            // Length bytes (2 bytes for longer payloads)
            buffer[index++] = (payload.size shr 8).toByte()
            buffer[index++] = (payload.size and 0xff).toByte()
        }

        // Copy payload
        for (i in payload.indices) {
            buffer[index++] = payload[i]
        }

        // Calculate CRC16
        val crc = calculateCrc16(payload)

        // Write CRC16 (2 bytes)
        buffer[index++] = (crc shr 8).toByte()
        buffer[index++] = (crc and 0xff).toByte()

        // End byte
        buffer[index] = 3

        return buffer
    }

    private fun calculateCrc16(buffer: ByteArray): Int {
        var checksum = 0

        for (byte in buffer) {
            checksum =
                    CRC_16_TABLE[((checksum shr 8) xor (byte.toInt() and 0xff)) and 0xff] xor
                            (checksum shl 8)
            checksum = checksum and 0xffff // Keep it as 16-bit value
        }

        return checksum
    }
}
