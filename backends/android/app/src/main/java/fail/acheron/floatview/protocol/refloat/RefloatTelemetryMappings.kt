package fail.acheron.floatview.protocol.refloat

import fail.acheron.floatview.application.AlertReason
import fail.acheron.floatview.application.FirmwareFault
import fail.acheron.floatview.application.FootpadState
import fail.acheron.floatview.application.PackageMode
import fail.acheron.floatview.application.PackageState
import fail.acheron.floatview.application.SetpointAdjustment
import fail.acheron.floatview.application.StopCondition

internal object RefloatTelemetryMappings {
    fun packageState(code: Int) =
        PackageState.entries.getOrNull(code) ?: PackageState.UNKNOWN

    fun packageMode(code: Int) =
        listOf(PackageMode.NORMAL, PackageMode.HAND_TEST, PackageMode.FLYWHEEL)
            .getOrNull(code) ?: PackageMode.UNKNOWN

    fun footpad(code: Int) =
        listOf(FootpadState.NONE, FootpadState.LEFT, FootpadState.RIGHT, FootpadState.BOTH)
            .getOrNull(code) ?: FootpadState.UNKNOWN

    fun stopCondition(code: Int) =
        listOf(
            StopCondition.NONE,
            StopCondition.PITCH,
            StopCondition.ROLL,
            StopCondition.SWITCH_HALF,
            StopCondition.SWITCH_FULL,
            StopCondition.REVERSE_STOP,
            StopCondition.QUICK_STOP,
        ).getOrNull(code) ?: StopCondition.UNKNOWN

    fun setpointAdjustment(code: Int) = when (code) {
        0 -> SetpointAdjustment.NONE
        1 -> SetpointAdjustment.CENTERING
        2 -> SetpointAdjustment.REVERSE_STOP
        5 -> SetpointAdjustment.SPEED
        6 -> SetpointAdjustment.DUTY
        7 -> SetpointAdjustment.ERROR
        10 -> SetpointAdjustment.HIGH_VOLTAGE
        11 -> SetpointAdjustment.LOW_VOLTAGE
        12 -> SetpointAdjustment.TEMPERATURE
        else -> SetpointAdjustment.UNKNOWN
    }

    fun alertReason(code: Int) =
        AlertReason.entries.getOrNull(code) ?: AlertReason.UNKNOWN

    fun firmwareFault(code: Int): FirmwareFault? {
        if (code == 0) return null
        return listOf(
            FirmwareFault.UNKNOWN,
            FirmwareFault.OVER_VOLTAGE,
            FirmwareFault.UNDER_VOLTAGE,
            FirmwareFault.DRIVER,
            FirmwareFault.ABSOLUTE_OVER_CURRENT,
            FirmwareFault.CONTROLLER_OVER_TEMPERATURE,
            FirmwareFault.MOTOR_OVER_TEMPERATURE,
            FirmwareFault.GATE_DRIVER_OVER_VOLTAGE,
            FirmwareFault.GATE_DRIVER_UNDER_VOLTAGE,
            FirmwareFault.MCU_UNDER_VOLTAGE,
            FirmwareFault.WATCHDOG_RESET,
            FirmwareFault.ENCODER_SPI,
            FirmwareFault.ENCODER_SINCOS_BELOW_MINIMUM,
            FirmwareFault.ENCODER_SINCOS_ABOVE_MAXIMUM,
            FirmwareFault.FLASH_CORRUPTION,
            FirmwareFault.CURRENT_SENSOR_OFFSET_1,
            FirmwareFault.CURRENT_SENSOR_OFFSET_2,
            FirmwareFault.CURRENT_SENSOR_OFFSET_3,
            FirmwareFault.UNBALANCED_CURRENTS,
            FirmwareFault.BRAKE,
            FirmwareFault.RESOLVER_LOSS_OF_TRACKING,
            FirmwareFault.RESOLVER_DEGRADATION_OF_SIGNAL,
            FirmwareFault.RESOLVER_LOSS_OF_SIGNAL,
            FirmwareFault.APP_CONFIG_FLASH_CORRUPTION,
            FirmwareFault.MOTOR_CONFIG_FLASH_CORRUPTION,
            FirmwareFault.ENCODER_NO_MAGNET,
            FirmwareFault.ENCODER_MAGNET_TOO_STRONG,
            FirmwareFault.PHASE_FILTER,
        ).getOrNull(code) ?: FirmwareFault.UNKNOWN
    }
}
