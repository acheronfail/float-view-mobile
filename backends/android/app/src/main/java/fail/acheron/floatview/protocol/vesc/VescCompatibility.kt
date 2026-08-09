package fail.acheron.floatview.protocol.vesc

import fail.acheron.floatview.application.FirmwareRecord

data class VescFirmwareVersion(val major: Int, val minor: Int)

enum class VescAccessMode { READ_ONLY, READ_WRITE }

enum class VescOperation { READ, WRITE }

object VescCompatibility {
    val supported: Set<VescFirmwareVersion> = setOf(
        VescFirmwareVersion(6, 2),
        VescFirmwareVersion(6, 5),
        VescFirmwareVersion(6, 6),
        VescFirmwareVersion(7, 0),
    )

    fun accessMode(firmware: FirmwareRecord): VescAccessMode? {
        if (firmware.hardwareType != 0 ||
            VescFirmwareVersion(firmware.major, firmware.minor) !in supported) {
            return null
        }
        return if (firmware.testVersion == 0) {
            VescAccessMode.READ_WRITE
        } else {
            VescAccessMode.READ_ONLY
        }
    }

    fun supports(firmware: FirmwareRecord): Boolean = accessMode(firmware) != null

    fun permits(operation: VescOperation, firmware: FirmwareRecord): Boolean {
        val mode = accessMode(firmware) ?: return false
        return operation == VescOperation.READ || mode == VescAccessMode.READ_WRITE
    }

    fun supportsShutdown(firmware: FirmwareRecord): Boolean =
        accessMode(firmware) == VescAccessMode.READ_WRITE &&
            VescFirmwareVersion(firmware.major, firmware.minor) in setOf(
                VescFirmwareVersion(6, 5),
                VescFirmwareVersion(6, 6),
                VescFirmwareVersion(7, 0),
            )
}
