package fail.acheron.floatview

import android.annotation.SuppressLint
import android.bluetooth.BluetoothGatt
import android.bluetooth.BluetoothGattCharacteristic
import android.bluetooth.BluetoothStatusCodes
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

class VescBlePacketLayer(
        private val gatt: BluetoothGatt,
        private val vescRx: BluetoothGattCharacteristic
) {
    val vescParser = VescCommandParser()
    private val commandQueue = ArrayDeque<Pair<List<Int>, Int>>()
    private val scope = CoroutineScope(Dispatchers.Main + SupervisorJob())
    private var commandTimer: Job? = null
    private var isProcessingCommand = false

    fun send(payload: List<Int>, writeType: Int = BluetoothGattCharacteristic.WRITE_TYPE_DEFAULT) {
        commandQueue.add(Pair(payload, writeType))
        processNextCommand()
    }

    fun parseResponse(bytes: ByteArray) {
        // If we've any queued commands send them now
        try {
            val (payload, writeType) = commandQueue.removeLast()
            sendPacket(payload, writeType)
        } catch (e: NoSuchElementException) {
            // No more commands to send
        }

        val parsed = parseBlePacket(bytes)
        if (parsed != null) {
            val (command, response) = parsed
            val buf = VescBuffer(response)
            when (CommPacketId.fromInt(command)) {
                CommPacketId.COMM_FW_VERSION ->
                        LocalServer.broadcastMessage(
                                BackendMessage.vescFirmware(
                                        gatt.device.address,
                                        vescParser.firmware(buf)
                                )
                        )
                CommPacketId.COMM_CUSTOM_APP_DATA -> {
                    buf.readUInt8() // appId
                    val refloatPacketId = RefloatCommPacketId.fromInt(buf.readInt8())
                    if (refloatPacketId == null) {
                        return LocalServer.broadcastMessage(
                                BackendMessage.commandError("Unknown Refloat packet ID: $command")
                        )
                    }

                    when (refloatPacketId) {
                        RefloatCommPacketId.COMMAND_GET_RTDATA ->
                                LocalServer.broadcastMessage(
                                        BackendMessage.refloatRtData(
                                                gatt.device.address,
                                                vescParser.refloatRtData(buf)
                                        )
                                )
                        else ->
                                LocalServer.broadcastMessage(
                                        BackendMessage.commandError(
                                                "No parser for app command: $command"
                                        )
                                )
                    }
                }
                else ->
                        LocalServer.broadcastMessage(
                                BackendMessage.commandError("No parser for command: $command")
                        )
            }
        } else {
            LocalServer.broadcastMessage(
                    BackendMessage.commandError("Failed to parse VESC response")
            )
        }
    }

    private fun processNextCommand() {
        if (isProcessingCommand || commandQueue.isEmpty()) return

        val (payload, writeType) = commandQueue.removeLast()
        isProcessingCommand = true
        sendPacket(payload, writeType)
    }

    private fun handleCommandComplete() {
        commandTimer?.cancel()
        commandTimer = null
        isProcessingCommand = false
        processNextCommand()
    }

    @SuppressLint("NewApi")
    private fun sendPacket(payload: List<Int>, writeType: Int = BluetoothGattCharacteristic.WRITE_TYPE_DEFAULT) {
        val packet = createBlePacket(payload)
        print("Command payload: ${payload.joinToString(" ") { "%02X".format(it) }}")
        print("Command packet: ${packet.joinToString(" ") { "%02X".format(it) }}")
        print("Write Type: $writeType")

        try {
            // we write the data here, and we will receive the response in the
            // `onCharacteristicChanged` callback function in the `VescManager` class
            val result = gatt.writeCharacteristic(vescRx, packet, writeType)

            if (result != BluetoothStatusCodes.SUCCESS) {
                LocalServer.broadcastMessage(
                        BackendMessage.commandError("Failed to write characteristic: $result")
                )
            }
        } catch (error: SecurityException) {
            LocalServer.broadcastMessage(
                    BackendMessage.commandError("Permission denied: ${error.message}")
            )
        }

        commandTimer = scope.launch {
            delay(1000L)
            handleCommandComplete()
        }
    }

    private fun parseBlePacket(data: ByteArray): Pair<Int, ByteArray>? {
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
    }

    private fun createBlePacket(data: List<Int>): ByteArray {
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
