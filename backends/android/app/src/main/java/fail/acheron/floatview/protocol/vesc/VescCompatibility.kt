package fail.acheron.floatview.protocol.vesc

import fail.acheron.floatview.backend.FirmwareRecord

data class VescFirmwareVersion(val major: Int, val minor: Int)

object VescCompatibility {
    val supported: Set<VescFirmwareVersion> = setOf(
        VescFirmwareVersion(6, 2),
        VescFirmwareVersion(6, 5),
        VescFirmwareVersion(6, 6),
    )

    fun supports(firmware: FirmwareRecord): Boolean =
        VescFirmwareVersion(firmware.major, firmware.minor) in supported
}
