package fail.acheron.floatview.protocol.refloat

import fail.acheron.floatview.backend.RefloatTelemetryKind
import fail.acheron.floatview.backend.RefloatTelemetryRecord
import fail.acheron.floatview.backend.RideState
import java.nio.ByteBuffer
import java.nio.charset.CodingErrorAction
import kotlin.math.pow

class RefloatDynamicTelemetryDecoder {
    var fields: List<String> = emptyList()
        private set
    var runtimeFields: List<String> = emptyList()
        private set

    fun reset() {
        fields = emptyList()
        runtimeFields = emptyList()
    }

    fun decodeFields(payload: ByteArray): Pair<List<String>, List<String>> {
        require(payload.size >= 5) { "Truncated Refloat field list" }
        var index = 3
        var aggregateBytes = 0
        fun list(): List<String> {
            require(index < payload.size) { "Truncated Refloat field list" }
            val count = payload[index++].toInt() and 0xff
            return List(count) {
                require(index < payload.size) { "Truncated Refloat field ID" }
                val length = payload[index++].toInt() and 0xff
                require(length in 1..MAX_FIELD_BYTES && index + length <= payload.size) {
                    "Invalid Refloat field ID"
                }
                val bytes = payload.copyOfRange(index, index + length)
                val value = Charsets.UTF_8.newDecoder()
                    .onMalformedInput(CodingErrorAction.REPORT)
                    .onUnmappableCharacter(CodingErrorAction.REPORT)
                    .decode(ByteBuffer.wrap(bytes))
                    .toString()
                index += length
                aggregateBytes += length
                require(aggregateBytes <= MAX_AGGREGATE_FIELD_BYTES) {
                    "Refloat field names exceed aggregate bound"
                }
                value
            }
        }
        val decoded = list() to list()
        val all = decoded.first + decoded.second
        require(index == payload.size && all.size <= MAX_FIELDS && all.toSet().size == all.size) {
            "Invalid Refloat field layout"
        }
        fields = decoded.first
        runtimeFields = decoded.second
        return decoded
    }

    fun decodeData(payload: ByteArray): RefloatTelemetryRecord {
        require(fields.isNotEmpty()) { "Refloat telemetry IDs are unavailable" }
        require(payload.size >= 13) { "Truncated Refloat realtime response" }
        val mask = payload[3].toInt() and 0xff
        val includedRuntimeFields = if (mask and 0x1 != 0) runtimeFields else emptyList()
        var expected = 13 + (fields.size + includedRuntimeFields.size) * 2
        if (mask and 0x2 != 0) expected += 4
        if (mask and 0x4 != 0) expected += 9
        require(payload.size == expected) { "Inconsistent Refloat realtime response" }

        var index = 13
        val values = linkedMapOf<String, Double>()
        for (field in fields + includedRuntimeFields) {
            values[field] = decodeFloat16(payload, index)
            index += 2
        }
        var chargingCurrent: Double? = null
        var chargingVoltage: Double? = null
        if (mask and 0x2 != 0) {
            chargingCurrent = decodeFloat16(payload, index)
            chargingVoltage = decodeFloat16(payload, index + 2)
            index += 4
        }
        var firmwareFault = null as fail.acheron.floatview.backend.FirmwareFault?
        if (mask and 0x4 != 0) {
            val activeAlerts =
                ((payload[index].toLong() and 0xff) shl 24) or
                    ((payload[index + 1].toLong() and 0xff) shl 16) or
                    ((payload[index + 2].toLong() and 0xff) shl 8) or
                    (payload[index + 3].toLong() and 0xff)
            if (activeAlerts and 1L != 0L) {
                firmwareFault = RefloatTelemetryMappings.firmwareFault(payload[index + 8].toInt() and 0xff)
            }
        }
        val extensions = values.filterKeys { it !in KNOWN_FIELDS }
        require(extensions.size <= MAX_EXTENSIONS) { "Too many Refloat extension fields" }
        val stateCode = payload[9].toInt() and 0x03
        val packageState = RefloatTelemetryMappings.packageState(stateCode)
        val flags = payload[10].toInt() and 0xff
        return RefloatTelemetryRecord(
            kind = RefloatTelemetryKind.REALTIME,
            rideState = if (packageState.wireValue == "running") RideState.RIDING else RideState.IDLE,
            packageState = packageState,
            packageMode = RefloatTelemetryMappings.packageMode((payload[9].toInt() ushr 4) and 0x03),
            footpadState = RefloatTelemetryMappings.footpad((flags ushr 6) and 0x03),
            stopCondition = RefloatTelemetryMappings.stopCondition(payload[11].toInt() and 0x0f),
            setpointAdjustment =
                RefloatTelemetryMappings.setpointAdjustment((payload[11].toInt() ushr 4) and 0x0f),
            wheelslip = flags and 0x01 != 0,
            darkride = flags and 0x02 != 0,
            charging = flags and 0x20 != 0,
            alertReason = RefloatTelemetryMappings.alertReason(payload[12].toInt() and 0xff),
            firmwareFault = firmwareFault,
            packVoltageV = values["motor.batt_voltage"],
            dutyCycle = values["motor.duty_cycle"],
            controllerTempC = values["motor.mosfet_temp"],
            motorTempC = values["motor.motor_temp"],
            motorCurrentA = values["motor.current"],
            batteryCurrentA = values["motor.batt_current"],
            erpm = values["motor.erpm"],
            speedKph = values["motor.speed"]?,
            pitchDeg = values["imu.pitch"],
            balancePitchDeg = values["imu.balance_pitch"],
            rollDeg = values["imu.roll"],
            directionalMotorCurrentA = values["motor.dir_current"],
            filteredMotorCurrentA = values["motor.filt_current"],
            requestedBalanceCurrentA = values["balance_current"],
            boosterCurrentA = values["booster.current"],
            footpadAdc1V = values["footpad.adc1"],
            footpadAdc2V = values["footpad.adc2"],
            remoteInput = values["remote.input"],
            setpointDeg = values["setpoint"],
            atrSetpointDeg = values["atr.setpoint"],
            brakeTiltSetpointDeg = values["brake_tilt.setpoint"],
            torqueTiltSetpointDeg = values["torque_tilt.setpoint"],
            turnTiltSetpointDeg = values["turn_tilt.setpoint"],
            remoteSetpointDeg = values["remote.setpoint"],
            atrAccelerationDifferenceErpmPerSecond = values["atr.accel_diff"],
            atrSpeedBoost = values["atr.speed_boost"],
            chargingCurrentA = chargingCurrent,
            chargingVoltageV = chargingVoltage,
            extensions = extensions,
        )
    }

    private fun decodeFloat16(payload: ByteArray, index: Int): Double {
        val bits =
            ((payload[index].toInt() and 0xff) shl 8) or
                (payload[index + 1].toInt() and 0xff)
        val sign = if (bits and 0x8000 == 0) 1.0 else -1.0
        val exponent = bits ushr 10 and 0x1f
        val fraction = bits and 0x03ff
        require(exponent != 0x1f) { "Non-finite Refloat value" }
        val value = if (exponent == 0) {
            sign * fraction * 2.0.pow(-24)
        } else {
            sign * (1.0 + fraction / 1024.0) * 2.0.pow(exponent - 15)
        }
        require(value.isFinite()) { "Non-finite Refloat value" }
        return value
    }

    private companion object {
        const val MAX_FIELDS = 64
        const val MAX_FIELD_BYTES = 64
        const val MAX_AGGREGATE_FIELD_BYTES = 4_096
        const val MAX_EXTENSIONS = 32
        val KNOWN_FIELDS = setOf(
            "motor.speed", "motor.erpm", "motor.current", "motor.dir_current",
            "motor.filt_current", "motor.duty_cycle", "motor.batt_voltage",
            "motor.batt_current", "motor.mosfet_temp", "motor.motor_temp", "imu.pitch",
            "imu.balance_pitch", "imu.roll", "footpad.adc1", "footpad.adc2", "remote.input",
            "setpoint", "atr.setpoint", "brake_tilt.setpoint", "torque_tilt.setpoint",
            "turn_tilt.setpoint", "remote.setpoint", "balance_current", "atr.accel_diff",
            "atr.speed_boost", "booster.current",
        )
    }
}
