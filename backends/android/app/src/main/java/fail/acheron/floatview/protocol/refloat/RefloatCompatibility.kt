package fail.acheron.floatview.protocol.refloat

import fail.acheron.floatview.protocol.RustRefloatCompatibilityBridge
import fail.acheron.floatview.protocol.VescProtocolException
import fail.acheron.floatview.protocol.verifyRefloatCompatibilityAbiVersion

enum class RefloatTelemetryFamily { LEGACY_FIXED, DYNAMIC }

enum class RefloatWireProfile(val family: RefloatTelemetryFamily) {
    LEGACY_FIXED(RefloatTelemetryFamily.LEGACY_FIXED),
    DYNAMIC(RefloatTelemetryFamily.DYNAMIC),
    DYNAMIC_ALERTS(RefloatTelemetryFamily.DYNAMIC),
}

enum class RefloatLightingProfile { LEGACY_COMMAND_202, STABLE_COMMAND_20 }

data class RefloatControlProfile(
    val lighting: RefloatLightingProfile,
    val fatalClearAvailable: Boolean,
)

data class RefloatCompatibilityDecision(
    val wireVersion: String,
    val profile: RefloatWireProfile,
    val family: RefloatTelemetryFamily,
    val controlProfile: RefloatControlProfile,
) {
    init {
        require(family == profile.family)
    }
}

/** Android adapter for Rust-owned Refloat INFO and compatibility policy. */
object RefloatCompatibility {
    fun infoRequest(): ByteArray {
        requireCompatibleAbi()
        val output = ByteArray(INFO_REQUEST_BYTES)
        val result = IntArray(1)
        requireSuccess(RustRefloatCompatibilityBridge.infoRequest(output, result), result[0])
        if (result[0] != output.size) fail()
        return output
    }

    fun decodeInfo(payload: ByteArray): RefloatCompatibilityDecision {
        requireCompatibleAbi()
        val output = ByteArray(COMPATIBILITY_RECORD_BYTES)
        val result = IntArray(1)
        requireSuccess(RustRefloatCompatibilityBridge.decodeInfo(payload, output, result), result[0])
        if (result[0] != output.size) fail()
        return decision(output)
    }

    fun admits(layoutRecord: ByteArray): Boolean {
        requireCompatibleAbi()
        return when (val status = RustRefloatCompatibilityBridge.admitLayout(layoutRecord)) {
            STATUS_OK -> true
            STATUS_INVALID_LAYOUT -> false
            else -> {
                requireSuccess(status, 0)
                false
            }
        }
    }

    internal fun decision(record: ByteArray): RefloatCompatibilityDecision {
        if (
            record.size != COMPATIBILITY_RECORD_BYTES ||
            record[0].unsigned() != RECORD_VERSION ||
            record[1].unsigned() != RECORD_KIND ||
            record[4].unsigned() !in 0..1 ||
            record[9].unsigned() !in 0..1 ||
            record[10].unsigned() != 0 ||
            record[11].unsigned() != 0 ||
            (record[4].unsigned() == 0 && record[5].unsigned() != 0)
        ) fail()

        val wireVersion = if (record[4].unsigned() == 1) {
            "${record[2].unsigned()}.${record[3].unsigned()}.${record[5].unsigned()}"
        } else {
            "${record[2].unsigned()}.${record[3].unsigned()}"
        }
        val profile: RefloatWireProfile
        val family: RefloatTelemetryFamily
        val control: RefloatControlProfile
        when (listOf(record[6].unsigned(), record[7].unsigned(), record[8].unsigned(), record[9].unsigned())) {
            listOf(1, 1, 1, 0) -> {
                profile = RefloatWireProfile.LEGACY_FIXED
                family = RefloatTelemetryFamily.LEGACY_FIXED
                control = RefloatControlProfile(RefloatLightingProfile.LEGACY_COMMAND_202, false)
            }
            listOf(2, 2, 1, 0) -> {
                profile = RefloatWireProfile.DYNAMIC
                family = RefloatTelemetryFamily.DYNAMIC
                control = RefloatControlProfile(RefloatLightingProfile.LEGACY_COMMAND_202, false)
            }
            listOf(3, 2, 2, 1) -> {
                profile = RefloatWireProfile.DYNAMIC_ALERTS
                family = RefloatTelemetryFamily.DYNAMIC
                control = RefloatControlProfile(RefloatLightingProfile.STABLE_COMMAND_20, true)
            }
            else -> fail()
        }
        return RefloatCompatibilityDecision(wireVersion, profile, family, control)
    }

    private fun requireCompatibleAbi() {
        verifyRefloatCompatibilityAbiVersion(linkedAbiVersion)
    }

    private fun requireSuccess(status: Int, outputLength: Int) {
        when (status) {
            STATUS_OK -> return
            STATUS_UNSUPPORTED_ENVELOPE,
            STATUS_UNSUPPORTED_PROTOCOL,
            STATUS_UNSUPPORTED_RELEASE,
            STATUS_INVALID_LAYOUT,
            -> {
                if (outputLength != 0) fail()
                throw VescProtocolException.InvalidFrame
            }
            else -> {
                if (status != STATUS_OUTPUT_TOO_SMALL && outputLength != 0) fail()
                throw VescProtocolException.TruncatedPayload
            }
        }
    }

    private fun Byte.unsigned(): Int = toInt() and 0xff

    private fun fail(): Nothing = throw VescProtocolException.TruncatedPayload

    private const val INFO_REQUEST_BYTES = 5
    private const val COMPATIBILITY_RECORD_BYTES = 12
    private const val RECORD_VERSION = 1
    private const val RECORD_KIND = 1
    private const val STATUS_OK = 0
    private const val STATUS_UNSUPPORTED_ENVELOPE = 2
    private const val STATUS_UNSUPPORTED_PROTOCOL = 4
    private const val STATUS_UNSUPPORTED_RELEASE = 5
    private const val STATUS_INVALID_LAYOUT = 6
    private const val STATUS_OUTPUT_TOO_SMALL = 7
    private val linkedAbiVersion = RustRefloatCompatibilityBridge.abiVersion()
}
