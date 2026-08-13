package fail.acheron.floatview.protocol

import fail.acheron.floatview.protocol.refloat.RefloatCompatibility
import fail.acheron.floatview.protocol.refloat.RefloatCompatibilityDecision

internal val refloatRequiredFields = listOf(
    "motor.speed", "motor.erpm", "motor.current", "motor.dir_current",
    "motor.filt_current", "motor.duty_cycle", "motor.batt_voltage",
    "motor.batt_current", "motor.mosfet_temp", "motor.motor_temp", "imu.pitch",
    "imu.balance_pitch", "imu.roll", "footpad.adc1", "footpad.adc2", "remote.input",
)

internal val refloatRequiredRuntimeFields = listOf(
    "setpoint", "atr.setpoint", "brake_tilt.setpoint", "torque_tilt.setpoint",
    "turn_tilt.setpoint", "remote.setpoint", "balance_current", "atr.accel_diff",
    "atr.speed_boost", "booster.current",
)

internal fun refloatInfoPayload(version: String): ByteArray {
    if (version == "1.0") return byteArrayOf(36, 101, 0, 10)
    val components = version.split('.').map(String::toInt)
    require(components.size == 3 && components.all { it in 0..255 })
    return ByteArray(61).also {
        byteArrayOf(36, 101, 0, 2, 0).copyInto(it)
        it[25] = components[0].toByte()
        it[26] = components[1].toByte()
        it[27] = components[2].toByte()
    }
}

internal fun refloatCompatibilityDecision(version: String): RefloatCompatibilityDecision =
    RefloatCompatibility.decodeInfo(refloatInfoPayload(version))
