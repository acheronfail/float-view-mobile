package fail.acheron.floatview.protocol.refloat

import fail.acheron.floatview.protocol.RustRefloatControlBridge
import fail.acheron.floatview.protocol.VescProtocolException
import fail.acheron.floatview.protocol.verifyRefloatControlAbiVersion

enum class RefloatBooleanControl(val wireValue: String) {
    PACKAGE("package"), FLYWHEEL("flywheel"), LEDS("leds"), HEADLIGHTS("headlights")
}

data class RefloatLightingState(
    val ledsEnabled: Boolean,
    val headlightsEnabled: Boolean,
)

object RefloatControlCodec {
    private const val OPERATION_PACKAGE = 1
    private const val OPERATION_FLYWHEEL = 2
    private const val OPERATION_LIGHTING = 3
    private const val OPERATION_FATAL_CLEAR = 4
    private const val PROFILE_NONE = 0
    private const val PROFILE_LEGACY = 1
    private const val PROFILE_STABLE = 2
    private const val TARGET_NONE = 0
    private const val TARGET_LEDS = 1
    private const val TARGET_HEADLIGHTS = 2
    private const val STATUS_OK = 0
    private const val STATUS_UNSUPPORTED_COMMAND = 3
    private const val MAXIMUM_PAYLOAD_BYTES = 9
    private val linkedAbiVersion = RustRefloatControlBridge.abiVersion()

    fun packageRequest(enabled: Boolean): ByteArray =
        encode(OPERATION_PACKAGE, PROFILE_NONE, false, TARGET_NONE, enabled, 0)

    fun flywheelRequest(enabled: Boolean): ByteArray =
        encode(OPERATION_FLYWHEEL, PROFILE_NONE, false, TARGET_NONE, enabled, 0)

    fun lightingRequest(
        profile: RefloatControlProfile,
        control: RefloatBooleanControl?,
        enabled: Boolean = false,
        current: RefloatLightingState,
    ): ByteArray {
        require(control == null || control == RefloatBooleanControl.LEDS || control == RefloatBooleanControl.HEADLIGHTS)
        return encode(
            OPERATION_LIGHTING,
            profileValue(profile),
            false,
            when (control) {
                RefloatBooleanControl.LEDS -> TARGET_LEDS
                RefloatBooleanControl.HEADLIGHTS -> TARGET_HEADLIGHTS
                else -> TARGET_NONE
            },
            enabled,
            flags(current),
        )
    }

    fun fatalClearRequest(profile: RefloatControlProfile): ByteArray =
        encode(OPERATION_FATAL_CLEAR, PROFILE_NONE, profile.fatalClearAvailable, TARGET_NONE, false, 0)

    fun decodeLightingResponse(payload: ByteArray, profile: RefloatControlProfile): RefloatLightingState {
        verifyRefloatControlAbiVersion(linkedAbiVersion)
        val result = IntArray(1)
        val status = RustRefloatControlBridge.decodeLighting(payload, profileValue(profile), result)
        return decodeBridgeResult(status, result)
    }

    internal fun decodeBridgeResult(status: Int, result: IntArray): RefloatLightingState {
        if (status != STATUS_OK || result.size != 1 || result[0] !in 0..3) fail()
        return RefloatLightingState(result[0] and 1 != 0, result[0] and 2 != 0)
    }

    @Suppress("LongParameterList")
    private fun encode(
        operation: Int,
        profile: Int,
        fatalClearAvailable: Boolean,
        target: Int,
        enabled: Boolean,
        currentFlags: Int,
    ): ByteArray {
        verifyRefloatControlAbiVersion(linkedAbiVersion)
        val output = ByteArray(MAXIMUM_PAYLOAD_BYTES)
        val result = IntArray(2)
        val status = RustRefloatControlBridge.encode(
            operation,
            profile,
            if (fatalClearAvailable) 1 else 0,
            target,
            if (enabled) 1 else 0,
            currentFlags,
            output,
            result,
        )
        return encodeBridgeResult(status, output, result)
    }

    internal fun encodeBridgeResult(status: Int, output: ByteArray, result: IntArray): ByteArray {
        if (result.size != 2) fail()
        if (status == STATUS_UNSUPPORTED_COMMAND) {
            if (result[0] != 0 || result[1] !in 0..255) fail()
            throw VescProtocolException.UnsupportedCommand(result[1])
        }
        if (status != STATUS_OK || result[0] !in 1..output.size || result[1] != 0) fail()
        return output.copyOf(result[0])
    }

    private fun profileValue(profile: RefloatControlProfile): Int = when (profile.lighting) {
        RefloatLightingProfile.LEGACY_COMMAND_202 -> PROFILE_LEGACY
        RefloatLightingProfile.STABLE_COMMAND_20 -> PROFILE_STABLE
    }

    private fun flags(state: RefloatLightingState): Int =
        (if (state.ledsEnabled) 1 else 0) or (if (state.headlightsEnabled) 2 else 0)

    private fun fail(): Nothing = throw VescProtocolException.InvalidFrame
}
