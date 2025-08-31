package fail.acheron.floatview

import android.annotation.SuppressLint
import android.bluetooth.BluetoothGatt
import android.bluetooth.BluetoothGattCharacteristic
import android.bluetooth.BluetoothGattDescriptor
import android.bluetooth.BluetoothGattCallback
import android.util.Log

val COMM_FW_VERSION = 0
val COMM_FORWARD_CAN = 34

class VescCommands(private val gatt: BluetoothGatt) {

    interface CommandCallback {
        fun onResponse(command: Int, data: ByteArray)
        fun onError(command: Int, message: String)
    }

    private val logTag = "FloatView_VescCommands"
    private val commandCallbacks = mutableMapOf<Int, CommandCallback>()

    // Cache characteristics for better performance
    private val writeCharacteristic: BluetoothGattCharacteristic by lazy {
        gatt.getService(VescBle.VescService).getCharacteristic(VescBle.VescCharacteristicsRx)
    }

    private val readCharacteristic: BluetoothGattCharacteristic by lazy {
        gatt.getService(VescBle.VescService).getCharacteristic(VescBle.VescCharacteristicsTx)
    }

    @SuppressLint("MissingPermission")
    fun enableNotifications(): Boolean {
        try {
            val char = readCharacteristic
            val success = gatt.setCharacteristicNotification(char, true)
            if (!success) {
                Log.e(logTag, "Failed to enable notifications")
                return false
            }

            val descriptor = char.getDescriptor(VescBle.BleClientCharacteristicConfig)
            if (descriptor == null) {
                Log.e(logTag, "Notification descriptor not found")
                return false
            }

            val writeSuccess = gatt.writeDescriptor(descriptor, BluetoothGattDescriptor.ENABLE_NOTIFICATION_VALUE)
            if (writeSuccess != BluetoothGatt.GATT_SUCCESS) {
                Log.e(logTag, "Failed to write notification descriptor")
                return false
            }

            Log.d(logTag, "Notifications enabled successfully")
            return true
        } catch (e: Exception) {
            Log.e(logTag, "Error enabling notifications: ${e.message}")
            return false
        }
    }

    fun onCharacteristicChanged(characteristic: BluetoothGattCharacteristic, value: ByteArray) {
        if (characteristic.uuid == VescBle.VescCharacteristicsTx) {
            Log.d(logTag, "Received data: ${value.joinToString(" ") { "%02X".format(it) }}")

            // Parse the VESC packet to extract the command and payload
            val parsedCommand = parseVescResponse(value)
            if (parsedCommand != null) {
                val (command, data) = parsedCommand
                // Call the appropriate callback for this command
                commandCallbacks[command]?.onResponse(command, data)
                    ?: Log.w(logTag, "No callback registered for command: $command")
            } else {
                Log.w(logTag, "Failed to parse VESC response")
            }
        }
    }

    private fun parseVescResponse(data: ByteArray): Pair<Int, ByteArray>? {
        try {
            if (data.size < 5) return null // Minimum packet size

            var index = 0

            // Check start byte
            if (data[index] != 2.toByte() && data[index] != 3.toByte()) return null
            val isLongPacket = data[index] == 3.toByte()
            index++

            // Read length
            val payloadLength = if (isLongPacket) {
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

            return Pair(command, payload)
        } catch (e: Exception) {
            Log.e(logTag, "Error parsing VESC response: ${e.message}")
            return null
        }
    }

    fun monitor() {
        val char = writeCharacteristic
        gatt.setCharacteristicNotification(char, true)

        val descriptor = char.getDescriptor(VescBle.BleClientCharacteristicConfig)
        descriptor?.let { desc ->
            gatt.writeDescriptor(desc, BluetoothGattDescriptor.ENABLE_NOTIFICATION_VALUE)
        }
    }

    fun getFwVersion(callback: CommandCallback) {
        // Register the callback for this command
        commandCallbacks[COMM_FW_VERSION] = callback

        // TODO: detect if need to use can or not, rather than hardcoding
        val canId = 1
        val payload = mutableListOf(COMM_FORWARD_CAN, canId, COMM_FW_VERSION)
        val packet = generateVescPacket(payload)

        val characteristic = writeCharacteristic
        val result = gatt.writeCharacteristic(characteristic, packet, BluetoothGattCharacteristic.WRITE_TYPE_NO_RESPONSE)

        if (result != BluetoothGatt.GATT_SUCCESS) {
            Log.e(logTag, "Failed to write firmware version command")
            callback.onError(COMM_FW_VERSION, "Failed to send command")
        } else {
            Log.d(logTag, "Firmware version command sent successfully")
        }
    }

    /**
     * Clean up resources and clear callbacks
     */
    fun cleanup() {
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
