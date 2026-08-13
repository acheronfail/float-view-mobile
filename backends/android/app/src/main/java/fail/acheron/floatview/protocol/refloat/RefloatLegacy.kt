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
import fail.acheron.floatview.protocol.RustRefloatLegacyTelemetryBridge
import fail.acheron.floatview.protocol.VescProtocolException
import fail.acheron.floatview.protocol.verifyRefloatLegacyTelemetryAbiVersion

/** Android adapter for the Rust-owned Refloat command-10 canonical record. */
object RefloatLegacyTelemetryDecoder {
    private const val HEADER_BYTES = 16
    private const val SAMPLE_BYTES = 264
    private const val ACTIVE_FAULT_BYTES = 16
    private const val VALUE_COUNT = 31
    private const val KNOWN_BEHAVIOR_FLAGS = 0b111
    private const val KNOWN_OPTION_FLAGS = 0x0fff
    private const val STATUS_OK = 0
    private const val STATUS_UNSUPPORTED_ENVELOPE = 2
    private const val STATUS_TRUNCATED_PAYLOAD = 3
    private const val STATUS_INVALID_PAYLOAD = 4

    private val linkedAbiVersion = RustRefloatLegacyTelemetryBridge.abiVersion()

    fun decode(payload: ByteArray): RefloatTelemetryRecord {
        verifyRefloatLegacyTelemetryAbiVersion(linkedAbiVersion)
        val record = ByteArray(SAMPLE_BYTES)
        val result = IntArray(2)
        val status = RustRefloatLegacyTelemetryBridge.decode(payload, record, result)
        return decodeBridgeResult(status, record, result)
    }

    internal fun decodeBridgeResult(
        status: Int,
        record: ByteArray,
        result: IntArray,
    ): RefloatTelemetryRecord {
        if (result.size != 2) throw VescProtocolException.TruncatedPayload
        val (recordLength, unsupportedCommand) = result
        return when (status) {
            STATUS_OK -> {
                if (
                    recordLength !in HEADER_BYTES..minOf(record.size, SAMPLE_BYTES) ||
                    unsupportedCommand != 0
                ) throw VescProtocolException.TruncatedPayload
                telemetry(record.copyOf(recordLength))
            }
            STATUS_UNSUPPORTED_ENVELOPE -> {
                if (recordLength != 0 || unsupportedCommand !in 1..255) {
                    throw VescProtocolException.TruncatedPayload
                }
                throw VescProtocolException.UnsupportedCommand(unsupportedCommand)
            }
            STATUS_TRUNCATED_PAYLOAD -> {
                requireEmptyFailure(recordLength, unsupportedCommand)
                throw VescProtocolException.TruncatedPayload
            }
            STATUS_INVALID_PAYLOAD -> {
                requireEmptyFailure(recordLength, unsupportedCommand)
                throw VescProtocolException.InvalidFrame
            }
            else -> throw VescProtocolException.TruncatedPayload
        }
    }

    internal fun telemetry(record: ByteArray): RefloatTelemetryRecord {
        if (
            record.size < HEADER_BYTES ||
            record[0].unsigned() != 1 ||
            record[14].unsigned() != 0 ||
            record[15].unsigned() != 0
        ) throw VescProtocolException.TruncatedPayload
        return when (record[1].unsigned()) {
            1 -> sample(record)
            2 -> activeFault(record)
            else -> throw VescProtocolException.TruncatedPayload
        }
    }

    private fun sample(record: ByteArray): RefloatTelemetryRecord {
        if (
            record.size != SAMPLE_BYTES ||
            record[2].unsigned() !in 1..4 ||
            record[8].unsigned() and KNOWN_BEHAVIOR_FLAGS.inv() != 0 ||
            record[10].unsigned() != 0 ||
            record[11].unsigned() != 0
        ) throw VescProtocolException.TruncatedPayload
        val options = record.bigEndianUnsignedShort(12)
        if (options and KNOWN_OPTION_FLAGS.inv() != 0) throw VescProtocolException.TruncatedPayload

        val packageState = closed(record[3], PackageState.STARTUP, PackageState.RUNNING, PackageState.READY, PackageState.DISABLED)
        val packageMode = closed(record[4], PackageMode.NORMAL, PackageMode.HAND_TEST, PackageMode.FLYWHEEL)
        val footpadState = closed(record[5], FootpadState.NONE, FootpadState.LEFT, FootpadState.RIGHT, FootpadState.BOTH)
        val stopCondition = closed(
            record[6], StopCondition.NONE, StopCondition.PITCH, StopCondition.ROLL,
            StopCondition.SWITCH_HALF, StopCondition.SWITCH_FULL, StopCondition.REVERSE_STOP,
            StopCondition.QUICK_STOP,
        )
        val adjustment = closed(
            record[7], SetpointAdjustment.CENTERING, SetpointAdjustment.REVERSE_STOP,
            SetpointAdjustment.NONE, SetpointAdjustment.DUTY, SetpointAdjustment.HIGH_VOLTAGE,
            SetpointAdjustment.LOW_VOLTAGE, SetpointAdjustment.TEMPERATURE, SetpointAdjustment.UNKNOWN,
        )
        val alert = closed(
            record[9], AlertReason.NONE, AlertReason.LOW_VOLTAGE, AlertReason.HIGH_VOLTAGE,
            AlertReason.CONTROLLER_TEMPERATURE, AlertReason.MOTOR_TEMPERATURE, AlertReason.CURRENT,
            AlertReason.DUTY, AlertReason.SENSORS, AlertReason.LOW_BATTERY, AlertReason.IDLE,
            AlertReason.ERROR, AlertReason.SPEED, AlertReason.CELL_UNDER_TEMPERATURE,
            AlertReason.CELL_OVER_TEMPERATURE, AlertReason.CELL_LOW_VOLTAGE,
            AlertReason.CELL_HIGH_VOLTAGE, AlertReason.UNKNOWN,
        )

        val values = DoubleArray(VALUE_COUNT) { index -> record.bigEndianDouble(HEADER_BYTES + index * 8) }
        val optionFlags = intArrayOf(0x001, 0x002, 0x004, 0x008, 0x010, 0x020, 0x040, 0x080, 0x100, 0x200, 0x400, 0x800)
        val optionIndexes = intArrayOf(2, 3, 13, 22, 23, 24, 25, 26, 27, 28, 29, 30)
        optionFlags.indices.forEach { position ->
            if (options and optionFlags[position] == 0 && values[optionIndexes[position]].toBits() != 0L) {
                throw VescProtocolException.TruncatedPayload
            }
        }
        val detail = record[2].unsigned()
        val required = when (detail) { 1 -> 0; 2 -> 0x00b; 3 -> 0x3fb; else -> 0xffb }
        if (
            options and required != required ||
            (detail < 2 && options and 0xffb != 0) ||
            (detail < 3 && options and 0xff0 != 0) ||
            (detail < 4 && options and 0xc00 != 0)
        ) throw VescProtocolException.TruncatedPayload
        fun optional(index: Int, flag: Int): Double? = if (options and flag == 0) null else values[index]

        return RefloatTelemetryRecord(
            kind = RefloatTelemetryKind.COMPACT,
            rideState = if (packageState == PackageState.RUNNING) RideState.RIDING else RideState.IDLE,
            packageState = packageState, packageMode = packageMode, footpadState = footpadState,
            stopCondition = stopCondition, setpointAdjustment = adjustment,
            wheelslip = record[8].unsigned() and 0x01 != 0,
            darkride = record[8].unsigned() and 0x02 != 0,
            charging = record[8].unsigned() and 0x04 != 0,
            alertReason = alert,
            packVoltageV = values[0], dutyCycle = values[1],
            controllerTempC = optional(2, 0x001), motorTempC = optional(3, 0x002),
            motorCurrentA = values[4], batteryCurrentA = values[5], erpm = values[6],
            speedKph = values[7], pitchDeg = values[8], balancePitchDeg = values[9],
            rollDeg = values[10], requestedBalanceCurrentA = values[11],
            boosterCurrentA = values[12], fieldWeakeningCurrentA = optional(13, 0x004),
            footpadAdc1V = values[14], footpadAdc2V = values[15],
            setpointDeg = values[16], atrSetpointDeg = values[17],
            brakeTiltSetpointDeg = values[18], torqueTiltSetpointDeg = values[19],
            turnTiltSetpointDeg = values[20], remoteSetpointDeg = values[21],
            distanceM = optional(22, 0x008), odometerM = optional(23, 0x010),
            ampHoursDrawnAh = optional(24, 0x020), ampHoursChargedAh = optional(25, 0x040),
            wattHoursDrawnWh = optional(26, 0x080), wattHoursChargedWh = optional(27, 0x100),
            batteryLevel = optional(28, 0x200), chargingCurrentA = optional(29, 0x400),
            chargingVoltageV = optional(30, 0x800),
        )
    }

    private fun activeFault(record: ByteArray): RefloatTelemetryRecord {
        if (
            record.size != ACTIVE_FAULT_BYTES ||
            record.sliceArray(2 until 10).any { it.unsigned() != 0 } ||
            record[11].unsigned() == 0 || record[12].unsigned() != 0 || record[13].unsigned() != 0
        ) throw VescProtocolException.TruncatedPayload
        val fault = closed(
            record[10], FirmwareFault.OVER_VOLTAGE, FirmwareFault.UNDER_VOLTAGE,
            FirmwareFault.DRIVER, FirmwareFault.ABSOLUTE_OVER_CURRENT,
            FirmwareFault.CONTROLLER_OVER_TEMPERATURE, FirmwareFault.MOTOR_OVER_TEMPERATURE,
            FirmwareFault.GATE_DRIVER_OVER_VOLTAGE, FirmwareFault.GATE_DRIVER_UNDER_VOLTAGE,
            FirmwareFault.MCU_UNDER_VOLTAGE, FirmwareFault.WATCHDOG_RESET, FirmwareFault.ENCODER_SPI,
            FirmwareFault.ENCODER_SINCOS_BELOW_MINIMUM, FirmwareFault.ENCODER_SINCOS_ABOVE_MAXIMUM,
            FirmwareFault.FLASH_CORRUPTION, FirmwareFault.CURRENT_SENSOR_OFFSET_1,
            FirmwareFault.CURRENT_SENSOR_OFFSET_2, FirmwareFault.CURRENT_SENSOR_OFFSET_3,
            FirmwareFault.UNBALANCED_CURRENTS, FirmwareFault.BRAKE,
            FirmwareFault.RESOLVER_LOSS_OF_TRACKING, FirmwareFault.RESOLVER_DEGRADATION_OF_SIGNAL,
            FirmwareFault.RESOLVER_LOSS_OF_SIGNAL, FirmwareFault.APP_CONFIG_FLASH_CORRUPTION,
            FirmwareFault.MOTOR_CONFIG_FLASH_CORRUPTION, FirmwareFault.ENCODER_NO_MAGNET,
            FirmwareFault.ENCODER_MAGNET_TOO_STRONG, FirmwareFault.PHASE_FILTER, FirmwareFault.UNKNOWN,
        )
        if ((record[10].unsigned() == 28) != (record[11].unsigned() > 27)) {
            throw VescProtocolException.TruncatedPayload
        }
        return RefloatTelemetryRecord(
            kind = RefloatTelemetryKind.COMPACT,
            rideState = RideState.UNKNOWN,
            alertReason = AlertReason.FIRMWARE_FAULT,
            firmwareFault = fault,
        )
    }

    private fun requireEmptyFailure(recordLength: Int, unsupportedCommand: Int) {
        if (recordLength != 0 || unsupportedCommand != 0) throw VescProtocolException.TruncatedPayload
    }

    private fun <T> closed(code: Byte, vararg values: T): T =
        values.getOrNull(code.unsigned() - 1) ?: throw VescProtocolException.TruncatedPayload

    private fun Byte.unsigned(): Int = toInt() and 0xff

    private fun ByteArray.bigEndianUnsignedShort(offset: Int): Int =
        (this[offset].unsigned() shl 8) or this[offset + 1].unsigned()

    private fun ByteArray.bigEndianDouble(offset: Int): Double {
        var bits = 0L
        for (index in offset until offset + 8) bits = (bits shl 8) or this[index].unsigned().toLong()
        return Double.fromBits(bits).takeIf { it.isFinite() } ?: throw VescProtocolException.TruncatedPayload
    }
}
