package fail.acheron.floatview.protocol.refloat

import fail.acheron.floatview.protocol.VescProtocolException

enum class RefloatBooleanControl(val wireValue: String) {
    PACKAGE("package"), FLYWHEEL("flywheel"), LEDS("leds"), HEADLIGHTS("headlights")
}

data class RefloatLightingState(
    val ledsEnabled: Boolean,
    val headlightsEnabled: Boolean,
)

object RefloatControlCodec {
    fun packageRequest(enabled: Boolean): ByteArray =
        RefloatWire.request(RefloatCommand.PACKAGE_LOCK, if (enabled) 0 else 1)

    fun flywheelRequest(enabled: Boolean): ByteArray =
        RefloatWire.request(
            RefloatCommand.FLYWHEEL,
            if (enabled) 0x81 else 0x80,
            0,
            0,
            0,
            0,
            1,
        )

    fun lightingRequest(
        profile: RefloatControlProfile,
        control: RefloatBooleanControl?,
        enabled: Boolean = false,
        current: RefloatLightingState,
    ): ByteArray {
        require(control == null || control == RefloatBooleanControl.LEDS || control == RefloatBooleanControl.HEADLIGHTS)
        val mask = when (control) {
            RefloatBooleanControl.LEDS -> 1
            RefloatBooleanControl.HEADLIGHTS -> 2
            else -> 0
        }
        var flags = (if (current.ledsEnabled) 1 else 0) or (if (current.headlightsEnabled) 2 else 0)
        if (control != null) {
            val bit = if (control == RefloatBooleanControl.LEDS) 1 else 2
            flags = if (enabled) flags or bit else flags and bit.inv()
        }
        return when (profile.lighting) {
            RefloatLightingProfile.LEGACY_COMMAND_202 ->
                RefloatWire.request(RefloatCommand.LEGACY_LIGHTS, mask, flags)
            RefloatLightingProfile.STABLE_COMMAND_20 ->
                RefloatWire.request(RefloatCommand.STABLE_LIGHTS, 0, 0, 0, mask, flags)
        }
    }

    fun fatalClearRequest(profile: RefloatControlProfile): ByteArray {
        if (!profile.fatalClearAvailable) throw VescProtocolException.UnsupportedCommand(36)
        return RefloatWire.request(RefloatCommand.ALERTS_CONTROL, 1)
    }

    fun decodeLightingResponse(payload: ByteArray, profile: RefloatControlProfile): RefloatLightingState {
        val command = when (profile.lighting) {
            RefloatLightingProfile.LEGACY_COMMAND_202 -> RefloatCommand.LEGACY_LIGHTS
            RefloatLightingProfile.STABLE_COMMAND_20 -> RefloatCommand.STABLE_LIGHTS
        }
        if (
            payload.size != 4 ||
            payload[0].toInt() and 0xff != 36 ||
            payload[1].toInt() and 0xff != RefloatWire.PACKAGE_ID ||
            payload[2].toInt() and 0xff != command.id ||
            payload[3].toInt() and 0xfc != 0
        ) {
            throw VescProtocolException.InvalidFrame
        }
        val flags = payload[3].toInt() and 0xff
        return RefloatLightingState(flags and 1 != 0, flags and 2 != 0)
    }
}
