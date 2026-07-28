package fail.acheron.floatview.storage

enum class StorageLevel { NORMAL, WARNING, CRITICAL, STOP }

object StoragePolicy {
    const val WARNING_BYTES = 1_073_741_824L
    const val CRITICAL_BYTES = 524_288_000L
    const val STOP_BYTES = 262_144_000L

    fun level(availableBytes: Long): StorageLevel = when {
        availableBytes < STOP_BYTES -> StorageLevel.STOP
        availableBytes < CRITICAL_BYTES -> StorageLevel.CRITICAL
        availableBytes < WARNING_BYTES -> StorageLevel.WARNING
        else -> StorageLevel.NORMAL
    }
}
