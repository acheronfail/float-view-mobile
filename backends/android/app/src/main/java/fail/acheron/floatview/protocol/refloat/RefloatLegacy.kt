package fail.acheron.floatview.protocol.refloat

import fail.acheron.floatview.application.AlertReason
import fail.acheron.floatview.application.FootpadState
import fail.acheron.floatview.application.PackageMode
import fail.acheron.floatview.application.PackageState
import fail.acheron.floatview.application.RefloatTelemetryKind
import fail.acheron.floatview.application.RefloatTelemetryRecord
import fail.acheron.floatview.application.RideState
import fail.acheron.floatview.application.SetpointAdjustment
import fail.acheron.floatview.application.StopCondition

object RefloatLegacyTelemetryDecoder {
    fun decode(payload: ByteArray): RefloatTelemetryRecord {
        require(payload.size >= 4) { "Truncated Refloat compact response" }
        fun unsigned(index: Int) = payload[index].toInt() and 0xff
        if (unsigned(3) == 69) {
            require(payload.size == 5) { "Invalid Refloat compact fault response" }
            return RefloatTelemetryRecord(
                kind = RefloatTelemetryKind.COMPACT,
                rideState = RideState.UNKNOWN,
                alertReason = AlertReason.FIRMWARE_FAULT,
                firmwareFault = RefloatTelemetryMappings.firmwareFault(unsigned(4)),
            )
        }
        val detailLevel = unsigned(3)
        require(detailLevel in 1..4) { "Unsupported Refloat detail level" }
        val required =
            if (detailLevel >= 4) 59 else if (detailLevel >= 3) 55 else if (detailLevel >= 2) 42 else 35
        require(payload.size == required) { "Truncated Refloat telemetry response" }
        fun signed16(index: Int): Int =
            ((unsigned(index) shl 8) or unsigned(index + 1)).toShort().toInt()
        fun unsigned32(index: Int): Long =
            ((unsigned(index).toLong() shl 24) or
                (unsigned(index + 1).toLong() shl 16) or
                (unsigned(index + 2).toLong() shl 8) or
                unsigned(index + 3).toLong())
        val legacyState = unsigned(10) and 0x0f
        val switchBits = unsigned(11) and 0x07
        val adc1 = unsigned(12) / 50.0
        val adc2 = unsigned(13) / 50.0
        val footpad = when (switchBits) {
            2 -> FootpadState.BOTH
            1 -> if (adc1 > adc2) FootpadState.LEFT else FootpadState.RIGHT
            else -> FootpadState.NONE
        }
        val (packageState, stopCondition) = legacySemanticState(legacyState)
        return RefloatTelemetryRecord(
            kind = RefloatTelemetryKind.COMPACT,
            rideState = if (packageState == PackageState.RUNNING) RideState.RIDING else RideState.IDLE,
            packageState = packageState,
            packageMode = if (unsigned(11) and 0x08 != 0) {
                PackageMode.HAND_TEST
            } else if (legacyState == 5) {
                PackageMode.FLYWHEEL
            } else {
                PackageMode.NORMAL
            },
            footpadState = footpad,
            stopCondition = stopCondition,
            setpointAdjustment = legacySetpointAdjustment(unsigned(10) ushr 4),
            wheelslip = legacyState == 3,
            darkride = legacyState == 4,
            charging = legacyState == 14,
            alertReason = RefloatTelemetryMappings.alertReason(unsigned(11) ushr 4),
            packVoltageV = signed16(23) / 10.0,
            dutyCycle = (unsigned(33) - 128) / 100.0,
            controllerTempC = if (detailLevel >= 2) unsigned(39) / 2.0 else null,
            motorTempC = if (detailLevel >= 2) unsigned(40) / 2.0 else null,
            motorCurrentA = signed16(29) / 10.0,
            batteryCurrentA = signed16(31) / 10.0,
            erpm = signed16(25).toDouble(),
            speedKph = signed16(27) / 10.0 * 3.6,
            pitchDeg = signed16(20) / 10.0,
            balancePitchDeg = signed16(6) / 10.0,
            rollDeg = signed16(8) / 10.0,
            requestedBalanceCurrentA = signed16(4) / 10.0,
            boosterCurrentA = (unsigned(22) - 128).toDouble(),
            fieldWeakeningCurrentA = unsigned(34) / 3.0,
            footpadAdc1V = adc1,
            footpadAdc2V = adc2,
            setpointDeg = (unsigned(14) - 128) / 5.0,
            atrSetpointDeg = (unsigned(15) - 128) / 5.0,
            brakeTiltSetpointDeg = (unsigned(16) - 128) / 5.0,
            torqueTiltSetpointDeg = (unsigned(17) - 128) / 5.0,
            turnTiltSetpointDeg = (unsigned(18) - 128) / 5.0,
            remoteSetpointDeg = (unsigned(19) - 128) / 5.0,
            distanceM = if (detailLevel >= 2) decodeFloat32(payload, 35) else null,
            odometerM = if (detailLevel >= 3) unsigned32(42).toDouble() else null,
            ampHoursDrawnAh = if (detailLevel >= 3) signed16(46) / 10.0 else null,
            ampHoursChargedAh = if (detailLevel >= 3) signed16(48) / 10.0 else null,
            wattHoursDrawnWh = if (detailLevel >= 3) signed16(50).toDouble() else null,
            wattHoursChargedWh = if (detailLevel >= 3) signed16(52).toDouble() else null,
            batteryLevel = if (detailLevel >= 3) unsigned(54) / 200.0 else null,
            chargingCurrentA = if (detailLevel >= 4) signed16(55) / 10.0 else null,
            chargingVoltageV = if (detailLevel >= 4) signed16(57) / 10.0 else null,
        )
    }

    private fun decodeFloat32(payload: ByteArray, index: Int): Double {
        val bits =
            ((payload[index].toInt() and 0xff) shl 24) or
                ((payload[index + 1].toInt() and 0xff) shl 16) or
                ((payload[index + 2].toInt() and 0xff) shl 8) or
                (payload[index + 3].toInt() and 0xff)
        return Float.fromBits(bits).toDouble()
    }

    private fun legacySetpointAdjustment(value: Int) = when (value) {
        0 -> SetpointAdjustment.CENTERING
        1 -> SetpointAdjustment.REVERSE_STOP
        2 -> SetpointAdjustment.NONE
        3 -> SetpointAdjustment.DUTY
        4 -> SetpointAdjustment.HIGH_VOLTAGE
        5 -> SetpointAdjustment.LOW_VOLTAGE
        6 -> SetpointAdjustment.TEMPERATURE
        else -> SetpointAdjustment.UNKNOWN
    }

    private fun legacySemanticState(value: Int): Pair<PackageState, StopCondition> = when (value) {
        0 -> PackageState.STARTUP to StopCondition.NONE
        in 1..5 -> PackageState.RUNNING to StopCondition.NONE
        6 -> PackageState.READY to StopCondition.PITCH
        7 -> PackageState.READY to StopCondition.ROLL
        8 -> PackageState.READY to StopCondition.SWITCH_HALF
        9 -> PackageState.READY to StopCondition.SWITCH_FULL
        12 -> PackageState.READY to StopCondition.REVERSE_STOP
        13 -> PackageState.READY to StopCondition.QUICK_STOP
        14 -> PackageState.READY to StopCondition.NONE
        15 -> PackageState.DISABLED to StopCondition.NONE
        else -> PackageState.READY to StopCondition.NONE
    }
}
