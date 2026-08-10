package fail.acheron.floatview.bluetooth

import fail.acheron.floatview.application.DeviceId

internal object VescBluetoothPolicy {
    enum class WriteMode {
        WITH_RESPONSE,
        WITHOUT_RESPONSE,
        UNSUPPORTED,
    }

    enum class NotificationSetup {
        REJECT,
        COMPATIBILITY_READY,
        WRITE_DESCRIPTOR,
    }

    const val MAXIMUM_PENDING_WRITES = 128
    const val MAXIMUM_PENDING_BYTES = 65_536

    fun shouldPublish(
        advertisesUart: Boolean,
        deviceId: DeviceId,
        savedDeviceIds: Set<DeviceId>,
    ): Boolean = advertisesUart || deviceId in savedDeviceIds

    fun chunks(bytes: ByteArray, maximumPayloadBytes: Int): List<ByteArray> {
        val chunkSize = maximumPayloadBytes.coerceAtLeast(1)
        return bytes.asList().chunked(chunkSize).map { it.toByteArray() }
    }

    fun writeMode(
        supportsWriteWithResponse: Boolean,
        supportsWriteWithoutResponse: Boolean,
    ): WriteMode =
        when {
            supportsWriteWithResponse -> WriteMode.WITH_RESPONSE
            supportsWriteWithoutResponse -> WriteMode.WITHOUT_RESPONSE
            else -> WriteMode.UNSUPPORTED
        }

    fun writeCompletionRequired(writeMode: WriteMode): Boolean =
        writeMode == WriteMode.WITH_RESPONSE

    fun setupFailure(
        hasService: Boolean,
        hasWriteCharacteristic: Boolean,
        writeMode: WriteMode,
        hasNotifyCharacteristic: Boolean,
        hasNotifyProperty: Boolean,
    ): String? =
        when {
            !hasService -> "vesc_gatt_profile_unavailable"
            !hasWriteCharacteristic ||
                writeMode == WriteMode.UNSUPPORTED ||
                !hasNotifyCharacteristic ||
                !hasNotifyProperty -> "vesc_gatt_profile_unavailable"
            else -> null
        }

    fun acceptsCallback(
        callbackGeneration: Long,
        activeGeneration: Long?,
        isCancelled: Boolean,
    ): Boolean = !isCancelled && activeGeneration == callbackGeneration

    fun acceptsQueuedWrite(
        pendingWrites: Int,
        pendingBytes: Int,
        newWrites: Int,
        newBytes: Int,
    ): Boolean {
        if (
            pendingWrites !in 0..MAXIMUM_PENDING_WRITES ||
            pendingBytes !in 0..MAXIMUM_PENDING_BYTES ||
            newWrites < 0 ||
            newBytes < 0
        ) {
            return false
        }
        return newWrites <= MAXIMUM_PENDING_WRITES - pendingWrites &&
            newBytes <= MAXIMUM_PENDING_BYTES - pendingBytes
    }

    fun notificationSetup(
        hasNotifyProperty: Boolean,
        localNotificationEnabled: Boolean,
        hasDescriptor: Boolean,
    ): NotificationSetup =
        when {
            !hasNotifyProperty || !localNotificationEnabled -> NotificationSetup.REJECT
            !hasDescriptor -> NotificationSetup.COMPATIBILITY_READY
            else -> NotificationSetup.WRITE_DESCRIPTOR
        }
}
