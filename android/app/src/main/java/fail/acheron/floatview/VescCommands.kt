package fail.acheron.floatview

import android.annotation.SuppressLint
import android.bluetooth.BluetoothGatt
import android.bluetooth.BluetoothGattCharacteristic
import android.bluetooth.BluetoothGattDescriptor
import android.bluetooth.BluetoothGattCallback
import android.bluetooth.BluetoothManager
import android.bluetooth.BluetoothProfile
import android.os.Build
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
    private val writeCharacteristic: BluetoothGattCharacteristic? by lazy {
        try {
            gatt.getService(VescBle.VescService)?.getCharacteristic(VescBle.VescCharacteristicsRx)
        } catch (e: Exception) {
            Log.e(logTag, "Error getting write characteristic: ${e.message}")
            null
        }
    }

    private val readCharacteristic: BluetoothGattCharacteristic? by lazy {
        try {
            gatt.getService(VescBle.VescService)?.getCharacteristic(VescBle.VescCharacteristicsTx)
        } catch (e: Exception) {
            Log.e(logTag, "Error getting read characteristic: ${e.message}")
            null
        }
    }

    @SuppressLint("MissingPermission")
    fun enableNotifications(): Boolean {
        try {
            val char = readCharacteristic
            if (char == null) {
                Log.e(logTag, "Read characteristic not available")
                return false
            }

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

            val writeSuccess = if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.TIRAMISU) {
                gatt.writeDescriptor(descriptor, BluetoothGattDescriptor.ENABLE_NOTIFICATION_VALUE)
            } else {
                descriptor.value = BluetoothGattDescriptor.ENABLE_NOTIFICATION_VALUE
                gatt.writeDescriptor(descriptor)
            }

            when (writeSuccess) {
                BluetoothGatt.GATT_SUCCESS, true -> {
                    Log.d(logTag, "Notifications enabled successfully")
                    return true
                }
                else -> {
                    Log.e(logTag, "Failed to write notification descriptor, result: $writeSuccess")
                    return false
                }
            }
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

    @SuppressLint("MissingPermission")
    fun monitor() {
        try {
            val char = writeCharacteristic
            if (char == null) {
                Log.e(logTag, "Write characteristic not available for monitoring")
                return
            }

            val success = gatt.setCharacteristicNotification(char, true)
            if (!success) {
                Log.e(logTag, "Failed to set characteristic notification for monitoring")
                return
            }

            val descriptor = char.getDescriptor(VescBle.BleClientCharacteristicConfig)
            if (descriptor == null) {
                Log.e(logTag, "Descriptor not found for monitoring")
                return
            }

            val writeResult = if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.TIRAMISU) {
                gatt.writeDescriptor(descriptor, BluetoothGattDescriptor.ENABLE_NOTIFICATION_VALUE)
            } else {
                descriptor.value = BluetoothGattDescriptor.ENABLE_NOTIFICATION_VALUE
                gatt.writeDescriptor(descriptor)
            }

            when (writeResult) {
                BluetoothGatt.GATT_SUCCESS, true -> {
                    Log.d(logTag, "Monitoring enabled successfully")
                }
                else -> {
                    Log.e(logTag, "Failed to write descriptor for monitoring, result: $writeResult")
                }
            }
        } catch (e: Exception) {
            Log.e(logTag, "Exception in monitor: ${e.message}")
        }
    }

    @SuppressLint("MissingPermission")
    fun getFwVersion(callback: CommandCallback) {
        try {
            // Check if the device is connected
            // Note: We can't easily check connection state from GATT object alone
            // But we can check if services are available which indicates readiness
            if (gatt.getService(VescBle.VescService) == null) {
                Log.e(logTag, "VESC service not available - connection may not be ready")
                callback.onError(COMM_FW_VERSION, "GATT connection not ready")
                return
            }

            // Register the callback for this command
            commandCallbacks[COMM_FW_VERSION] = callback

            // TODO: detect if need to use can or not, rather than hardcoding
            val canId = 1
            val payload = mutableListOf(COMM_FORWARD_CAN, canId, COMM_FW_VERSION)
            val packet = generateVescPacket(payload)

            Log.d(logTag, "Generated packet: ${packet.joinToString(" ") { "%02X".format(it) }}")
            Log.d(logTag, "Packet size: ${packet.size} bytes")

            val characteristic = writeCharacteristic

            // Check if characteristic is available
            if (characteristic == null) {
                Log.e(logTag, "Write characteristic not available")
                callback.onError(COMM_FW_VERSION, "Write characteristic not available")
                return
            }

            // Determine appropriate write type based on characteristic properties
            val supportsWriteNoResponse = (characteristic.properties and BluetoothGattCharacteristic.PROPERTY_WRITE_NO_RESPONSE) != 0
            val supportsWrite = (characteristic.properties and BluetoothGattCharacteristic.PROPERTY_WRITE) != 0

            val writeType = when {
                supportsWrite -> BluetoothGattCharacteristic.WRITE_TYPE_DEFAULT
                supportsWriteNoResponse -> BluetoothGattCharacteristic.WRITE_TYPE_NO_RESPONSE
                else -> {
                    Log.e(logTag, "Characteristic doesn't support any write operations")
                    callback.onError(COMM_FW_VERSION, "Characteristic doesn't support write operations")
                    return
                }
            }

            Log.d(logTag, "Using write type: $writeType for characteristic with properties: ${characteristic.properties}")

            // Check if packet size is within reasonable limits (most BLE devices support at least 20 bytes)
            if (packet.size > 20) {
                Log.w(logTag, "Packet size (${packet.size}) may exceed MTU limit, consider requesting MTU increase")
            }

            // Use the newer API for Android API 33+
            val result = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                gatt.writeCharacteristic(characteristic, packet, writeType)
            } else {
                // Legacy API for older Android versions
                characteristic.value = packet
                characteristic.writeType = writeType
                gatt.writeCharacteristic(characteristic)
            }

            when (result) {
                BluetoothGatt.GATT_SUCCESS, true -> {
                    Log.d(logTag, "Firmware version command sent successfully")
                }
                else -> {
                    val errorMessage = when (result) {
                        201 -> "GATT_FAILURE - Generic failure"
                        133 -> "GATT_INTERNAL_ERROR - Internal error"
                        8 -> "GATT_INSUFFICIENT_AUTHENTICATION - Insufficient authentication"
                        15 -> "GATT_INSUFFICIENT_ENCRYPTION - Insufficient encryption"
                        143 -> "GATT_WRITE_NOT_PERMITTED - Write not permitted"
                        else -> "Unknown error code: $result"
                    }
                    Log.e(logTag, "Failed to write firmware version command - $errorMessage")
                    callback.onError(COMM_FW_VERSION, "Failed to send command: $errorMessage")
                }
            }
        } catch (e: Exception) {
            Log.e(logTag, "Exception in getFwVersion: ${e.message}")
            callback.onError(COMM_FW_VERSION, "Exception: ${e.message}")
        }
    }

    /**
     * Request a larger MTU for better performance
     */
    @SuppressLint("MissingPermission")
    fun requestMtu(mtuSize: Int = 247): Boolean {
        return try {
            val result = gatt.requestMtu(mtuSize)
            Log.d(logTag, "Requested MTU $mtuSize, result: $result")
            result
        } catch (e: Exception) {
            Log.e(logTag, "Error requesting MTU: ${e.message}")
            false
        }
    }

    /**
     * Check if GATT connection and required characteristics are available
     */
    fun isReady(): Boolean {
        try {
            val service = gatt.getService(VescBle.VescService)
            if (service == null) {
                Log.e(logTag, "VESC service not found")
                return false
            }

            val rxChar = service.getCharacteristic(VescBle.VescCharacteristicsRx)
            val txChar = service.getCharacteristic(VescBle.VescCharacteristicsTx)

            if (rxChar == null || txChar == null) {
                Log.e(logTag, "Required characteristics not found - RX: $rxChar, TX: $txChar")
                return false
            }

            // Check characteristic properties
            Log.d(logTag, "RX Characteristic properties: ${rxChar.properties}")
            Log.d(logTag, "TX Characteristic properties: ${txChar.properties}")

            // Check if RX characteristic supports write operations
            val supportsWrite = (rxChar.properties and BluetoothGattCharacteristic.PROPERTY_WRITE) != 0
            val supportsWriteNoResponse = (rxChar.properties and BluetoothGattCharacteristic.PROPERTY_WRITE_NO_RESPONSE) != 0

            Log.d(logTag, "RX supports WRITE: $supportsWrite, WRITE_NO_RESPONSE: $supportsWriteNoResponse")

            if (!supportsWrite && !supportsWriteNoResponse) {
                Log.e(logTag, "RX characteristic doesn't support write operations")
                return false
            }

            Log.d(logTag, "GATT connection is ready")
            return true
        } catch (e: Exception) {
            Log.e(logTag, "Error checking GATT readiness: ${e.message}")
            return false
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
