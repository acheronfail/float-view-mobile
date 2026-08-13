package fail.acheron.floatview.protocol.refloat

import fail.acheron.floatview.application.AlertReason
import fail.acheron.floatview.application.FirmwareFault
import fail.acheron.floatview.application.FootpadState
import fail.acheron.floatview.application.PackageMode
import fail.acheron.floatview.application.PackageState
import fail.acheron.floatview.application.RefloatTelemetryKind
import fail.acheron.floatview.application.RefloatTelemetryRecord
import fail.acheron.floatview.application.RideState
import fail.acheron.floatview.application.SetpointAdjustment
import fail.acheron.floatview.application.StopCondition
import fail.acheron.floatview.protocol.RustRefloatDynamicTelemetryBridge
import fail.acheron.floatview.protocol.VescProtocolException
import fail.acheron.floatview.protocol.verifyRefloatDynamicTelemetryAbiVersion
import java.nio.ByteBuffer
import java.nio.charset.CharacterCodingException
import java.nio.charset.CodingErrorAction
import java.nio.charset.StandardCharsets

/** Android adapter for Rust-owned Refloat command-31/32 canonical records. */
class RefloatDynamicTelemetryDecoder {
    var fields: List<String> = emptyList()
        private set
    var runtimeFields: List<String> = emptyList()
        private set
    private var layoutRecord: ByteArray? = null

    fun reset() {
        fields = emptyList()
        runtimeFields = emptyList()
        layoutRecord = null
    }

    fun decodeFields(payload: ByteArray): Pair<List<String>, List<String>> {
        val decoded = decodeFieldsWithRecord(payload)
        return decoded.fields to decoded.runtimeFields
    }

    internal fun decodeFieldsWithRecord(payload: ByteArray): DynamicTelemetryLayout {
        verifyRefloatDynamicTelemetryAbiVersion(linkedAbiVersion)
        val output = ByteArray(MAX_LAYOUT_RECORD_BYTES)
        val result = IntArray(1)
        val decoded = layoutBridgeResult(
            RustRefloatDynamicTelemetryBridge.decodeLayout(payload, output, result),
            output,
            result,
        )
        fields = decoded.fields
        runtimeFields = decoded.runtimeFields
        layoutRecord = decoded.record
        return decoded
    }

    fun decodeData(payload: ByteArray): RefloatTelemetryRecord {
        verifyRefloatDynamicTelemetryAbiVersion(linkedAbiVersion)
        val layout = layoutRecord ?: throw VescProtocolException.TruncatedPayload
        val output = ByteArray(MAX_SAMPLE_RECORD_BYTES)
        val result = IntArray(1)
        return telemetryBridgeResult(
            RustRefloatDynamicTelemetryBridge.decodeRealtime(layout, payload, output, result),
            output,
            result,
        )
    }

    internal fun layoutBridgeResult(
        status: Int,
        output: ByteArray,
        result: IntArray,
    ): DynamicTelemetryLayout {
        if (result.size != 1) fail()
        val recordLength = result[0]
        return when (status) {
            STATUS_OK -> {
                if (recordLength !in LAYOUT_HEADER_BYTES..minOf(output.size, MAX_LAYOUT_RECORD_BYTES)) fail()
                layout(output.copyOf(recordLength))
            }
            STATUS_TRUNCATED_PAYLOAD -> {
                requireEmptyFailure(recordLength)
                throw VescProtocolException.TruncatedPayload
            }
            STATUS_UNSUPPORTED_ENVELOPE,
            STATUS_INVALID_LAYOUT,
            -> {
                requireEmptyFailure(recordLength)
                throw VescProtocolException.InvalidFrame
            }
            else -> fail()
        }
    }

    internal fun telemetryBridgeResult(
        status: Int,
        output: ByteArray,
        result: IntArray,
    ): RefloatTelemetryRecord {
        if (result.size != 1) fail()
        val recordLength = result[0]
        return when (status) {
            STATUS_OK -> {
                if (recordLength !in SAMPLE_FIXED_BYTES..minOf(output.size, MAX_SAMPLE_RECORD_BYTES)) fail()
                telemetry(output.copyOf(recordLength))
            }
            STATUS_TRUNCATED_PAYLOAD -> {
                requireEmptyFailure(recordLength)
                throw VescProtocolException.TruncatedPayload
            }
            STATUS_UNSUPPORTED_ENVELOPE,
            STATUS_INVALID_LAYOUT,
            STATUS_INVALID_TELEMETRY,
            -> {
                requireEmptyFailure(recordLength)
                throw VescProtocolException.InvalidFrame
            }
            else -> fail()
        }
    }

    internal fun layout(record: ByteArray): DynamicTelemetryLayout {
        if (
            record.size !in LAYOUT_HEADER_BYTES..MAX_LAYOUT_RECORD_BYTES ||
            record[0].unsigned() != RECORD_VERSION ||
            record[1].unsigned() != LAYOUT_RECORD_KIND ||
            record[6].unsigned() != 0 ||
            record[7].unsigned() != 0
        ) fail()
        val fieldsCount = record[2].unsigned()
        val runtimeCount = record[3].unsigned()
        val expectedAggregate = record.bigEndianUnsignedShort(4)
        if (
            fieldsCount == 0 ||
            fieldsCount + runtimeCount > MAX_FIELDS ||
            expectedAggregate > MAX_AGGREGATE_FIELD_BYTES
        ) fail()
        val cursor = RecordCursor(record, LAYOUT_HEADER_BYTES)
        val fields = List(fieldsCount) { cursor.readName() }
        val runtimeFields = List(runtimeCount) { cursor.readName() }
        val all = fields + runtimeFields
        if (
            !cursor.atEnd ||
            all.sumOf { it.encodeToByteArray().size } != expectedAggregate ||
            all.distinct().size != all.size
        ) fail()
        return DynamicTelemetryLayout(fields, runtimeFields, record)
    }

    internal fun telemetry(record: ByteArray): RefloatTelemetryRecord {
        if (
            record.size !in SAMPLE_FIXED_BYTES..MAX_SAMPLE_RECORD_BYTES ||
            record[0].unsigned() != RECORD_VERSION ||
            record[1].unsigned() != SAMPLE_RECORD_KIND ||
            record[7].unsigned() and KNOWN_BEHAVIOR_FLAGS.inv() != 0 ||
            record[11].unsigned() and KNOWN_OPTION_FLAGS.inv() != 0 ||
            record.sliceArray(13..15).any { it.unsigned() != 0 }
        ) fail()

        val packageState = closed(
            record[2],
            PackageState.DISABLED,
            PackageState.STARTUP,
            PackageState.READY,
            PackageState.RUNNING,
        )
        val packageMode = closed(
            record[3],
            PackageMode.NORMAL,
            PackageMode.HAND_TEST,
            PackageMode.FLYWHEEL,
            PackageMode.UNKNOWN,
        )
        val footpadState = closed(
            record[4],
            FootpadState.NONE,
            FootpadState.LEFT,
            FootpadState.RIGHT,
            FootpadState.BOTH,
        )
        val stopCondition = closed(
            record[5],
            StopCondition.NONE,
            StopCondition.PITCH,
            StopCondition.ROLL,
            StopCondition.SWITCH_HALF,
            StopCondition.SWITCH_FULL,
            StopCondition.REVERSE_STOP,
            StopCondition.QUICK_STOP,
            StopCondition.UNKNOWN,
        )
        val adjustment = closed(
            record[6],
            SetpointAdjustment.NONE,
            SetpointAdjustment.CENTERING,
            SetpointAdjustment.REVERSE_STOP,
            SetpointAdjustment.SPEED,
            SetpointAdjustment.DUTY,
            SetpointAdjustment.ERROR,
            SetpointAdjustment.HIGH_VOLTAGE,
            SetpointAdjustment.LOW_VOLTAGE,
            SetpointAdjustment.TEMPERATURE,
            SetpointAdjustment.UNKNOWN,
        )
        val alert = closed(
            record[8],
            AlertReason.NONE,
            AlertReason.LOW_VOLTAGE,
            AlertReason.HIGH_VOLTAGE,
            AlertReason.CONTROLLER_TEMPERATURE,
            AlertReason.MOTOR_TEMPERATURE,
            AlertReason.CURRENT,
            AlertReason.DUTY,
            AlertReason.SENSORS,
            AlertReason.LOW_BATTERY,
            AlertReason.IDLE,
            AlertReason.ERROR,
            AlertReason.SPEED,
            AlertReason.CELL_UNDER_TEMPERATURE,
            AlertReason.CELL_OVER_TEMPERATURE,
            AlertReason.CELL_LOW_VOLTAGE,
            AlertReason.CELL_HIGH_VOLTAGE,
            AlertReason.CELL_IMBALANCE,
            AlertReason.BMS_CONNECTION,
            AlertReason.BMS_OVER_TEMPERATURE,
            AlertReason.FIRMWARE_FAULT,
            AlertReason.UNKNOWN,
        )

        val behavior = record[7].unsigned()
        val options = record[11].unsigned()
        val extensionCount = record[12].unsigned()
        if (extensionCount > MAX_EXTENSIONS) fail()
        val hasFatal = options and FATAL_PRESENT != 0
        val fatalValue = options and FATAL_VALUE != 0
        val hasAlerts = options and ACTIVE_ALERTS_PRESENT != 0
        if (hasFatal != hasAlerts || (!hasFatal && fatalValue)) fail()

        val word0 = record.bigEndianUnsignedInt(24)
        val word1 = record.bigEndianUnsignedInt(28)
        val rawFault = record[10].unsigned()
        val faultCode = record[9].unsigned()
        if (
            (!hasAlerts && (word0 != 0L || word1 != 0L)) ||
            ((rawFault == 0) != (faultCode == 0)) ||
            ((rawFault != 0) != (word0 and 1L != 0L)) ||
            (rawFault != 0 && faultCode != if (rawFault <= 27) rawFault else 28)
        ) fail()
        val firmwareFault = if (faultCode == 0) {
            null
        } else {
            closed(
                record[9],
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
                FirmwareFault.UNKNOWN,
            )
        }

        val presence = record.bigEndianUnsignedInt(20)
        if (presence ushr FIXED_VALUE_COUNT != 0L) fail()
        val cursor = RecordCursor(record, SAMPLE_RECORD_HEADER_BYTES)
        val values = List<Double?>(FIXED_VALUE_COUNT) { index ->
            val bits = cursor.readLong()
            if (presence and (1L shl index) == 0L) {
                if (bits != 0L) fail()
                null
            } else {
                Double.fromBits(bits).takeIf { it.isFinite() } ?: fail()
            }
        }
        val extensions = linkedMapOf<String, Double>()
        repeat(extensionCount) {
            val name = cursor.readName()
            val value = Double.fromBits(cursor.readLong()).takeIf { it.isFinite() } ?: fail()
            if (name in KNOWN_FIELDS || extensions.put(name, value) != null) fail()
        }
        val charging = behavior and CHARGING != 0
        if (
            !cursor.atEnd ||
            charging != (values[26] != null && values[27] != null) ||
            (values[26] == null) != (values[27] == null)
        ) fail()

        return RefloatTelemetryRecord(
            kind = RefloatTelemetryKind.REALTIME,
            rideState = if (packageState == PackageState.RUNNING) RideState.RIDING else RideState.IDLE,
            packageState = packageState,
            packageMode = packageMode,
            footpadState = footpadState,
            stopCondition = stopCondition,
            setpointAdjustment = adjustment,
            wheelslip = behavior and WHEELSLIP != 0,
            darkride = behavior and DARKRIDE != 0,
            charging = charging,
            alertReason = alert,
            firmwareFault = firmwareFault,
            fatalErrorLatched = if (hasFatal) fatalValue else null,
            packVoltageV = values[6],
            dutyCycle = values[5],
            controllerTempC = values[8],
            motorTempC = values[9],
            motorCurrentA = values[2],
            batteryCurrentA = values[7],
            erpm = values[1],
            speedKph = values[0],
            pitchDeg = values[10],
            balancePitchDeg = values[11],
            rollDeg = values[12],
            directionalMotorCurrentA = values[3],
            filteredMotorCurrentA = values[4],
            requestedBalanceCurrentA = values[22],
            boosterCurrentA = values[25],
            footpadAdc1V = values[13],
            footpadAdc2V = values[14],
            remoteInput = values[15],
            setpointDeg = values[16],
            atrSetpointDeg = values[17],
            brakeTiltSetpointDeg = values[18],
            torqueTiltSetpointDeg = values[19],
            turnTiltSetpointDeg = values[20],
            remoteSetpointDeg = values[21],
            atrAccelerationDifferenceErpmPerSecond = values[23],
            atrSpeedBoost = values[24],
            chargingCurrentA = values[26],
            chargingVoltageV = values[27],
            extensions = extensions,
        )
    }

    private fun requireEmptyFailure(recordLength: Int) {
        if (recordLength != 0) fail()
    }

    private fun <T> closed(code: Byte, vararg values: T): T =
        values.getOrNull(code.unsigned() - 1) ?: fail()

    private fun fail(): Nothing = throw VescProtocolException.TruncatedPayload

    internal data class DynamicTelemetryLayout(
        val fields: List<String>,
        val runtimeFields: List<String>,
        val record: ByteArray,
    ) {
        override fun equals(other: Any?): Boolean =
            other is DynamicTelemetryLayout &&
                fields == other.fields &&
                runtimeFields == other.runtimeFields &&
                record.contentEquals(other.record)

        override fun hashCode(): Int = 31 * (31 * fields.hashCode() + runtimeFields.hashCode()) + record.contentHashCode()
    }

    private class RecordCursor(
        private val bytes: ByteArray,
        private var index: Int,
    ) {
        val atEnd: Boolean get() = index == bytes.size

        fun readName(): String {
            val length = readByte().unsigned()
            if (length !in 1..MAX_FIELD_BYTES || index > bytes.size - length) invalid()
            val value = bytes.strictUtf8(index, index + length)
            index += length
            return value
        }

        fun readLong(): Long {
            if (index > bytes.size - Long.SIZE_BYTES) invalid()
            var value = 0L
            repeat(Long.SIZE_BYTES) { value = (value shl 8) or bytes[index++].unsigned().toLong() }
            return value
        }

        private fun readByte(): Byte {
            if (index >= bytes.size) invalid()
            return bytes[index++]
        }

        private fun invalid(): Nothing = throw VescProtocolException.TruncatedPayload
    }

    private companion object {
        const val RECORD_VERSION = 1
        const val LAYOUT_RECORD_KIND = 1
        const val SAMPLE_RECORD_KIND = 2
        const val LAYOUT_HEADER_BYTES = 8
        const val SAMPLE_RECORD_HEADER_BYTES = 32
        const val FIXED_VALUE_COUNT = 28
        const val SAMPLE_FIXED_BYTES = SAMPLE_RECORD_HEADER_BYTES + FIXED_VALUE_COUNT * Long.SIZE_BYTES
        const val MAX_FIELDS = 64
        const val MAX_FIELD_BYTES = 64
        const val MAX_AGGREGATE_FIELD_BYTES = 4_096
        const val MAX_EXTENSIONS = 32
        const val MAX_LAYOUT_RECORD_BYTES = LAYOUT_HEADER_BYTES + MAX_FIELDS + MAX_AGGREGATE_FIELD_BYTES
        const val MAX_SAMPLE_RECORD_BYTES = SAMPLE_FIXED_BYTES + MAX_EXTENSIONS * (1 + MAX_FIELD_BYTES + Long.SIZE_BYTES)
        const val KNOWN_BEHAVIOR_FLAGS = 0x07
        const val KNOWN_OPTION_FLAGS = 0x07
        const val WHEELSLIP = 0x01
        const val DARKRIDE = 0x02
        const val CHARGING = 0x04
        const val FATAL_PRESENT = 0x01
        const val FATAL_VALUE = 0x02
        const val ACTIVE_ALERTS_PRESENT = 0x04
        const val STATUS_OK = 0
        const val STATUS_UNSUPPORTED_ENVELOPE = 2
        const val STATUS_TRUNCATED_PAYLOAD = 3
        const val STATUS_INVALID_LAYOUT = 4
        const val STATUS_INVALID_TELEMETRY = 5
        val linkedAbiVersion = RustRefloatDynamicTelemetryBridge.abiVersion()
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

private fun Byte.unsigned(): Int = toInt() and 0xff

private fun ByteArray.bigEndianUnsignedShort(offset: Int): Int =
    (this[offset].unsigned() shl 8) or this[offset + 1].unsigned()

private fun ByteArray.bigEndianUnsignedInt(offset: Int): Long =
    (this[offset].unsigned().toLong() shl 24) or
        (this[offset + 1].unsigned().toLong() shl 16) or
        (this[offset + 2].unsigned().toLong() shl 8) or
        this[offset + 3].unsigned().toLong()

private fun ByteArray.strictUtf8(start: Int, end: Int): String {
    try {
        return StandardCharsets.UTF_8.newDecoder()
            .onMalformedInput(CodingErrorAction.REPORT)
            .onUnmappableCharacter(CodingErrorAction.REPORT)
            .decode(ByteBuffer.wrap(this, start, end - start))
            .toString()
    } catch (_: CharacterCodingException) {
        throw VescProtocolException.TruncatedPayload
    }
}
